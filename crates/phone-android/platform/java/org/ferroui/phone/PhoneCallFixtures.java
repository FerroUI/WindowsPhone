package org.ferroui.phone;

import android.content.ContentUris;
import android.content.ContentValues;
import android.content.Context;
import android.database.Cursor;
import android.net.Uri;
import android.os.Build;
import android.os.CancellationSignal;
import android.provider.CallLog;
import java.io.File;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.StandardCopyOption;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.HashSet;
import java.util.Set;
import org.json.JSONArray;
import org.json.JSONObject;

/** Explicit, one-attempt fixture for the CI-owned emulator. Never starts a call. */
final class PhoneCallFixtures {
    private static String attempted;
    private PhoneCallFixtures() {}
    private static void require(boolean condition, String message) {
        if (!condition) throw new IllegalStateException(message);
    }
    private static boolean emulator() {
        return ("ranchu".equals(Build.HARDWARE) || "goldfish".equals(Build.HARDWARE))
                && Arrays.asList(Build.SUPPORTED_ABIS).contains("x86_64")
                && (Build.PRODUCT.startsWith("sdk_") || Build.PRODUCT.startsWith("google_sdk"));
    }
    private static void writable(Context context) {
        require(BuildConfig.DEBUG && emulator(), "The CallLog fixture requires a debug x86_64 Android emulator.");
        require(PhoneActivity.foreground() != null, "The CallLog fixture requires the current foreground phone.");
        require(PhoneRoles.held(context, "dialer") && PhoneCallHistory.writable(context),
                "The CallLog fixture requires the actual dialer role and current read/write access.");
    }
    private static void marker(File marker, String nonce) throws Exception {
        require(marker.isFile() && marker.length() <= 65 && nonce.matches("[0-9a-f]{32}")
                && nonce.equals(new String(Files.readAllBytes(marker.toPath()), StandardCharsets.UTF_8).trim()),
                "The CallLog fixture requires the explicit current-session nonce marker.");
    }
    private static JSONObject response(JSONObject result, String kind) {
        require("device".equals(result.optString("kind")), "Native provider rejected the fixture: " + result);
        JSONObject response = result.optJSONObject("response");
        require(response != null && kind.equals(response.optString("kind")), "Unexpected provider response: " + result);
        return response;
    }
    private static int count(JSONObject result) throws Exception {
        return response(result, "missedCalls").getJSONObject("value").getInt("count");
    }
    private static JSONArray rows(Context context) throws Exception {
        JSONArray result = new JSONArray();
        try (Cursor cursor = context.getContentResolver().query(CallLog.Calls.CONTENT_URI,
                new String[]{CallLog.Calls._ID, CallLog.Calls.DATE, CallLog.Calls.TYPE, CallLog.Calls.NEW, CallLog.Calls.IS_READ},
                null, null, CallLog.Calls._ID + " ASC")) {
            require(cursor != null, "CallLog did not return row observations.");
            require(cursor.getCount() <= 512, "The isolated fixture refuses a nonempty large call catalog.");
            while (cursor.moveToNext()) {
                JSONObject row = PhoneJson.put(new JSONObject(), "id", cursor.getLong(0));
                PhoneJson.put(row, "timestamp", cursor.getLong(1)); PhoneJson.put(row, "type", cursor.getInt(2));
                PhoneJson.put(row, "new", cursor.isNull(3) ? JSONObject.NULL : cursor.getInt(3));
                PhoneJson.put(row, "is_read", cursor.isNull(4) ? JSONObject.NULL : cursor.getInt(4));
                result.put(row);
            }
        }
        return result;
    }
    private static Set<Long> unacknowledged(JSONArray rows) throws Exception {
        Set<Long> ids = new HashSet<>();
        for (int i = 0; i < rows.length(); i++) {
            JSONObject row = rows.getJSONObject(i);
            if (row.getInt("type") == CallLog.Calls.MISSED_TYPE && row.optInt("new", 0) == 1) ids.add(row.getLong("id"));
        }
        return ids;
    }
    private static JSONObject insert(Context context, File marker, String nonce, int index, long timestamp, ArrayList<JSONObject> owned) throws Exception {
        marker(marker, nonce);
        writable(context);
        String number = "555014" + index;
        ContentValues values = new ContentValues();
        values.put(CallLog.Calls.NUMBER, number); values.put(CallLog.Calls.CACHED_NAME, "FerroUI CallLog " + nonce);
        values.put(CallLog.Calls.DATE, timestamp); values.put(CallLog.Calls.DURATION, 0);
        values.put(CallLog.Calls.TYPE, CallLog.Calls.MISSED_TYPE); values.put(CallLog.Calls.NEW, 1);
        values.put(CallLog.Calls.IS_READ, index == 2 ? 1 : 0);
        Uri uri = context.getContentResolver().insert(CallLog.Calls.CONTENT_URI, values);
        require(uri != null, "CallLog did not return the inserted fixture ID.");
        long id = ContentUris.parseId(uri);
        require(id > 0, "CallLog returned an invalid fixture ID.");
        JSONObject row = PhoneJson.put(new JSONObject(), "id", id);
        PhoneJson.put(row, "timestamp", timestamp); PhoneJson.put(row, "number", number);
        PhoneJson.put(row, "is_read", index == 2 ? 1 : 0);
        owned.add(row); // Record authority immediately, before any later assertion.
        return row;
    }
    private static void changedOnlyCaptured(JSONArray before, JSONArray after, Set<Long> captured) throws Exception {
        require(before.length() == after.length(), "Acknowledgment changed the number of call rows.");
        for (int i = 0; i < before.length(); i++) {
            JSONObject old = before.getJSONObject(i), now = after.getJSONObject(i);
            for (String key : new String[]{"id", "timestamp", "type", "is_read"})
                require(old.get(key).equals(now.get(key)), "Acknowledgment changed " + key + " or unrelated row identity.");
            Object expected = captured.contains(old.getLong("id")) ? Integer.valueOf(0) : old.get("new");
            require(expected.equals(now.get("new")), "Acknowledgment changed an uncaptured NEW flag or missed a captured row.");
        }
    }
    private static void publish(File directory, JSONObject proof, ArrayList<JSONObject> owned) throws Exception {
        PhoneJson.put(proof, "inserted_rows", new JSONArray(owned));
        File temporary = new File(directory, "phone-call-fixture.tmp");
        Files.write(temporary.toPath(), proof.toString().getBytes(StandardCharsets.UTF_8));
        Files.move(temporary.toPath(), new File(directory, "phone-call-fixture.json").toPath(),
                StandardCopyOption.REPLACE_EXISTING, StandardCopyOption.ATOMIC_MOVE);
    }
    static synchronized void runIfRequested(Context context, File directory, String nonce) {
        if (!BuildConfig.DEBUG || nonce.equals(attempted)) return;
        File marker = new File(directory, "phone-call-fixture.enabled");
        if (!marker.isFile()) return;
        attempted = nonce; // One attempt, including refusal/failure; never retry silently.
        File output = new File(directory, "phone-call-fixture.json");
        try {
            if (output.isFile() && output.length() <= 512000 && nonce.equals(new JSONObject(
                    new String(Files.readAllBytes(output.toPath()), StandardCharsets.UTF_8)).optString("nonce"))) return;
        } catch (Exception unreadable) { /* A nonce claim below still prevents replay. */ }
        JSONObject proof = PhoneJson.put(new JSONObject(), "schema", 1);
        PhoneJson.put(proof, "nonce", nonce); PhoneJson.put(proof, "kind", "real_calllog_provider_fixture");
        PhoneJson.put(proof, "result", "running"); PhoneJson.put(proof, "rf_requests", 0);
        PhoneJson.put(proof, "started_at", System.currentTimeMillis());
        ArrayList<JSONObject> owned = new ArrayList<>();
        JSONArray cleanupErrors = new JSONArray(), deleted = new JSONArray(), remaining = new JSONArray();
        JSONArray initial = null;
        try {
            marker(marker, nonce);
            writable(context);
            require(new File(directory, "phone-call-fixture-" + nonce + ".claim").createNewFile(),
                    "This fixture nonce was already attempted; refusing to replay after a process restart.");
            JSONObject guards = PhoneJson.put(new JSONObject(), "debug", true);
            PhoneJson.put(guards, "emulator", true); PhoneJson.put(guards, "foreground", true);
            PhoneJson.put(guards, "dialer", true); PhoneJson.put(guards, "writable", true); PhoneJson.put(guards, "nonce_marker", true);
            PhoneJson.put(guards, "hardware", Build.HARDWARE); PhoneJson.put(guards, "product", Build.PRODUCT);
            PhoneJson.put(guards, "abis", new JSONArray(Arrays.asList(Build.SUPPORTED_ABIS)));
            PhoneJson.put(proof, "guards", guards);
            CancellationSignal signal = new CancellationSignal();
            JSONObject baseline = PhoneCallHistory.missed(context, signal);
            PhoneJson.put(proof, "initial_receipt", baseline);
            require(count(baseline) == 0, "Refusing to acknowledge a catalog with preexisting unacknowledged missed calls.");
            initial = rows(context); PhoneJson.put(proof, "rows_initial", initial);
            require(unacknowledged(initial).isEmpty(), "An unrelated missed call appeared before fixture insertion.");
            PhoneJson.put(proof, "phase", "inserting_captured_rows"); publish(directory, proof, owned);
            long date = System.currentTimeMillis() - 6000;
            JSONObject first = insert(context, marker, nonce, 1, date, owned);
            publish(directory, proof, owned);
            JSONObject second = insert(context, marker, nonce, 2, date + 2000, owned);
            publish(directory, proof, owned);
            JSONObject captured = PhoneCallHistory.missed(context, signal);
            PhoneJson.put(proof, "captured_receipt", captured);
            require(count(captured) == 2, "The real missed-call count must include both fixture rows.");
            JSONObject acknowledgement = response(captured, "missedCalls").getJSONObject("value").getJSONObject("acknowledgement");
            require(acknowledgement.getInt("count") == 2, "The captured provider receipt must cover both rows.");
            String token = acknowledgement.getString("token");
            JSONObject history = PhoneCallHistory.read(context, 1, signal);
            PhoneJson.put(proof, "history_limit", 1); PhoneJson.put(proof, "history_receipt", history);
            JSONArray page = response(history, "callHistory").getJSONArray("value");
            require(page.length() == 1 && ("call-log:" + second.getLong("id")).equals(page.getJSONObject(0).getString("id")),
                    "The real one-row history page must contain the newer fixture row.");
            JSONObject later = insert(context, marker, nonce, 3, date + 4000, owned);
            PhoneJson.put(proof, "phase", "inserted_later_row"); publish(directory, proof, owned);
            JSONObject beforeReceipt = PhoneCallHistory.missed(context, signal);
            PhoneJson.put(proof, "before_ack_receipt", beforeReceipt);
            require(count(beforeReceipt) == 3, "The later missed call must be counted before acknowledgment.");
            JSONArray before = rows(context); PhoneJson.put(proof, "rows_before_ack", before);
            Set<Long> all = new HashSet<>(Arrays.asList(first.getLong("id"), second.getLong("id"), later.getLong("id")));
            require(unacknowledged(before).equals(all), "Refusing to acknowledge after an unrelated missed call appeared.");
            marker(marker, nonce);
            writable(context);
            PhoneJson.put(proof, "phase", "acknowledging_captured_receipt"); publish(directory, proof, owned);
            JSONObject afterReceipt = PhoneCallHistory.acknowledge(context, token, signal);
            PhoneJson.put(proof, "after_ack_receipt", afterReceipt);
            require(count(afterReceipt) == 1, "Acknowledging the captured selection must leave the later missed call.");
            Set<Long> capturedIds = new HashSet<>(Arrays.asList(first.getLong("id"), second.getLong("id")));
            JSONArray after = rows(context); PhoneJson.put(proof, "rows_after_ack", after);
            changedOnlyCaptured(before, after, capturedIds);
            marker(marker, nonce);
            writable(context);
            JSONObject repeat = PhoneCallHistory.acknowledge(context, token, signal);
            PhoneJson.put(proof, "repeat_receipt", repeat);
            require("error".equals(repeat.optString("kind")), "A consumed provider receipt was accepted twice.");
            JSONArray repeated = rows(context); PhoneJson.put(proof, "rows_after_repeat", repeated);
            require(after.toString().equals(repeated.toString()), "Reusing the receipt changed call rows.");
            JSONObject latest = PhoneCallHistory.missed(context, signal);
            PhoneJson.put(proof, "after_repeat_receipt", latest);
            require(count(latest) == 1 && unacknowledged(repeated).equals(new HashSet<>(Arrays.asList(later.getLong("id")))),
                    "The later missed record did not survive repeated acknowledgment.");
            PhoneJson.put(proof, "result", "passed");
        } catch (Exception failure) {
            PhoneJson.put(proof, "result", "failed"); PhoneJson.put(proof, "error", PhoneJson.text(failure.toString(), 1024, false));
        } finally {
            PhoneJson.put(proof, "inserted_rows", new JSONArray(owned));
            for (JSONObject row : owned) {
                try {
                    int removed = context.getContentResolver().delete(CallLog.Calls.CONTENT_URI,
                            CallLog.Calls._ID + "=? AND " + CallLog.Calls.DATE + "=? AND " + CallLog.Calls.NUMBER + "=?",
                            new String[]{Long.toString(row.getLong("id")), Long.toString(row.getLong("timestamp")), row.getString("number")});
                    require(removed == 1, "Cleanup did not delete exactly its identified fixture row " + row.getLong("id"));
                    deleted.put(row.getLong("id"));
                } catch (Exception failure) { cleanupErrors.put(PhoneJson.text(failure.toString(), 512, false)); }
            }
            if (initial != null) {
                try {
                    JSONArray finalRows = rows(context); PhoneJson.put(proof, "rows_after_cleanup", finalRows);
                    for (int i = 0; i < finalRows.length(); i++) for (JSONObject row : owned)
                        if (finalRows.getJSONObject(i).getLong("id") == row.getLong("id")) remaining.put(row.getLong("id"));
                    require(initial.toString().equals(finalRows.toString()), "Cleanup did not restore the initial CallLog flag snapshot.");
                    JSONObject finalReceipt = PhoneCallHistory.missed(context, new CancellationSignal());
                    PhoneJson.put(proof, "cleanup_receipt", finalReceipt);
                    require(count(finalReceipt) == 0, "Cleanup left an unacknowledged missed call.");
                } catch (Exception failure) { cleanupErrors.put(PhoneJson.text(failure.toString(), 512, false)); }
            }
            JSONObject cleanup = PhoneJson.put(new JSONObject(), "deleted_ids", deleted);
            PhoneJson.put(cleanup, "remaining_ids", remaining); PhoneJson.put(cleanup, "errors", cleanupErrors);
            PhoneJson.put(proof, "cleanup", cleanup); PhoneJson.put(proof, "completed_at", System.currentTimeMillis());
            PhoneJson.put(proof, "phase", "complete");
            if (cleanupErrors.length() > 0) PhoneJson.put(proof, "result", "failed");
            try {
                publish(directory, proof, owned);
            } catch (Exception failure) { android.util.Log.e("PhoneCallFixtures", "Could not retain CallLog fixture evidence", failure); }
        }
    }
}
