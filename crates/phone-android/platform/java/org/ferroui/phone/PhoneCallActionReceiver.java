package org.ferroui.phone;

import android.content.BroadcastReceiver;
import android.content.Context;
import android.content.Intent;

/** Non-exported receiver reached only through a specific immutable notification PendingIntent. */
public final class PhoneCallActionReceiver extends BroadcastReceiver {
    @Override public void onReceive(Context context, Intent intent) {
        String command = intent.getStringExtra("command");
        String id = intent.getStringExtra("call");
        if (id == null || id.length() > 128 || !("end".equals(command) || "reject".equals(command))) return;
        PhoneInCallService.action(id, PhoneJson.value(command));
    }
}
