package org.ferroui.phone;

import android.Manifest;
import android.app.Activity;
import android.app.PendingIntent;
import android.content.ContentUris;
import android.content.ContentValues;
import android.content.Context;
import android.content.Intent;
import android.database.Cursor;
import android.database.sqlite.SQLiteDatabase;
import android.net.Uri;
import android.os.Build;
import android.provider.Telephony;
import android.telephony.SmsManager;
import android.telephony.SmsMessage;
import java.util.ArrayList;
import java.util.HashSet;
import java.util.UUID;
import org.json.JSONArray;
import org.json.JSONObject;

final class PhoneSmsSender {
    private PhoneSmsSender() {}
    private static final class Transmission {
        String address, token;
        ArrayList<PendingIntent> sent = new ArrayList<>(), delivered = new ArrayList<>();
    }

    static JSONObject send(Context context, JSONObject request) {
        if (!PhoneMessageProvider.writable(context) || !PhonePermissions.granted(context, Manifest.permission.SEND_SMS))
            return PhoneJson.error("Select the default SMS role and grant SMS sending permission before sending.");
        JSONArray recipients = request.optJSONArray("recipients");
        String body = request.optString("body"), client = request.optString("client_id");
        if (!validRecipients(recipients) || body.trim().isEmpty() || body.getBytes(java.nio.charset.StandardCharsets.UTF_8).length > 16000 || client.isEmpty() || client.length() > 1024)
            return PhoneJson.error("Invalid SMS recipients, body or send identity.");
        int subscription = PhoneAccounts.resolveSms(context, request.isNull("subscription") ? null : request.optString("subscription"));
        SmsManager manager = PhoneAccounts.sms(context, subscription);
        ArrayList<String> parts = manager.divideMessage(body);
        if (parts.isEmpty() || parts.size() > 256) return PhoneJson.error("This text exceeds Android's multipart SMS limit.");
        String fingerprint = PhoneMessageJournal.digest(recipients.toString() + "\n" + body + "\n" + subscription);
        JSONObject existing = reserve(context, client, fingerprint, "sms-batch:" + UUID.randomUUID());
        if (existing != null) return existing;
        ArrayList<Transmission> transmissions = new ArrayList<>();
        try {
            for (int recipient = 0; recipient < recipients.length(); recipient++) {
                if (Thread.currentThread().isInterrupted()) throw new IllegalStateException("SMS sending was interrupted.");
                String address = PhoneNumbers.messageAddress(recipients.optString(recipient));
                ContentValues values = new ContentValues();
                values.put(Telephony.Sms.ADDRESS, address); values.put(Telephony.Sms.BODY, body);
                values.put(Telephony.Sms.DATE, System.currentTimeMillis()); values.put(Telephony.Sms.READ, 1); values.put(Telephony.Sms.SEEN, 1);
                values.put(Telephony.Sms.SUBSCRIPTION_ID, subscription); values.put(Telephony.Sms.STATUS, Telephony.Sms.STATUS_PENDING);
                values.put(Telephony.Sms.THREAD_ID, Telephony.Threads.getOrCreateThreadId(context, address));
                Uri uri = context.getContentResolver().insert(Telephony.Sms.Outbox.CONTENT_URI, values);
                if (uri == null) throw new IllegalStateException("Android could not create an SMS outbox record.");
                long provider = ContentUris.parseId(uri);
                String token = UUID.randomUUID().toString();
                record(context, token, client, provider, "sms", parts.size());
                Transmission transmission = new Transmission();
                transmission.address = address; transmission.token = token;
                for (int part = 0; part < parts.size(); part++) {
                    transmission.sent.add(receipt(context, token, part, "sent"));
                    transmission.delivered.add(receipt(context, token, part, "delivered"));
                }
                transmissions.add(transmission);
            }
        } catch (RuntimeException interrupted) {
            for (Transmission transmission : transmissions) fail(context, transmission.token);
            setBatch(context, client, "failed");
            return batch(context, client);
        }
        // Journal every recipient before any carrier callback can complete the aggregate.
        for (Transmission transmission : transmissions) {
            try { manager.sendMultipartTextMessage(transmission.address, null, parts, transmission.sent, transmission.delivered); }
            catch (RuntimeException rejected) { fail(context, transmission.token); }
        }
        aggregate(context, client);
        PhoneDevice.messagesChanged();
        return batch(context, client);
    }

    static boolean validRecipients(JSONArray recipients) {
        if (recipients == null || recipients.length() < 1 || recipients.length() > 20) return false;
        HashSet<String> seen = new HashSet<>();
        for (int i = 0; i < recipients.length(); i++) {
            String number = PhoneNumbers.messageAddress(recipients.optString(i));
            if (number == null || !seen.add(number)) return false;
        }
        return true;
    }

    /** Returns an existing durable result; client-id reuse with changed contents is rejected. */
    static synchronized JSONObject reserve(Context context, String client, String digest, String result) {
        PhoneMessageJournal.get(context).prune(context);
        SQLiteDatabase db = PhoneMessageJournal.get(context).getWritableDatabase();
        try (Cursor cursor = db.query("batches", new String[]{"digest"}, "client=?", new String[]{client}, null, null, null)) {
            if (cursor.moveToFirst()) return digest.equals(cursor.getString(0)) ? batch(context, client)
                    : PhoneJson.error("That send identity was already used for different message contents.");
        }
        // Existing identities remain queryable when the bounded journal is full.
        try (Cursor count = db.rawQuery("SELECT COUNT(*) FROM batches", null)) {
            if (count.moveToFirst() && count.getLong(0) >= 4096) throw new IllegalStateException("The 30-day message receipt journal is full.");
        }
        ContentValues row = new ContentValues(); row.put("client", client); row.put("digest", digest); row.put("result", result);
        row.put("status", "queued"); row.put("created", System.currentTimeMillis());
        if (db.insertOrThrow("batches", null, row) < 0) throw new IllegalStateException("Cannot journal SMS operation.");
        return null;
    }

    static void record(Context context, String token, String client, long provider, String transport, int parts) {
        ContentValues row = new ContentValues(); row.put("token", token); row.put("client", client);
        row.put("provider_id", provider); row.put("transport", transport); row.put("parts", parts);
        row.put("provider_key", PhoneMessageIdentity.capture(context, transport, provider));
        SQLiteDatabase db = PhoneMessageJournal.get(context).getWritableDatabase();
        db.beginTransaction();
        try {
            db.delete("sends", "provider_id=? AND transport=?", new String[]{Long.toString(provider), transport});
            db.insertOrThrow("sends", null, row);
            db.setTransactionSuccessful();
        } finally { db.endTransaction(); }
        setBatch(context, client, "sending");
    }

    static PendingIntent receipt(Context context, String token, int part, String phase) {
        Intent intent = new Intent(context, PhoneSmsResultReceiver.class)
                .setData(Uri.parse("phone-sms-result:" + token + "/" + phase + "/" + part))
                .putExtra("token", token).putExtra("part", part).putExtra("phase", phase);
        int flags = PendingIntent.FLAG_UPDATE_CURRENT;
        if (!"delivered".equals(phase)) flags |= PendingIntent.FLAG_ONE_SHOT;
        // Delivery reports carry a modem PDU, including temporary reports before the final report.
        // This mutable intent is explicit/private and is cancelled after a terminal receipt.
        flags |= "delivered".equals(phase) && Build.VERSION.SDK_INT >= 31 ? PendingIntent.FLAG_MUTABLE
                : "delivered".equals(phase) ? 0 : PendingIntent.FLAG_IMMUTABLE;
        return PendingIntent.getBroadcast(context, 0, intent, flags);
    }

    static void receivedResult(Context context, JSONObject data) {
        String token = data.optString("token"), phase = data.optString("phase");
        SQLiteDatabase db = PhoneMessageJournal.get(context).getWritableDatabase();
        try (Cursor cursor = db.query("sends", new String[]{"client", "provider_id", "parts", "sent", "delivered", "failed", "transport", "provider_key"},
                "token=?", new String[]{token}, null, null, null)) {
            if (!cursor.moveToFirst() || !"sms".equals(cursor.getString(6))) return;
            String client = cursor.getString(0);
            long provider = cursor.getLong(1);
            PhonePartProgress progress = new PhonePartProgress(cursor.getInt(2), cursor.getString(3), cursor.getString(4), cursor.getString(5));
            int part = data.optInt("part", -1);
            if ("sent".equals(phase)) progress.sent(part, data.optInt("result") == Activity.RESULT_OK);
            else if ("delivered".equals(phase)) {
                byte[] pdu = android.util.Base64.decode(data.optString("pdu"), android.util.Base64.DEFAULT);
                SmsMessage report = pdu.length == 0 ? null : SmsMessage.createFromPdu(pdu, data.optString("format", "3gpp"));
                if (report != null) {
                    int status = PhoneDeliveryStatus.normalize(data.optString("format", "3gpp"), report.getStatus(), report.isStatusReportMessage());
                    progress.delivered(part, status);
                }
                else if (data.optInt("result") != Activity.RESULT_OK) progress.delivered(part, 64);
            } else return;
            ContentValues saved = new ContentValues(); saved.put("sent", PhonePartProgress.encode(progress.sent));
            saved.put("delivered", PhonePartProgress.encode(progress.delivered)); saved.put("failed", PhonePartProgress.encode(progress.failed));
            db.update("sends", saved, "token=?", new String[]{token});
            if ("delivered".equals(phase) && (progress.delivered.get(part) || progress.failed.get(part)))
                receipt(context, token, part, "delivered").cancel();
            updateProvider(context, provider, cursor.getString(7), progress.status());
            aggregate(context, client);
        }
        PhoneDevice.messagesChanged();
    }

    static void fail(Context context, String token) {
        JSONObject data = PhoneJson.put(new JSONObject(), "token", token);
        PhoneJson.put(data, "phase", "sent"); PhoneJson.put(data, "part", 0); PhoneJson.put(data, "result", Activity.RESULT_CANCELED);
        receivedResult(context, data);
    }

    private static void updateProvider(Context context, long provider, String key, String state) {
        if (!PhoneRoles.held(context, "sms")) return;
        ContentValues values = new ContentValues();
        values.put(Telephony.Sms.TYPE, "failed".equals(state) ? Telephony.Sms.MESSAGE_TYPE_FAILED
                : "sent".equals(state) || "delivered".equals(state) ? Telephony.Sms.MESSAGE_TYPE_SENT : Telephony.Sms.MESSAGE_TYPE_OUTBOX);
        values.put(Telephony.Sms.STATUS, "failed".equals(state) ? Telephony.Sms.STATUS_FAILED
                : "delivered".equals(state) ? Telephony.Sms.STATUS_COMPLETE : Telephony.Sms.STATUS_PENDING);
        PhoneMessageIdentity.update(context, "sms", provider, key, values);
    }

    static void aggregate(Context context, String client) {
        String state = "delivered"; boolean any = false;
        try (Cursor cursor = PhoneMessageJournal.get(context).getReadableDatabase().query("sends", new String[]{"parts", "sent", "delivered", "failed"},
                "client=?", new String[]{client}, null, null, null)) {
            while (cursor.moveToNext()) {
                any = true;
                String part = new PhonePartProgress(cursor.getInt(0), cursor.getString(1), cursor.getString(2), cursor.getString(3)).status();
                if ("failed".equals(part)) { state = "failed"; break; }
                if ("sending".equals(part)) state = "sending";
                else if ("sent".equals(part) && "delivered".equals(state)) state = "sent";
            }
        }
        if (!any) state = "queued";
        setBatch(context, client, state);
        JSONObject response = batch(context, client).optJSONObject("response");
        if (response != null) PhoneDevice.event(response);
    }

    static void setBatch(Context context, String client, String status) {
        ContentValues values = new ContentValues(); values.put("status", status);
        PhoneMessageJournal.get(context).getWritableDatabase().update("batches", values, "client=?", new String[]{client});
    }

    static JSONObject batch(Context context, String client) {
        try (Cursor cursor = PhoneMessageJournal.get(context).getReadableDatabase().query("batches", new String[]{"result", "status"},
                "client=?", new String[]{client}, null, null, null)) {
            if (!cursor.moveToFirst()) return PhoneJson.error("The message operation is unavailable.");
            JSONObject result = PhoneJson.put(new JSONObject(), "client_id", client);
            PhoneJson.put(result, "id", cursor.getString(0)); PhoneJson.put(result, "status", cursor.getString(1));
            return PhoneJson.device("messageSent", result);
        }
    }
}
