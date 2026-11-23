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
import android.telephony.SubscriptionManager;
import com.android.messaging.mmslib.pdu.*;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import org.json.JSONObject;

/** Carrier networking is delegated to SmsManager, including APN routing, response PDUs and failure codes. */
final class PhoneMmsTransport {
    private PhoneMmsTransport() {}
    static int maximum(SmsManager manager) {
        return Math.max(32000, Math.min((int) PhoneMessageFiles.MAX_BYTES,
                manager.getCarrierConfigValues().getInt(SmsManager.MMS_CONFIG_MAX_MESSAGE_SIZE, 300000)));
    }

    static JSONObject send(Context context, JSONObject request) {
        if (!PhoneMessageProvider.writable(context) || !PhonePermissions.granted(context, Manifest.permission.SEND_SMS))
            return PhoneJson.error("The default SMS role and SMS sending permission are required to send multimedia messages.");
        String client = request.optString("client_id");
        if (client.isEmpty() || client.length() > 1024) return PhoneJson.error("Invalid multimedia send identity.");
        int subscription = PhoneAccounts.resolveSms(context, request.isNull("subscription") ? null : request.optString("subscription"));
        String fingerprint = PhoneMessageJournal.digest(request.toString() + "\n" + subscription);
        JSONObject existing = PhoneSmsSender.reserve(context, client, fingerprint, "mms-operation:" + java.util.UUID.randomUUID());
        if (existing != null) return existing;
        PhoneMessageFiles.Lease transfer = null;
        long provider = 0; String providerKey = null;
        try {
            SmsManager manager = PhoneAccounts.sms(context, subscription);
            SendReq pdu = PhoneMmsCodec.request(context, request);
            byte[] bytes = PhoneMmsCodec.compose(context, pdu);
            if (bytes.length > maximum(manager)) throw new IllegalArgumentException("This MMS exceeds the selected carrier's message-size limit. Choose smaller attachments.");
            transfer = PhoneMessageFiles.create(context, "send", "message.pdu", "application/vnd.wap.mms-message");
            Files.write(transfer.file.toPath(), bytes);
            provider = PhoneMmsStore.outgoing(context, pdu, subscription);
            providerKey = PhoneMessageIdentity.capture(context, "mms", provider);
            PhoneMessageJournal.get(context).getWritableDatabase().delete("mms_delivery", "provider_id=?", new String[]{Long.toString(provider)});
            for (EncodedStringValue recipient : pdu.getTo()) {
                ContentValues recipientRow = new ContentValues(); recipientRow.put("provider_id", provider);
                recipientRow.put("address", PhoneNumbers.messageAddress(recipient.getString()));
                PhoneMessageJournal.get(context).getWritableDatabase().insertOrThrow("mms_delivery", null, recipientRow);
            }
            PhoneSmsSender.record(context, transfer.token, client, provider, "mms", 1);
            ContentValues result = new ContentValues(); result.put("result", "mms:" + provider);
            PhoneMessageJournal.get(context).getWritableDatabase().update("batches", result, "client=?", new String[]{client});
            manager.sendMultimediaMessage(context, transfer.uri(context), null, null, receipt(context, transfer.token, "sent"));
            PhoneDevice.messagesChanged();
            return PhoneSmsSender.batch(context, client);
        } catch (Exception unavailable) {
            if (transfer != null) PhoneMessageFiles.remove(context, transfer.token);
            if (provider > 0) failedSend(context, provider, providerKey);
            PhoneSmsSender.setBatch(context, client, "failed");
            PhoneDevice.messagesChanged();
            return PhoneJson.error("Android could not send the multimedia message: " + PhoneJson.text(unavailable.getMessage(), 256, false));
        }
    }

    static void notification(Context context, NotificationInd notice, int subscription) {
        long id = PhoneMmsStore.notification(context, notice, subscription);
        JSONObject result = download(context, "mms:" + id);
        // download() records failure only against the exact current notification tuple.
        PhoneDevice.messagesChanged();
        PhoneMessageNotifications.show(context, "Multimedia message", PhoneMmsCodec.text(notice.getSubject()), thread(context, id));
    }

    static synchronized JSONObject download(Context context, String messageId) {
        if (!PhoneMessageProvider.writable(context)) return PhoneJson.error("The default SMS role and SMS read access are required to download MMS.");
        long id = PhoneMessageProvider.identity(messageId, "mms:");
        Uri message = ContentUris.withAppendedId(Telephony.Mms.CONTENT_URI, id);
        String location, transaction; int subscription;
        try (Cursor cursor = context.getContentResolver().query(message,
                new String[]{Telephony.Mms.MESSAGE_TYPE, Telephony.Mms.CONTENT_LOCATION, Telephony.Mms.TRANSACTION_ID, Telephony.Mms.SUBSCRIPTION_ID, Telephony.Mms.MESSAGE_SIZE}, null, null, null)) {
            if (cursor == null || !cursor.moveToFirst() || cursor.getInt(0) != PduHeaders.MESSAGE_TYPE_NOTIFICATION_IND)
                return PhoneJson.error("That MMS is no longer awaiting download.");
            location = cursor.getString(1); transaction = cursor.getString(2); subscription = cursor.getInt(3);
            if (cursor.getLong(4) > PhoneMessageFiles.MAX_BYTES) return PhoneJson.error("The MMS exceeds the private download limit.");
        }
        Uri remote = location == null ? Uri.EMPTY : Uri.parse(location);
        if (!("http".equals(remote.getScheme()) || "https".equals(remote.getScheme())) || remote.getHost() == null
                || remote.getUserInfo() != null || location.length() > 4096 || transaction == null || transaction.length() > 512
                || !SubscriptionManager.isValidSubscriptionId(subscription))
            return PhoneJson.error("The carrier's MMS address or originating SIM is unavailable.");
        SQLiteDatabase db = PhoneMessageJournal.get(context).getWritableDatabase();
        try (Cursor pending = db.query("downloads", new String[]{"token"}, "provider_id=?", new String[]{Long.toString(id)}, null, null, null)) {
            while (pending.moveToNext()) {
                try { PhoneMessageFiles.resolve(context, pending.getString(0), "download"); return PhoneJson.deviceAccepted("That multimedia download is already pending."); }
                catch (java.io.IOException expired) { db.delete("downloads", "token=?", new String[]{pending.getString(0)}); }
            }
        }
        PhoneMessageFiles.Lease transfer = null;
        try {
            transfer = PhoneMessageFiles.create(context, "download", "message.pdu", "application/vnd.wap.mms-message");
            ContentValues work = new ContentValues(); work.put("token", transfer.token); work.put("provider_id", id);
            work.put("subscription", subscription); work.put("location", location); work.put("transaction_id", transaction);
            db.insertOrThrow("downloads", null, work);
            ContentValues pending = new ContentValues(); pending.put(Telephony.Mms.RETRIEVE_STATUS, 0);
            if (updateNotification(context, id, subscription, location, transaction, pending) != 1) throw new IllegalStateException("That MMS notification was removed before download.");
            PhoneAccounts.sms(context, subscription).downloadMultimediaMessage(context, location, transfer.uri(context), null, receipt(context, transfer.token, "download"));
            return PhoneJson.deviceAccepted("Android accepted the carrier download request. The message is updated only after the actual result.");
        } catch (Exception unavailable) {
            if (transfer != null) { db.delete("downloads", "token=?", new String[]{transfer.token}); PhoneMessageFiles.remove(context, transfer.token); }
            failedDownload(context, id, subscription, location, transaction);
            return PhoneJson.error("Android could not start the MMS download: " + PhoneJson.text(unavailable.getMessage(), 160, false));
        }
    }

    static PendingIntent receipt(Context context, String token, String phase) {
        Intent intent = new Intent(context, PhoneMmsResultReceiver.class).setData(Uri.parse("phone-mms-result:" + token + "/" + phase))
                .putExtra("token", token).putExtra("phase", phase);
        int flags = PendingIntent.FLAG_UPDATE_CURRENT | PendingIntent.FLAG_ONE_SHOT;
        // The platform fills in the SendConf PDU. Component, token and phase are already fixed.
        if (Build.VERSION.SDK_INT >= 31) flags |= PendingIntent.FLAG_MUTABLE;
        return PendingIntent.getBroadcast(context, 0, intent, flags);
    }

    static void result(Context context, JSONObject response) {
        String token = response.optString("token"), phase = response.optString("phase");
        if ("ack".equals(phase)) { PhoneMessageFiles.remove(context, token); return; }
        if ("download".equals(phase)) { downloaded(context, token, response.optInt("result")); return; }
        if (!"sent".equals(phase)) return;
        SQLiteDatabase db = PhoneMessageJournal.get(context).getWritableDatabase();
        try (Cursor row = db.query("sends", new String[]{"client", "provider_id", "transport", "provider_key", "sent", "failed"}, "token=?", new String[]{token}, null, null, null)) {
            if (!row.moveToFirst() || !"mms".equals(row.getString(2)) || !row.getString(4).isEmpty() || !row.getString(5).isEmpty()) return;
            String client = row.getString(0); long id = row.getLong(1);
            GenericPdu parsed = PhoneMmsCodec.parse(android.util.Base64.decode(response.optString("pdu", ""), android.util.Base64.DEFAULT));
            SendConf confirmation = parsed instanceof SendConf ? (SendConf) parsed : null;
            boolean success = response.optInt("result") == Activity.RESULT_OK
                    && (confirmation == null || confirmation.getResponseStatus() == PduHeaders.RESPONSE_STATUS_OK);
            ContentValues values = new ContentValues();
            values.put(Telephony.Mms.MESSAGE_BOX, success ? Telephony.Mms.MESSAGE_BOX_SENT : Telephony.Mms.MESSAGE_BOX_OUTBOX);
            values.put(Telephony.Mms.RESPONSE_STATUS, success ? PduHeaders.RESPONSE_STATUS_OK
                    : confirmation == null ? PduHeaders.RESPONSE_STATUS_ERROR_UNSPECIFIED : confirmation.getResponseStatus());
            if (confirmation != null) values.put(Telephony.Mms.MESSAGE_ID, PhoneMmsCodec.iso(confirmation.getMessageId()));
            PhoneMessageIdentity.update(context, "mms", id, row.getString(3), values);
            ContentValues finished = new ContentValues(); finished.put(success ? "sent" : "failed", "AQ==");
            db.update("sends", finished, "token=?", new String[]{token});
            PhoneSmsSender.setBatch(context, client, success ? "sent" : "failed");
            JSONObject event = PhoneSmsSender.batch(context, client).optJSONObject("response");
            if (event != null) PhoneDevice.event(event);
        } finally { PhoneMessageFiles.remove(context, token); }
        PhoneDevice.messagesChanged();
    }

    private static void downloaded(Context context, String token, int result) {
        SQLiteDatabase db = PhoneMessageJournal.get(context).getWritableDatabase();
        try (Cursor row = db.query("downloads", new String[]{"provider_id", "subscription", "location", "transaction_id"},
                "token=?", new String[]{token}, null, null, null)) {
            if (!row.moveToFirst()) return;
            long id = row.getLong(0); int subscription = row.getInt(1);
            String location = row.getString(2), transaction = row.getString(3);
            boolean stored = false;
            boolean selected = notificationCurrent(context, id, subscription, location, transaction);
            if (!selected) {
                db.delete("downloads", "token=?", new String[]{token});
                return; // A deleted/replaced row is never recreated or mutated by an old transfer.
            }
            try {
                PhoneMessageFiles.Lease file = PhoneMessageFiles.resolve(context, token, "download");
                if (result == Activity.RESULT_OK && file.file.length() <= PhoneMessageFiles.MAX_BYTES) {
                    GenericPdu parsed = PhoneMmsCodec.parse(Files.readAllBytes(file.file.toPath()));
                    if (parsed instanceof RetrieveConf) {
                        RetrieveConf message = (RetrieveConf) parsed;
                        if (message.getRetrieveStatus() == 0 || message.getRetrieveStatus() == PduHeaders.RETRIEVE_STATUS_OK) {
                            PhoneMmsStore.incoming(context, id, message, subscription, location, transaction); stored = true;
                        }
                    }
                }
            } catch (Exception invalidOrUnavailable) { }
            if (!stored) failedDownload(context, id, subscription, location, transaction);
            notifyResponse(context, subscription, transaction, location, stored ? PduHeaders.STATUS_RETRIEVED : PduHeaders.STATUS_DEFERRED);
            if (stored) PhoneMessageNotifications.show(context, "Multimedia message", "The multimedia message was downloaded.", thread(context, id));
            db.delete("downloads", "token=?", new String[]{token});
        } finally { PhoneMessageFiles.remove(context, token); }
        PhoneDevice.messagesChanged();
    }

    private static void notifyResponse(Context context, int subscription, String transaction, String location, int status) {
        PhoneMessageFiles.Lease file = null;
        try {
            SmsManager manager = PhoneAccounts.sms(context, subscription);
            NotifyRespInd response = new NotifyRespInd(PduHeaders.CURRENT_MMS_VERSION, transaction.getBytes(StandardCharsets.ISO_8859_1), status);
            file = PhoneMessageFiles.create(context, "send", "response.pdu", "application/vnd.wap.mms-message");
            Files.write(file.file.toPath(), PhoneMmsCodec.compose(context, response));
            String destination = manager.getCarrierConfigValues().getBoolean(SmsManager.MMS_CONFIG_NOTIFY_WAP_MMSC_ENABLED) ? location : null;
            manager.sendMultimediaMessage(context, file.uri(context), destination, null, receipt(context, file.token, "ack"));
        } catch (Exception unavailable) { if (file != null) PhoneMessageFiles.remove(context, file.token); }
    }

    static void delivery(Context context, DeliveryInd receipt, int subscription) {
        if (receipt.getMessageId() == null || receipt.getTo() == null || receipt.getTo().length != 1 || !SubscriptionManager.isValidSubscriptionId(subscription)) return;
        String address = PhoneNumbers.messageAddress(receipt.getTo()[0].getString());
        if (address == null) return;
        String messageId = PhoneMmsCodec.iso(receipt.getMessageId());
        SQLiteDatabase db = PhoneMessageJournal.get(context).getWritableDatabase();
        try (Cursor rows = context.getContentResolver().query(Telephony.Mms.CONTENT_URI,
                new String[]{Telephony.Mms._ID}, "m_id=? AND msg_box=? AND sub_id=?",
                new String[]{messageId, Integer.toString(Telephony.Mms.MESSAGE_BOX_SENT), Integer.toString(subscription)}, null)) {
            if (rows == null) return;
            while (rows.moveToNext()) {
                long id = rows.getLong(0);
                try (Cursor send = db.query("sends", new String[]{"client", "provider_key"}, "provider_id=? AND transport='mms'", new String[]{Long.toString(id)}, null, null, null)) {
                    if (!send.moveToFirst() || !PhoneMessageIdentity.matches(context, "mms", id, send.getString(1))) continue;
                    int status = receipt.getStatus();
                    boolean terminal = status == PduHeaders.STATUS_RETRIEVED || status == PduHeaders.STATUS_EXPIRED
                            || status == PduHeaders.STATUS_REJECTED || status == PduHeaders.STATUS_UNREACHABLE;
                    if (!terminal) continue;
                    ContentValues received = new ContentValues(); received.put("status", status);
                    if (db.update("mms_delivery", received, "provider_id=? AND address=? AND status<>?",
                            new String[]{Long.toString(id), address, Integer.toString(PduHeaders.STATUS_RETRIEVED)}) == 0) continue;
                    boolean all = true, any = false, failed = false;
                    try (Cursor participants = db.query("mms_delivery", new String[]{"status"}, "provider_id=?", new String[]{Long.toString(id)}, null, null, null)) {
                        while (participants.moveToNext()) {
                            any = true; int actual = participants.getInt(0);
                            all &= actual == PduHeaders.STATUS_RETRIEVED;
                            failed |= actual == PduHeaders.STATUS_EXPIRED || actual == PduHeaders.STATUS_REJECTED || actual == PduHeaders.STATUS_UNREACHABLE;
                        }
                    }
                    if (!any || !all && !failed) continue;
                    ContentValues values = new ContentValues(); values.put(Telephony.Mms.STATUS, all ? PduHeaders.STATUS_RETRIEVED : PduHeaders.STATUS_REJECTED);
                    PhoneMessageIdentity.update(context, "mms", id, send.getString(1), values);
                    PhoneSmsSender.setBatch(context, send.getString(0), all ? "delivered" : "failed");
                    JSONObject event = PhoneSmsSender.batch(context, send.getString(0)).optJSONObject("response");
                    if (event != null) PhoneDevice.event(event);
                }
            }
        }
        PhoneDevice.messagesChanged();
    }

    static boolean notificationCurrent(Context context, long id, int subscription, String location, String transaction) {
        try (Cursor row = context.getContentResolver().query(ContentUris.withAppendedId(Telephony.Mms.CONTENT_URI, id),
                new String[]{"m_type"}, "sub_id=? AND ct_l=? AND tr_id=?",
                new String[]{Integer.toString(subscription), location, transaction}, null)) {
            return row != null && row.moveToFirst() && row.getInt(0) == PduHeaders.MESSAGE_TYPE_NOTIFICATION_IND;
        }
    }

    private static int updateNotification(Context context, long id, int subscription, String location, String transaction, ContentValues values) {
        return context.getContentResolver().update(ContentUris.withAppendedId(Telephony.Mms.CONTENT_URI, id), values,
                "m_type=? AND sub_id=? AND ct_l=? AND tr_id=?",
                new String[]{Integer.toString(PduHeaders.MESSAGE_TYPE_NOTIFICATION_IND), Integer.toString(subscription), location, transaction});
    }
    private static void failedDownload(Context context, long id, int subscription, String location, String transaction) {
        ContentValues value = new ContentValues(); value.put(Telephony.Mms.RETRIEVE_STATUS, PduHeaders.RETRIEVE_STATUS_ERROR_TRANSIENT_FAILURE);
        updateNotification(context, id, subscription, location, transaction, value);
    }
    private static void failedSend(Context context, long id, String key) {
        ContentValues value = new ContentValues(); value.put(Telephony.Mms.RESPONSE_STATUS, PduHeaders.RESPONSE_STATUS_ERROR_UNSPECIFIED);
        if (key != null) PhoneMessageIdentity.update(context, "mms", id, key, value);
    }
    private static String thread(Context context, long id) {
        try (Cursor row = context.getContentResolver().query(ContentUris.withAppendedId(Telephony.Mms.CONTENT_URI, id), new String[]{Telephony.Mms.THREAD_ID}, null, null, null)) {
            return row != null && row.moveToFirst() && row.getLong(0) > 0 ? "thread:" + row.getLong(0) : null;
        }
    }
}
