package org.ferroui.phone;

import android.app.Notification;
import android.app.NotificationChannel;
import android.app.NotificationManager;
import android.app.PendingIntent;
import android.app.Person;
import android.content.Context;
import android.content.Intent;
import android.net.Uri;
import android.os.Build;
import android.telecom.Call;
import java.util.HashSet;
import java.util.List;

/** Incoming/ongoing call UI remains available while the FerroUI shell is suspended. */
final class PhoneCallNotifications {
    private static final String INCOMING = "phone-incoming-calls", ONGOING = "phone-ongoing-calls";
    private static final int ID = 40;
    private static final HashSet<String> posted = new HashSet<>();
    private PhoneCallNotifications() {}

    static void update(Context context, List<PhoneInCallService.Entry> calls) {
        NotificationManager manager = context.getSystemService(NotificationManager.class);
        if (manager == null) return;
        NotificationChannel incoming = new NotificationChannel(INCOMING, "Incoming calls", NotificationManager.IMPORTANCE_HIGH);
        incoming.setDescription("Incoming calls when Windows Phone is the selected phone app");
        // Android Telecom remains responsible for ringing; the service does not claim IN_CALL_SERVICE_RINGING.
        incoming.setSound(null, null);
        NotificationChannel ongoing = new NotificationChannel(ONGOING, "Ongoing calls", NotificationManager.IMPORTANCE_LOW);
        ongoing.setSound(null, null);
        manager.createNotificationChannel(incoming);
        manager.createNotificationChannel(ongoing);
        HashSet<String> active = new HashSet<>();
        for (PhoneInCallService.Entry entry : calls) {
            int state = PhoneInCallService.state(entry.call);
            if (state == Call.STATE_DISCONNECTED || state == Call.STATE_DISCONNECTING) continue;
            active.add(entry.id);
            boolean ringing = state == Call.STATE_RINGING || state == Call.STATE_SIMULATED_RINGING;
            Call.Details details = entry.call.getDetails();
            String title = PhoneJson.text(details.getCallerDisplayName(), 512, false).trim();
            if (title.isEmpty() && details.getHandle() != null) title = PhoneJson.text(details.getHandle().getSchemeSpecificPart(), 256, false);
            if (title.isEmpty()) title = "Unknown caller";
            PendingIntent open = activity(context, entry.id, null);
            PendingIntent end = action(context, entry.id, ringing ? "reject" : "end");
            Notification.Builder builder = new Notification.Builder(context, ringing ? INCOMING : ONGOING)
                    .setSmallIcon(android.R.drawable.sym_action_call).setContentTitle(title)
                    .setContentText(ringing ? "Incoming call" : state == Call.STATE_HOLDING ? "Call on hold" : "Phone call")
                    .setCategory(Notification.CATEGORY_CALL).setVisibility(Notification.VISIBILITY_PRIVATE)
                    .setOngoing(true).setOnlyAlertOnce(true).setContentIntent(open);
            if (ringing) {
                PendingIntent answer = activity(context, entry.id, "answer");
                if (Build.VERSION.SDK_INT < 34 || manager.canUseFullScreenIntent()) builder.setFullScreenIntent(open, true);
                if (Build.VERSION.SDK_INT >= 31) builder.setStyle(Notification.CallStyle.forIncomingCall(
                        new Person.Builder().setName(title).setImportant(true).build(), end, answer));
                else builder.addAction(new Notification.Action.Builder(null, "Decline", end).build())
                        .addAction(new Notification.Action.Builder(null, "Answer", answer).build());
            } else {
                if (Build.VERSION.SDK_INT >= 31) builder.setStyle(Notification.CallStyle.forOngoingCall(
                        new Person.Builder().setName(title).setImportant(true).build(), end));
                else builder.addAction(new Notification.Action.Builder(null, "End call", end).build());
                if (details.getConnectTimeMillis() > 0) builder.setWhen(details.getConnectTimeMillis()).setUsesChronometer(true);
            }
            try { manager.notify(entry.id, ID, builder.build()); }
            catch (SecurityException permissionRevoked) { /* Android controls presentation when notification permission is revoked. */ }
        }
        for (String old : posted) if (!active.contains(old)) manager.cancel(old, ID);
        posted.clear();
        posted.addAll(active);
    }

    private static PendingIntent activity(Context context, String id, String command) {
        Intent intent = new Intent(context, PhoneCallActivity.class)
                .setData(Uri.parse("phone-call:" + id + "/" + (command == null ? "open" : command)))
                .putExtra("call", id).putExtra("command", command)
                .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK | Intent.FLAG_ACTIVITY_SINGLE_TOP);
        return PendingIntent.getActivity(context, 0, intent, PendingIntent.FLAG_UPDATE_CURRENT | PendingIntent.FLAG_IMMUTABLE);
    }

    private static PendingIntent action(Context context, String id, String command) {
        Intent intent = new Intent(context, PhoneCallActionReceiver.class)
                .setData(Uri.parse("phone-call:" + id + "/" + command)).putExtra("call", id).putExtra("command", command);
        return PendingIntent.getBroadcast(context, 0, intent, PendingIntent.FLAG_UPDATE_CURRENT | PendingIntent.FLAG_IMMUTABLE);
    }

    static void clear(Context context) {
        NotificationManager manager = context.getSystemService(NotificationManager.class);
        if (manager != null) for (String id : posted) manager.cancel(id, ID);
        posted.clear();
    }
}
