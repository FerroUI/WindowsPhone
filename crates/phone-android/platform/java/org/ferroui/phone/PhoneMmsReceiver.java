package org.ferroui.phone;

import android.content.BroadcastReceiver;
import android.content.Context;
import android.content.Intent;
import android.provider.Telephony;
import android.util.Base64;
import org.json.JSONObject;

/** WAP_PUSH_DELIVER is signature protected in the manifest and never exported as a general PDU API. */
public final class PhoneMmsReceiver extends BroadcastReceiver {
    @Override public void onReceive(Context context, Intent intent) {
        if (!Telephony.Sms.Intents.WAP_PUSH_DELIVER_ACTION.equals(intent.getAction())
                || !"application/vnd.wap.mms-message".equals(intent.getType()) || !PhoneRoles.held(context, "sms")) return;
        byte[] bytes = intent.getByteArrayExtra("data");
        if (bytes == null || bytes.length == 0 || bytes.length > 128000) return;
        Context application = context.getApplicationContext();
        PendingResult result = goAsync();
        PhoneSmsReceiver.INCOMING.execute(() -> {
            try {
                if (!PhoneRoles.held(application, "sms")) { result.setResultCode(Telephony.Sms.Intents.RESULT_SMS_GENERIC_ERROR); return; }
                JSONObject row = PhoneJson.put(new JSONObject(), "pdu", Base64.encodeToString(bytes, Base64.NO_WRAP));
                PhoneJson.put(row, "subscription", PhoneMessages.subscription(intent));
                String identity = PhoneMessageJournal.digest(bytes) + ":" + row.optInt("subscription");
                PhoneJson.put(row, "identity", identity);
                PhoneMessageJournal.get(application).enqueue("incoming-mms:" + identity, "incomingMms", row);
                PhoneMessageJobService.wake(application);
                result.setResultCode(Telephony.Sms.Intents.RESULT_SMS_HANDLED);
            } catch (RuntimeException unavailable) { result.setResultCode(Telephony.Sms.Intents.RESULT_SMS_OUT_OF_MEMORY); }
            finally { result.finish(); }
        });
    }
}
