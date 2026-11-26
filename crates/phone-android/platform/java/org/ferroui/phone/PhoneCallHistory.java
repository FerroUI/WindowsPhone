package org.ferroui.phone;

import android.Manifest;
import android.content.Context;
import android.content.ContentValues;
import android.database.Cursor;
import android.net.Uri;
import android.os.CancellationSignal;
import android.os.SystemClock;
import android.provider.CallLog;
import org.json.JSONArray;
import org.json.JSONObject;

final class PhoneCallHistory {
    private static final PhoneCallReceipts receipts = new PhoneCallReceipts();
    private PhoneCallHistory() {}

    static boolean readable(Context context) {
        return PhonePermissions.granted(context, Manifest.permission.READ_CALL_LOG)
                && usable(context, android.app.AppOpsManager.OPSTR_READ_CALL_LOG);
    }
    static boolean writable(Context context) {
        return readable(context) && PhoneRoles.held(context, "dialer")
                && PhonePermissions.granted(context, Manifest.permission.WRITE_CALL_LOG)
                && usable(context, android.app.AppOpsManager.OPSTR_WRITE_CALL_LOG);
    }
    private static boolean usable(Context context, String operation) {
        android.app.AppOpsManager manager = context.getSystemService(android.app.AppOpsManager.class);
        if (manager == null) return false;
        try {
            int mode = manager.checkOpNoThrow(operation, android.os.Process.myUid(), context.getPackageName());
            return mode == android.app.AppOpsManager.MODE_ALLOWED || mode == android.app.AppOpsManager.MODE_DEFAULT;
        } catch (RuntimeException unavailable) { return false; }
    }
    static void invalidate() { receipts.clear(); }

    static JSONObject missed(Context context, CancellationSignal signal) {
        if (!readable(context)) return PhoneJson.error("Grant Android call-history permission before reading missed calls.");
        // Allocate before the query so delayed earlier reads cannot resurrect an
        // acknowledged count. The lifetime counter is never reset by close/regrant.
        long revision = receipts.revision(), lease = receipts.lease();
        java.util.ArrayList<PhoneCallReceipts.Row> rows = new java.util.ArrayList<>();
        int count;
        String selection = CallLog.Calls.TYPE + "=? AND " + CallLog.Calls.NEW + "=?";
        try (Cursor cursor = context.getContentResolver().query(CallLog.Calls.CONTENT_URI,
                new String[]{CallLog.Calls._ID, CallLog.Calls.DATE}, selection,
                new String[]{Integer.toString(CallLog.Calls.MISSED_TYPE), "1"},
                CallLog.Calls.DATE + " DESC, " + CallLog.Calls._ID + " DESC", signal)) {
            if (cursor == null) return PhoneJson.error("Android did not return a missed-call observation.");
            count = cursor.getCount();
            while (rows.size() < PhoneCallReceipts.MAX_ROWS && cursor.moveToNext()) {
                signal.throwIfCanceled();
                rows.add(new PhoneCallReceipts.Row(cursor.getLong(0), cursor.getLong(1)));
            }
        }
        signal.throwIfCanceled();
        JSONObject value = PhoneJson.put(PhoneJson.put(new JSONObject(), "revision", revision), "count", count);
        String token = writable(context) ? receipts.issue(lease, SystemClock.elapsedRealtime(), rows) : null;
        JSONObject acknowledgement = token == null ? null : PhoneJson.put(PhoneJson.put(new JSONObject(), "token", token), "count", rows.size());
        PhoneJson.put(value, "acknowledgement", acknowledgement);
        return PhoneJson.device("missedCalls", value);
    }

    static JSONObject acknowledge(Context context, String token, CancellationSignal signal) {
        signal.throwIfCanceled();
        if (!writable(context)) return PhoneJson.error("Select Windows Phone as the default phone app and grant call-log write access before acknowledging calls.");
        long lease = receipts.lease();
        java.util.List<PhoneCallReceipts.Row> rows = receipts.consume(token, SystemClock.elapsedRealtime());
        if (rows.isEmpty()) return PhoneJson.error("This missed-call selection expired. Refresh the history and try again.");
        ContentValues values = new ContentValues();
        values.put(CallLog.Calls.NEW, 0);
        // IS_READ describes consumption, which merely viewing history does not
        // establish. Set only NEW, and only for the captured ID/date/type tuple.
        String selection = CallLog.Calls._ID + "=? AND " + CallLog.Calls.DATE + "=? AND "
                + CallLog.Calls.TYPE + "=? AND " + CallLog.Calls.NEW + "=?";
        try {
            for (PhoneCallReceipts.Row row : rows) {
                signal.throwIfCanceled();
                if (!receipts.current(lease) || !writable(context) || PhoneActivity.foreground() == null)
                    return PhoneJson.error("Call acknowledgement stopped because access or foreground ownership changed. Refresh the history.");
                context.getContentResolver().update(CallLog.Calls.CONTENT_URI, values, selection,
                        new String[]{Long.toString(row.id), Long.toString(row.timestamp), Integer.toString(CallLog.Calls.MISSED_TYPE), "1"});
            }
            return missed(context, signal);
        } finally {
            // Partial writes are idempotent and the token is already consumed.
            // The live observer refreshes even when the provider rejects a row.
            PhoneDevice.callsChanged();
        }
    }

    static JSONObject read(Context context, int limit, CancellationSignal signal) {
        if (!readable(context))
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
