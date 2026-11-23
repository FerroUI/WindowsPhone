package org.ferroui.phone;

import android.content.BroadcastReceiver;
import android.content.Context;
import android.content.Intent;
import android.util.Base64;
import org.json.JSONObject;

/** Private immutable sent callbacks and narrowly mutable delivery-report callbacks. */
public final class PhoneSmsResultReceiver extends BroadcastReceiver {
    @Override public void onReceive(Context context, Intent intent) {
        String token = intent.getStringExtra("token"), phase = intent.getStringExtra("phase");
        int part = intent.getIntExtra("part", -1);
        if (token == null || !token.matches("[0-9a-f-]{36}") || part < 0 || part >= 256
                || !("sent".equals(phase) || "delivered".equals(phase))) return;
        JSONObject row = PhoneJson.put(new JSONObject(), "token", token);
        PhoneJson.put(row, "phase", phase); PhoneJson.put(row, "part", part); PhoneJson.put(row, "result", getResultCode());
        byte[] pdu = intent.getByteArrayExtra("pdu");
        if (pdu != null && pdu.length <= 65536) PhoneJson.put(row, "pdu", Base64.encodeToString(pdu, Base64.NO_WRAP));
        PhoneJson.put(row, "format", intent.getStringExtra("format"));
        PhoneMessageJournal.get(context).enqueue("sms-result:" + token + ":" + phase + ":" + part + ":" + PhoneMessageJournal.digest(row.toString()), "smsResult", row);
        PhoneMessageJobService.wake(context);
    }
}
