package org.ferroui.phone;

import android.content.BroadcastReceiver;
import android.content.Context;
import android.content.Intent;
import android.provider.Telephony;
import android.telephony.SmsMessage;
import android.telephony.SubscriptionManager;
import android.util.Base64;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;
import org.json.JSONArray;
import org.json.JSONObject;

/** Only the protected, default-app SMS_DELIVER broadcast enters the durable incoming queue. */
public final class PhoneSmsReceiver extends BroadcastReceiver {
    static final ExecutorService INCOMING = Executors.newSingleThreadExecutor(r -> { Thread t = new Thread(r, "phone-incoming-message"); t.setDaemon(true); return t; });
    @Override public void onReceive(Context context, Intent intent) {
        if (!Telephony.Sms.Intents.SMS_DELIVER_ACTION.equals(intent.getAction()) || !PhoneRoles.held(context, "sms")) return;
        PendingResult result = goAsync();
        Context application = context.getApplicationContext();
        INCOMING.execute(() -> {
            try {
                if (!PhoneRoles.held(application, "sms")) { result.setResultCode(Telephony.Sms.Intents.RESULT_SMS_GENERIC_ERROR); return; }
                Object[] pdus = intent.getExtras() == null ? null : (Object[]) intent.getExtras().get("pdus");
                if (pdus == null || pdus.length == 0 || pdus.length > 256) throw new IllegalArgumentException("Invalid SMS segments.");
                JSONArray parts = new JSONArray(); int total = 0;
                for (Object value : pdus) {
                    if (!(value instanceof byte[])) throw new IllegalArgumentException("Invalid SMS segment.");
                    byte[] pdu = (byte[]) value; total += pdu.length;
                    if (pdu.length > 4096 || total > 128000) throw new IllegalArgumentException("SMS exceeds the receive limit.");
                    parts.put(Base64.encodeToString(pdu, Base64.NO_WRAP));
                }
                JSONObject row = PhoneJson.put(new JSONObject(), "parts", parts);
                PhoneJson.put(row, "format", intent.getStringExtra("format"));
                PhoneJson.put(row, "subscription", PhoneMessages.subscription(intent));
                PhoneJson.put(row, "received_at", System.currentTimeMillis());
                String token = PhoneMessageJournal.digest(parts.toString() + ":" + row.optString("format") + ":" + row.optInt("subscription"));
                PhoneJson.put(row, "identity", token);
                PhoneMessageJournal.get(application).enqueue("incoming-sms:" + token, "incomingSms", row);
                PhoneMessageJobService.wake(application);
                result.setResultCode(Telephony.Sms.Intents.RESULT_SMS_HANDLED);
            } catch (RuntimeException unavailable) { result.setResultCode(Telephony.Sms.Intents.RESULT_SMS_OUT_OF_MEMORY); }
            finally { result.finish(); }
        });
    }
}
