package org.ferroui.phone;

import android.content.ContentUris;
import android.content.Context;
import android.database.Cursor;
import android.net.Uri;
import android.os.CancellationSignal;
import android.os.ParcelFileDescriptor;
import android.provider.Telephony;
import com.android.messaging.mmslib.pdu.PduHeaders;
import java.util.ArrayList;
import org.json.JSONArray;
import org.json.JSONObject;

final class PhoneMmsReader {
    static final Uri PARTS = Telephony.Mms.CONTENT_URI.buildUpon().appendPath("part").build();
    private PhoneMmsReader() {}

    static ArrayList<JSONObject> messages(Context context, String threadId, long thread, int limit, CancellationSignal signal) {
        ArrayList<JSONObject> result = new ArrayList<>();
        String[] columns = {Telephony.Mms._ID, Telephony.Mms.DATE, Telephony.Mms.READ, Telephony.Mms.MESSAGE_BOX,
                Telephony.Mms.SUBJECT, Telephony.Mms.SUBJECT_CHARSET, Telephony.Mms.SUBSCRIPTION_ID,
                Telephony.Mms.MESSAGE_TYPE, Telephony.Mms.RETRIEVE_STATUS, Telephony.Mms.STATUS, Telephony.Mms.RESPONSE_STATUS};
        try (Cursor cursor = context.getContentResolver().query(Telephony.Mms.CONTENT_URI, columns, "thread_id=?",
                new String[]{Long.toString(thread)}, Telephony.Mms.DATE + " DESC", signal)) {
            if (cursor != null) while (result.size() < limit && cursor.moveToNext()) {
                signal.throwIfCanceled();
                long id = cursor.getLong(0);
                int box = cursor.getInt(3), type = cursor.getInt(7);
                boolean incoming = box == Telephony.Mms.MESSAGE_BOX_INBOX;
                boolean pending = type == PduHeaders.MESSAGE_TYPE_NOTIFICATION_IND;
                JSONObject content = parts(context, id, signal, false);
                String state = incoming ? pending ? cursor.getInt(8) >= 192 ? "failed" : "queued" : "received"
                        : box == Telephony.Mms.MESSAGE_BOX_DRAFTS ? "draft"
                        : (cursor.getInt(10) >= 129 || cursor.getInt(9) == PduHeaders.STATUS_REJECTED || cursor.getInt(9) == PduHeaders.STATUS_EXPIRED || cursor.getInt(9) == PduHeaders.STATUS_UNREACHABLE) ? "failed" : box == Telephony.Mms.MESSAGE_BOX_SENT
                        ? cursor.getInt(9) == PduHeaders.STATUS_RETRIEVED ? "delivered" : "sent" : "sending";
                JSONObject row = PhoneJson.put(new JSONObject(), "id", "mms:" + id);
                PhoneJson.put(row, "thread_id", threadId);
                PhoneJson.put(row, "address", address(context, id, incoming, signal));
                PhoneJson.put(row, "subject", PhoneJson.text(PhoneMmsCodec.decoded(cursor.getString(4), cursor.getInt(5)), 512, false));
                PhoneJson.put(row, "body", pending ? "Multimedia message awaiting download." : content.optString("body"));
                PhoneJson.put(row, "timestamp", cursor.getLong(1) * 1000);
                PhoneJson.put(row, "read", cursor.getInt(2) != 0); PhoneJson.put(row, "outgoing", !incoming);
                PhoneJson.put(row, "status", state);
                int subscription = cursor.getInt(6);
                PhoneJson.put(row, "subscription", subscription >= 0 ? "sms-subscription:" + subscription : null);
                PhoneJson.put(row, "attachment_count", content.optJSONArray("attachments").length());
                PhoneJson.put(row, "can_download", pending && PhoneRoles.held(context, "sms"));
                result.add(row);
            }
        }
        return result;
    }

    private static String address(Context context, long id, boolean incoming, CancellationSignal signal) {
        Uri uri = ContentUris.withAppendedId(Telephony.Mms.CONTENT_URI, id).buildUpon().appendPath("addr").build();
        try (Cursor cursor = context.getContentResolver().query(uri, new String[]{"address", "charset"}, "type=?",
                new String[]{Integer.toString(incoming ? PduHeaders.FROM : PduHeaders.TO)}, null, signal)) {
            if (cursor != null && cursor.moveToFirst()) return PhoneJson.text(PhoneMmsCodec.decoded(cursor.getString(0), cursor.getInt(1)), 256, false);
        }
        return "";
    }

    static JSONObject parts(Context context, long message, CancellationSignal signal, boolean sizes) {
        JSONArray attachments = new JSONArray(); StringBuilder body = new StringBuilder();
        String[] columns = {Telephony.Mms.Part._ID, Telephony.Mms.Part.CONTENT_TYPE, Telephony.Mms.Part.NAME,
                Telephony.Mms.Part.FILENAME, Telephony.Mms.Part.CONTENT_LOCATION, Telephony.Mms.Part.TEXT};
        try (Cursor cursor = context.getContentResolver().query(PARTS, columns, "mid=?", new String[]{Long.toString(message)}, "seq ASC", signal)) {
            if (cursor != null) while (attachments.length() < 100 && cursor.moveToNext()) {
                signal.throwIfCanceled();
                long id = cursor.getLong(0); String mime = cursor.getString(1);
                if ("application/smil".equals(mime)) continue;
                if ("text/plain".equals(mime) || "text/html".equals(mime)) {
                    String text = cursor.getString(5);
                    if (text != null && body.length() < 65536) { if (body.length() != 0) body.append('\n'); body.append(text); }
                    continue;
                }
                String name = cursor.getString(3);
                if (name == null || name.isEmpty()) name = cursor.getString(2);
                if (name == null || name.isEmpty()) name = cursor.getString(4);
                name = PhoneJson.text(name, 512, false).trim();
                if (name.isEmpty()) name = "attachment";
                JSONObject row = PhoneJson.put(new JSONObject(), "id", "mms-part:" + id);
                PhoneJson.put(row, "name", name); PhoneJson.put(row, "mime", PhoneMessageFiles.mime(mime));
                if (sizes) {
                    try (ParcelFileDescriptor descriptor = context.getContentResolver().openFileDescriptor(ContentUris.withAppendedId(PARTS, id), "r", signal)) {
                        if (descriptor != null && descriptor.getStatSize() >= 0 && descriptor.getStatSize() <= 32L * 1024 * 1024) PhoneJson.put(row, "bytes", descriptor.getStatSize());
                    } catch (java.io.IOException unavailable) { }
                }
                attachments.put(row);
            }
        }
        JSONObject result = PhoneJson.put(new JSONObject(), "body", PhoneJson.text(body, 65536, true));
        return PhoneJson.put(result, "attachments", attachments);
    }

    static JSONObject attachments(Context context, String messageId, CancellationSignal signal) {
        if (!PhoneMessageProvider.readable(context)) return PhoneJson.error("SMS read access is required to inspect MMS attachments.");
        long message = PhoneMessageProvider.identity(messageId, "mms:");
        JSONObject value = PhoneJson.put(new JSONObject(), "message_id", messageId);
        return PhoneJson.device("messageAttachments", PhoneJson.put(value, "attachments", parts(context, message, signal, true).optJSONArray("attachments")));
    }

    static JSONObject attachment(Context context, String messageId, String attachmentId, CancellationSignal signal) {
        long part = PhoneMessageProvider.identity(attachmentId, "mms-part:");
        JSONArray rows = parts(context, PhoneMessageProvider.identity(messageId, "mms:"), signal, true).optJSONArray("attachments");
        for (int index = 0; index < rows.length(); index++) {
            JSONObject row = rows.optJSONObject(index);
            if (row != null && attachmentId.equals(row.optString("id"))) return row;
        }
        throw new IllegalArgumentException("That attachment does not belong to the selected MMS message.");
    }
}
