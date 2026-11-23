package org.ferroui.phone;

import android.content.BroadcastReceiver;
import android.content.Context;
import android.content.Intent;

/** Resume already-journalled carrier receipts only; this never creates or retries an outgoing send. */
public final class PhoneMessageBootReceiver extends BroadcastReceiver {
    @Override public void onReceive(Context context, Intent intent) {
        String action = intent.getAction();
        if (!(Intent.ACTION_BOOT_COMPLETED.equals(action) || Intent.ACTION_MY_PACKAGE_REPLACED.equals(action)
                || android.provider.Telephony.Sms.Intents.ACTION_DEFAULT_SMS_PACKAGE_CHANGED.equals(action)
                || android.provider.Telephony.Sms.Intents.ACTION_EXTERNAL_PROVIDER_CHANGE.equals(action))) return;
        PhoneDevice.messagesChanged();
        Context application = context.getApplicationContext();
        PendingResult pending = goAsync();
        PhoneSmsReceiver.INCOMING.execute(() -> {
            try {
                if (application.getDatabasePath("phone-message-journal.db").isFile()) {
                    PhoneMessageJournal journal = PhoneMessageJournal.get(application);
                    if (PhoneRoles.held(application, "sms")) journal.prune(application); else journal.roleRevoked(application);
                }
            } catch (RuntimeException unavailable) {
                android.util.Log.w("PhoneMessageJournal", "Native message maintenance failed; retained work can be retried.");
            } finally {
                try { PhoneMessageJobService.wake(application); }
                finally { pending.finish(); }
            }
        });
    }
}
