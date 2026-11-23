package org.ferroui.phone;

import android.content.Context;
import android.database.Cursor;
import android.os.CancellationSignal;
import android.provider.Telephony;
import java.io.File;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.util.concurrent.atomic.AtomicBoolean;
import org.json.JSONArray;
import org.json.JSONObject;

/** Opt-in private debug observations. This cannot grant a role, inject activation or initiate carrier traffic. */
final class PhoneDeviceSmoke {
    private static String active;
    private static long deadline, sequence;
    private static JSONObject codec;
    private static final AtomicBoolean busy = new AtomicBoolean();
    private PhoneDeviceSmoke() {}
    static void start(Context context) {
        if (!BuildConfig.DEBUG || active != null) return;
        File directory = new File(context.getFilesDir(), "windows-phone");
        File enabled = new File(directory, "phone-device-smoke.enabled");
        try {
            if (!enabled.isFile() || enabled.length() > 65) return;
            String nonce = new String(Files.readAllBytes(enabled.toPath()), StandardCharsets.UTF_8).trim();
            if (!nonce.matches("[0-9a-f]{32}")) return;
            active = nonce; deadline = android.os.SystemClock.elapsedRealtime() + 600000;
            tick(context.getApplicationContext(), directory, enabled, nonce);
        } catch (Exception unavailable) { }
    }
    private static void tick(Context context, File directory, File marker, String nonce) {
        if (!BuildConfig.DEBUG || !nonce.equals(active) || !marker.isFile() || android.os.SystemClock.elapsedRealtime() > deadline) { active = null; return; }
        if (busy.compareAndSet(false, true)) {
            JSONObject proof = PhoneJson.put(new JSONObject(), "schema", 1);
            PhoneJson.put(proof, "nonce", nonce); PhoneJson.put(proof, "sequence", ++sequence); PhoneJson.put(proof, "observed_at", System.currentTimeMillis());
            PhoneJson.put(proof, "snapshot", PhoneDevice.snapshot(context).optJSONObject("response").optJSONObject("value"));
            PhoneJson.put(proof, "calls", PhoneInCallService.snapshot(context)); PhoneJson.put(proof, "native_call_visible", PhoneCallActivity.isVisible());
            PhoneJson.put(proof, "call_presentation", PhoneInCallService.presentation(context));
            PhoneSmsReceiver.INCOMING.execute(() -> {
                try {
                    if (codec == null) codec = PhoneCodecFixtures.run(context);
                    PhoneJson.put(proof, "codec", codec);
                    if (PhoneMessageProvider.readable(context)) {
                        String body = "FerroUI SMS " + nonce;
                        try (Cursor row = context.getContentResolver().query(Telephony.Sms.CONTENT_URI, new String[]{"_id", "thread_id", "body", "address", "type"}, "address=? AND body=?", new String[]{"5550101", body}, "date DESC")) {
                            if (row != null && row.moveToFirst()) {
                                JSONObject sms = PhoneJson.put(new JSONObject(), "id", "sms:" + row.getLong(0));
                                PhoneJson.put(sms, "thread_id", "thread:" + row.getLong(1)); PhoneJson.put(sms, "body_sha256", PhoneMessageJournal.digest(row.getString(2)));
                                PhoneJson.put(sms, "type", row.getInt(4)); PhoneJson.put(sms, "matching_rows", row.getCount());
                                JSONObject messages = PhoneMessageProvider.messages(context, "thread:" + row.getLong(1), 32, new CancellationSignal());
                                JSONObject threads = PhoneMessageProvider.threads(context, 32, new CancellationSignal());
                                PhoneJson.put(sms, "bridge_messages", messages.optJSONObject("response")); PhoneJson.put(sms, "bridge_threads", threads.optJSONObject("response"));
                                PhoneJson.put(proof, "received_sms", sms);
                            }
                        }
                    }
                } catch (Exception failure) { PhoneJson.put(proof, "error", PhoneJson.text(failure.toString(), 512, false)); }
                try {
                    File temporary = new File(directory, "phone-device-smoke.tmp"); Files.write(temporary.toPath(), proof.toString().getBytes(StandardCharsets.UTF_8));
                    Files.move(temporary.toPath(), new File(directory, "phone-device-smoke.json").toPath(), java.nio.file.StandardCopyOption.REPLACE_EXISTING, java.nio.file.StandardCopyOption.ATOMIC_MOVE);
                } catch (Exception unavailable) { }
                finally { busy.set(false); }
            });
        }
        PhoneTasks.MAIN.postDelayed(() -> tick(context, directory, marker, nonce), 500);
    }
}
