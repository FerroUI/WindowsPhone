package org.ferroui.phone;

import android.content.BroadcastReceiver;
import android.content.Context;
import android.content.Intent;
import android.telephony.SmsManager;
import android.util.Base64;
import org.json.JSONObject;

/** Explicit private carrier callbacks; only the system holding the PendingIntent can invoke them. */
public final class PhoneMmsResultReceiver extends BroadcastReceiver {
    @Override public void onReceive(Context context, Intent intent) {
        String token = intent.getStringExtra("token"), phase = intent.getStringExtra("phase");
        if (token == null || !token.matches("[0-9a-f-]{36}")
                || !("sent".equals(phase) || "download".equals(phase) || "ack".equals(phase))) return;
        JSONObject row = PhoneJson.put(new JSONObject(), "token", token);
        PhoneJson.put(row, "phase", phase); PhoneJson.put(row, "result", getResultCode());
        byte[] pdu = intent.getByteArrayExtra(SmsManager.EXTRA_MMS_DATA);
        if (pdu != null && pdu.length <= 128000) PhoneJson.put(row, "pdu", Base64.encodeToString(pdu, Base64.NO_WRAP));
        PhoneMessageJournal.get(context).enqueue("mms-result:" + token + ":" + phase, "mmsResult", row);
        PhoneMessageJobService.wake(context);
    }
}
