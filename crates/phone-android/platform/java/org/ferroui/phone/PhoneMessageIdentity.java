package org.ferroui.phone;

import android.content.ContentUris;
import android.content.ContentValues;
import android.content.Context;
import android.database.Cursor;
import android.net.Uri;
import android.provider.Telephony;
import java.util.ArrayList;
import org.json.JSONArray;
import org.json.JSONObject;

/** Receipt writes bind immutable provider fields as well as row ids, which providers may reuse after deletion. */
final class PhoneMessageIdentity {
    private PhoneMessageIdentity() {}
    private static String[] columns(String transport) {
        return "sms".equals(transport) ? new String[]{"date", "sub_id", "thread_id", "address", "body"}
                : new String[]{"date", "sub_id", "thread_id", "tr_id"};
    }
    static Uri uri(String transport, long id) { return ContentUris.withAppendedId("sms".equals(transport) ? Telephony.Sms.CONTENT_URI : Telephony.Mms.CONTENT_URI, id); }
    static String capture(Context context, String transport, long id) {
        String[] columns = columns(transport); JSONArray values = new JSONArray();
        try (Cursor row = context.getContentResolver().query(uri(transport, id), columns, null, null, null)) {
            if (row == null || !row.moveToFirst()) throw new IllegalStateException("The message row was removed before its carrier request.");
            for (int index = 0; index < columns.length; index++) values.put(row.isNull(index) ? JSONObject.NULL : row.getString(index));
        }
        return values.toString();
    }
    static boolean matches(Context context, String transport, long id, String key) {
        try { return key.equals(capture(context, transport, id)); }
        catch (RuntimeException removed) { return false; }
    }
    static int update(Context context, String transport, long id, String key, ContentValues update) {
        try {
            JSONArray values = new JSONArray(key); String[] columns = columns(transport);
            if (values.length() != columns.length) return 0;
            StringBuilder selection = new StringBuilder(); ArrayList<String> arguments = new ArrayList<>();
            for (int index = 0; index < columns.length; index++) {
                if (index > 0) selection.append(" AND "); selection.append(columns[index]);
                if (values.isNull(index)) selection.append(" IS NULL");
                else { selection.append("=?"); arguments.add(values.getString(index)); }
            }
            return context.getContentResolver().update(uri(transport, id), update, selection.toString(), arguments.toArray(new String[0]));
        } catch (org.json.JSONException invalid) { return 0; }
    }
}
