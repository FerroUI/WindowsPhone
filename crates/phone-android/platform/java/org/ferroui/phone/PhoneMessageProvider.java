package org.ferroui.phone;

import android.Manifest;
import android.content.ContentUris;
import android.content.ContentValues;
import android.content.Context;
import android.database.Cursor;
import android.net.Uri;
import android.os.CancellationSignal;
import android.provider.Telephony;
import java.util.ArrayList;
import java.util.Comparator;
import java.util.HashSet;
import org.json.JSONArray;
import org.json.JSONObject;

/** Actual SMS/MMS provider data, kept outside the portable document and its backups. */
final class PhoneMessageProvider {
    static final Object MUTATIONS = new Object();
    private PhoneMessageProvider() {}
    static boolean readable(Context context) {
        if (!PhonePermissions.granted(context, Manifest.permission.READ_SMS)) return false;
        android.app.AppOpsManager manager = context.getSystemService(android.app.AppOpsManager.class);
        if (manager == null) return false;
        try {
            int mode = manager.checkOpNoThrow(android.app.AppOpsManager.OPSTR_READ_SMS,
                    android.os.Process.myUid(), context.getPackageName());
            // This is an observed usability filter. The provider still enforces every actual read.
            return mode == android.app.AppOpsManager.MODE_ALLOWED || mode == android.app.AppOpsManager.MODE_DEFAULT;
        } catch (RuntimeException unavailable) { return false; }
    }
    static boolean writable(Context context) { return PhoneRoles.held(context, "sms") && readable(context); }

    static long identity(String value, String prefix) {
        if (value == null || !value.startsWith(prefix)) throw new IllegalArgumentException("Invalid message identity.");
        String number = value.substring(prefix.length());
        if (number.length() > 19 || !number.matches("[1-9][0-9]*")) throw new IllegalArgumentException("Invalid message identity.");
        return Long.parseLong(number);
    }

    static JSONObject threads(Context context, int limit, CancellationSignal signal) {
        if (!readable(context)) return PhoneJson.error("Grant SMS read permission before opening device messages.");
        if (limit < 1 || limit > 500) return PhoneJson.error("Invalid conversation limit.");
        // The public Threads columns use TelephonyProvider's simple conversation view.
        Uri uri = Telephony.Threads.CONTENT_URI.buildUpon().appendQueryParameter("simple", "true").build();
        String[] columns = {Telephony.Threads._ID, Telephony.Threads.DATE, Telephony.Threads.RECIPIENT_IDS,
                Telephony.Threads.SNIPPET, Telephony.Threads.HAS_ATTACHMENT};
        JSONArray rows = new JSONArray();
        int bytes = 0;
        try (Cursor cursor = context.getContentResolver().query(uri, columns, null, null, Telephony.Threads.DATE + " DESC", signal)) {
            if (cursor != null) while (rows.length() < limit && cursor.moveToNext()) {
                signal.throwIfCanceled();
                long id = cursor.getLong(0);
                if (id <= 0) continue;
                JSONArray recipients = recipients(context, cursor.getString(2), signal);
                StringBuilder title = new StringBuilder();
                for (int i = 0; i < recipients.length(); i++) { if (i != 0) title.append(", "); title.append(recipients.optString(i)); }
                JSONObject row = PhoneJson.put(new JSONObject(), "id", "thread:" + id);
                PhoneJson.put(row, "recipients", recipients);
                PhoneJson.put(row, "title", PhoneJson.text(title, 512, false));
                PhoneJson.put(row, "snippet", PhoneJson.text(cursor.getString(3), 4096, true));
                PhoneJson.put(row, "timestamp", cursor.getLong(1));
                PhoneJson.put(row, "unread", unread(context, id, signal));
                PhoneJson.put(row, "has_attachments", cursor.getInt(4) != 0);
                int size = PhoneJson.bytes(row) + 1;
                if (bytes + size > 850000) break;
                bytes += size;
                rows.put(row);
            }
        }
        return PhoneJson.device("messageThreads", rows);
    }

    private static JSONArray recipients(Context context, String values, CancellationSignal signal) {
        JSONArray result = new JSONArray();
        if (values == null || values.length() > 2048) return result;
        ArrayList<String> ids = new ArrayList<>();
        for (String value : values.split(" ")) {
            if (ids.size() >= 32) break;
            if (value.matches("[1-9][0-9]{0,18}")) ids.add(value);
        }
        if (ids.isEmpty()) return result;
        String selection = "_id IN (" + String.join(",", java.util.Collections.nCopies(ids.size(), "?")) + ")";
        Uri uri = Telephony.MmsSms.CONTENT_URI.buildUpon().appendPath("canonical-addresses").build();
        try (Cursor cursor = context.getContentResolver().query(uri, new String[]{"address"}, selection,
                ids.toArray(new String[0]), null, signal)) {
            if (cursor != null) {
                // AOSP returns its fixed _id,address projection for this endpoint,
                // regardless of the caller's requested projection. Never decode it by position.
                int addressColumn = cursor.getColumnIndexOrThrow("address");
                while (result.length() < 32 && cursor.moveToNext()) {
                    signal.throwIfCanceled();
                    String address = PhoneJson.text(cursor.getString(addressColumn), 256, false).trim();
                    if (!address.isEmpty()) result.put(address);
                }
            }
        }
        return result;
    }

    private static long unread(Context context, long thread, CancellationSignal signal) {
        long count = 0;
        for (Uri uri : new Uri[]{Telephony.Sms.CONTENT_URI, Telephony.Mms.CONTENT_URI}) {
            try (Cursor cursor = context.getContentResolver().query(uri, new String[]{"COUNT(*)"},
                    "thread_id=? AND read=0", new String[]{Long.toString(thread)}, null, signal)) {
                if (cursor != null && cursor.moveToFirst()) count += Math.max(0, cursor.getLong(0));
            }
        }
        return Math.min(0xffffffffL, count);
    }

    static JSONObject messages(Context context, String threadId, int limit, CancellationSignal signal) {
        if (!readable(context)) return PhoneJson.error("Grant SMS read permission before opening device messages.");
        long thread = identity(threadId, "thread:");
        if (limit < 1 || limit > 500) return PhoneJson.error("Invalid message limit.");
        ArrayList<JSONObject> messages = new ArrayList<>();
        String[] columns = {Telephony.Sms._ID, Telephony.Sms.ADDRESS, Telephony.Sms.BODY, Telephony.Sms.DATE,
                Telephony.Sms.READ, Telephony.Sms.TYPE, Telephony.Sms.STATUS, Telephony.Sms.SUBSCRIPTION_ID};
        try (Cursor cursor = context.getContentResolver().query(Telephony.Sms.CONTENT_URI, columns, "thread_id=?",
                new String[]{Long.toString(thread)}, Telephony.Sms.DATE + " DESC", signal)) {
            if (cursor != null) while (messages.size() < limit && cursor.moveToNext()) {
                signal.throwIfCanceled();
                JSONObject row = PhoneJson.put(new JSONObject(), "id", "sms:" + cursor.getLong(0));
                PhoneJson.put(row, "thread_id", threadId);
                PhoneJson.put(row, "address", PhoneJson.text(cursor.getString(1), 256, false));
                PhoneJson.put(row, "body", PhoneJson.text(cursor.getString(2), 65536, true));
                PhoneJson.put(row, "timestamp", cursor.getLong(3));
                PhoneJson.put(row, "read", cursor.getInt(4) != 0);
                PhoneJson.put(row, "outgoing", cursor.getInt(5) != Telephony.Sms.MESSAGE_TYPE_INBOX);
                PhoneJson.put(row, "status", smsStatus(cursor.getInt(5), cursor.getInt(6)));
                int subscription = cursor.getInt(7);
                PhoneJson.put(row, "subscription", subscription >= 0 ? "sms-subscription:" + subscription : null);
                PhoneJson.put(row, "attachment_count", 0);
                messages.add(row);
            }
        }
        messages.addAll(PhoneMmsReader.messages(context, threadId, thread, limit, signal));
        messages.sort(Comparator.comparingLong((JSONObject row) -> row.optLong("timestamp")).reversed());
        JSONArray selected = new JSONArray();
        int bytes = 0;
        for (JSONObject row : messages) {
            if (selected.length() >= limit) break;
            int size = PhoneJson.bytes(row) + 1;
            if (bytes + size > 850000) break;
            bytes += size;
            selected.put(row);
        }
        JSONObject payload = PhoneJson.put(new JSONObject(), "thread_id", threadId);
        return PhoneJson.device("messages", PhoneJson.put(payload, "messages", selected));
    }

    static String smsStatus(int type, int status) {
        switch (type) {
            case Telephony.Sms.MESSAGE_TYPE_INBOX: return "received";
            case Telephony.Sms.MESSAGE_TYPE_DRAFT: return "draft";
            case Telephony.Sms.MESSAGE_TYPE_OUTBOX: return "sending";
            case Telephony.Sms.MESSAGE_TYPE_QUEUED: return "queued";
            case Telephony.Sms.MESSAGE_TYPE_FAILED: return "failed";
            case Telephony.Sms.MESSAGE_TYPE_SENT:
                if (status >= Telephony.Sms.STATUS_FAILED) return "failed";
                return status == Telephony.Sms.STATUS_COMPLETE ? "delivered" : "sent";
            default: return "unknown";
        }
    }

    static JSONObject markRead(Context context, String threadId) {
        if (!writable(context)) return PhoneJson.error("The default SMS role and SMS read access are required to change messages.");
        long thread = identity(threadId, "thread:");
        ContentValues changes = new ContentValues(); changes.put("read", 1); changes.put("seen", 1);
        int count = context.getContentResolver().update(Telephony.Sms.CONTENT_URI, changes, "thread_id=? AND read=0", new String[]{Long.toString(thread)});
        count += context.getContentResolver().update(Telephony.Mms.CONTENT_URI, changes, "thread_id=? AND read=0", new String[]{Long.toString(thread)});
        PhoneDevice.messagesChanged();
        return PhoneJson.deviceAccepted("Android marked " + count + " message records as read.");
    }

    static JSONObject delete(Context context, String threadId, JSONArray ids) {
        if (!writable(context)) return PhoneJson.error("The default SMS role and SMS read access are required to delete messages.");
        long thread = identity(threadId, "thread:");
        if (ids == null || ids.length() == 0 || ids.length() > 500) return PhoneJson.error("Invalid message selection.");
        HashSet<String> unique = new HashSet<>();
        ArrayList<Uri> uris = new ArrayList<>();
        for (int i = 0; i < ids.length(); i++) {
            String id = ids.optString(i);
            if (!unique.add(id)) return PhoneJson.error("Duplicate message selection.");
            boolean sms = id.startsWith("sms:");
            long number = identity(id, sms ? "sms:" : "mms:");
            uris.add(ContentUris.withAppendedId(sms ? Telephony.Sms.CONTENT_URI : Telephony.Mms.CONTENT_URI, number));
        }
        int count = 0;
        synchronized (MUTATIONS) {
            for (Uri uri : uris) {
                int removed = context.getContentResolver().delete(uri, "thread_id=?", new String[]{Long.toString(thread)});
                count += removed;
                if (removed > 0) PhoneMessageJournal.get(context).deleted(context, "sms".equals(uri.getAuthority()) ? "sms" : "mms", ContentUris.parseId(uri));
            }
        }
        PhoneDevice.messagesChanged();
        return PhoneJson.deviceAccepted("Android deleted " + count + " selected message records from that conversation.");
    }
}
