package org.ferroui.phone;

import android.Manifest;
import android.content.Context;
import android.database.Cursor;
import android.net.Uri;
import android.os.CancellationSignal;
import android.provider.CallLog;
import org.json.JSONArray;
import org.json.JSONObject;

final class PhoneCallHistory {
    private PhoneCallHistory() {}

    static JSONObject read(Context context, int limit, CancellationSignal signal) {
        if (!PhonePermissions.granted(context, Manifest.permission.READ_CALL_LOG))
            return PhoneJson.error("Grant Android call-history permission before reading device calls.");
        if (limit < 1 || limit > 500) return PhoneJson.error("Invalid call-history limit.");
        Uri uri = CallLog.Calls.CONTENT_URI.buildUpon().appendQueryParameter(CallLog.Calls.LIMIT_PARAM_KEY, Integer.toString(limit)).build();
        String[] columns = {CallLog.Calls._ID, CallLog.Calls.NUMBER, CallLog.Calls.CACHED_NAME, CallLog.Calls.DATE,
                CallLog.Calls.DURATION, CallLog.Calls.TYPE, CallLog.Calls.PHONE_ACCOUNT_COMPONENT_NAME, CallLog.Calls.PHONE_ACCOUNT_ID};
        JSONArray rows = new JSONArray();
        try (Cursor cursor = context.getContentResolver().query(uri, columns, null, null, CallLog.Calls.DATE + " DESC", signal)) {
            if (cursor != null) while (rows.length() < limit && cursor.moveToNext()) {
                signal.throwIfCanceled();
                JSONObject row = PhoneJson.put(new JSONObject(), "id", "call-log:" + cursor.getLong(0));
                PhoneJson.put(row, "number", PhoneJson.text(cursor.getString(1), 256, false));
                PhoneJson.put(row, "name", PhoneJson.text(cursor.getString(2), 512, false));
                PhoneJson.put(row, "timestamp", cursor.getLong(3));
                PhoneJson.put(row, "duration", Math.max(0, Math.min(0xffffffffL, cursor.getLong(4))));
                PhoneJson.put(row, "direction", direction(cursor.getInt(5)));
                PhoneJson.put(row, "account", PhoneAccounts.id(cursor.getString(6), cursor.getString(7)));
                rows.put(row);
            }
        }
        return PhoneJson.device("callHistory", rows);
    }

    private static String direction(int type) {
        switch (type) {
            case CallLog.Calls.INCOMING_TYPE: return "incoming";
            case CallLog.Calls.OUTGOING_TYPE: return "outgoing";
            case CallLog.Calls.MISSED_TYPE: return "missed";
            case CallLog.Calls.REJECTED_TYPE: return "rejected";
            case CallLog.Calls.BLOCKED_TYPE: return "blocked";
            case CallLog.Calls.VOICEMAIL_TYPE: return "voicemail";
            default: return "unknown";
        }
    }
}
