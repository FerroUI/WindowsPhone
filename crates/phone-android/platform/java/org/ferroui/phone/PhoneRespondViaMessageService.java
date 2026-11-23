package org.ferroui.phone;

import android.app.Service;
import android.content.Intent;
import android.os.IBinder;
import android.telephony.TelephonyManager;
import org.json.JSONArray;
import org.json.JSONObject;

/** Telecom's protected explicit 'respond with text' action, independent of the foreground shell. */
public final class PhoneRespondViaMessageService extends Service {
    @Override public IBinder onBind(Intent intent) { return null; }
    @Override public int onStartCommand(Intent intent, int flags, int startId) {
        try {
            if (intent == null || !TelephonyManager.ACTION_RESPOND_VIA_MESSAGE.equals(intent.getAction()) || !PhoneRoles.held(this, "sms")) return START_NOT_STICKY;
            JSONArray recipients = PhoneIntents.recipients(intent.getData());
            CharSequence body = intent.getCharSequenceExtra(Intent.EXTRA_TEXT);
            if (!PhoneSmsSender.validRecipients(recipients) || body == null || body.toString().getBytes(java.nio.charset.StandardCharsets.UTF_8).length > 16000) return START_NOT_STICKY;
            String client = "telecom-reply:" + java.util.UUID.randomUUID();
            JSONObject request = PhoneJson.put(new JSONObject(), "client_id", client);
            PhoneJson.put(request, "recipients", recipients); PhoneJson.put(request, "body", body.toString());
            int subscription = PhoneMessages.subscription(intent);
            PhoneJson.put(request, "subscription", android.telephony.SubscriptionManager.isValidSubscriptionId(subscription)
                    ? "sms-subscription:" + subscription : JSONObject.NULL);
            PhoneMessageJournal.get(this).enqueue(client, "replySms", request);
            PhoneMessageJobService.wake(this);
        } finally { stopSelf(startId); }
        return START_NOT_STICKY;
    }
}
