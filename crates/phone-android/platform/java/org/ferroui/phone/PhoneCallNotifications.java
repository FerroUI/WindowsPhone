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
import android.service.notification.StatusBarNotification;
import android.telecom.Call;
import android.util.Log;
import java.util.HashSet;
import java.util.List;
import org.json.JSONObject;

/** Incoming/ongoing call UI remains available while the FerroUI shell is suspended. */
final class PhoneCallNotifications {
    private static final String INCOMING = "phone-incoming-calls", ONGOING = "phone-ongoing-calls";
    static final int ID = 40;
    private final HashSet<String> posted = new HashSet<>();
    private String failure;
    String error() { return failure; }

    void update(PhoneInCallService service, List<PhoneInCallService.Entry> calls) {
        PhoneInCallService.Entry primary = null;
        for (PhoneInCallService.Entry entry : calls) {
            if (rank(entry) == 0) continue;
            if (primary == null || rank(entry) > rank(primary)) primary = entry;
        }
        // Only a real Telecom call under the currently held Dialer role owns this FGS.
        // Nothing starts it at boot, shell startup, or merely after requesting a role.
        if (primary == null || !PhoneRoles.held(service, "dialer")) { clear(service); return; }
        NotificationManager manager = service.getSystemService(NotificationManager.class);
        if (manager == null) return;
        failure = null;
        NotificationChannel incoming = new NotificationChannel(INCOMING, "Incoming calls", NotificationManager.IMPORTANCE_HIGH);
        incoming.setDescription("Incoming calls when Windows Phone is the selected phone app");
        // Android Telecom remains responsible for ringing; the service does not claim IN_CALL_SERVICE_RINGING.
        incoming.setSound(null, null);
        NotificationChannel ongoing = new NotificationChannel(ONGOING, "Ongoing calls", NotificationManager.IMPORTANCE_LOW);
        ongoing.setSound(null, null);
        manager.createNotificationChannel(incoming);
        manager.createNotificationChannel(ongoing);
        boolean promoted = false;
        try {
            Notification notification = build(service, manager, primary, true);
            service.startCallForeground(notification);
            promoted = true;
        } catch (RuntimeException denied) {
            // Role/foreground policy may change while Telecom dispatches a callback.
            // Keep the call surface and generic actions usable; never kill a live call UI.
            failed(denied);
            stopForeground(service);
        }
        HashSet<String> active = new HashSet<>();
        for (PhoneInCallService.Entry entry : calls) {
            if (rank(entry) == 0 || promoted && entry == primary) continue;
            // A service owns exactly one untagged foreground notification. Secondary
            // calls retain their own actions without pretending to own another FGS.
            try { manager.notify(entry.id, ID, build(service, manager, entry, false)); active.add(entry.id); }
            catch (RuntimeException denied) { failed(denied); }
        }
        for (String old : posted) if (!active.contains(old)) manager.cancel(old, ID);
        posted.clear();
        posted.addAll(active);
    }

    private static int rank(PhoneInCallService.Entry entry) {
        int state = PhoneInCallService.state(entry.call);
        if (state == Call.STATE_DISCONNECTED || state == Call.STATE_DISCONNECTING) return 0;
        if (state == Call.STATE_RINGING || state == Call.STATE_SIMULATED_RINGING) return 3;
        return state == Call.STATE_HOLDING ? 1 : 2;
    }

    private static Notification build(Context context, NotificationManager manager, PhoneInCallService.Entry entry, boolean foreground) {
        int state = PhoneInCallService.state(entry.call);
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
            if (foreground && Build.VERSION.SDK_INT >= 31) builder.setStyle(Notification.CallStyle.forIncomingCall(
                    new Person.Builder().setName(title).setImportant(true).build(), end, answer));
            else builder.addAction(new Notification.Action.Builder(null, "Decline", end).build())
                    .addAction(new Notification.Action.Builder(null, "Answer", answer).build());
        } else {
            if (foreground && Build.VERSION.SDK_INT >= 31) builder.setStyle(Notification.CallStyle.forOngoingCall(
                    new Person.Builder().setName(title).setImportant(true).build(), end));
            else builder.addAction(new Notification.Action.Builder(null, "End call", end).build());
            if (details.getConnectTimeMillis() > 0) builder.setWhen(details.getConnectTimeMillis()).setUsesChronometer(true);
        }
        return builder.build();
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

    void clear(PhoneInCallService service) {
        stopForeground(service);
        NotificationManager manager = service.getSystemService(NotificationManager.class);
        if (manager != null) for (String id : posted) manager.cancel(id, ID);
        posted.clear();
    }

    private void stopForeground(PhoneInCallService service) {
        try { service.stopCallForeground(); }
        catch (RuntimeException denied) { failed(denied); }
    }

    private void failed(RuntimeException denied) {
        failure = PhoneJson.text(denied.toString(), 512, false);
        Log.w("PhoneCallNotifications", "Android rejected call-notification presentation", denied);
    }

    /** Reads Android-owned foreground/type/notification state only for the opt-in debug observer. */
    static JSONObject observe(Context context, PhoneInCallService service, String error) {
        JSONObject result = new JSONObject();
        int type = service == null || Build.VERSION.SDK_INT < 29 ? 0 : service.getForegroundServiceType();
        boolean foreground = false, callStyle = false;
        NotificationManager manager = context.getSystemService(NotificationManager.class);
        if (manager != null) for (StatusBarNotification posted : manager.getActiveNotifications()) {
            Notification notification = posted.getNotification();
            if (posted.getId() != ID || posted.getTag() != null || !Notification.CATEGORY_CALL.equals(notification.category)) continue;
            foreground = (notification.flags & Notification.FLAG_FOREGROUND_SERVICE) != 0;
            callStyle = Build.VERSION.SDK_INT >= 31 && Notification.CallStyle.class.getName()
                    .equals(notification.extras.getString(Notification.EXTRA_TEMPLATE));
        }
        PhoneJson.put(result, "service_type", type);
        PhoneJson.put(result, "foreground_notification", foreground);
        PhoneJson.put(result, "call_style", callStyle);
        PhoneJson.put(result, "error", error);
        return result;
    }
}
