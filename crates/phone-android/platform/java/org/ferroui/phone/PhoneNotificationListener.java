package org.ferroui.phone;

import android.app.ActivityOptions;
import android.app.KeyguardManager;
import android.app.Notification;
import android.app.NotificationManager;
import android.app.PendingIntent;
import android.app.RemoteInput;
import android.content.ComponentName;
import android.content.Context;
import android.content.Intent;
import android.os.Build;
import android.os.Bundle;
import android.os.Process;
import android.service.notification.NotificationListenerService;
import android.service.notification.StatusBarNotification;
import androidx.core.app.NotificationManagerCompat;
import java.lang.ref.WeakReference;
import java.nio.charset.StandardCharsets;
import java.security.MessageDigest;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.Comparator;
import java.util.HashMap;
import org.json.JSONArray;
import org.json.JSONObject;

/** System-bound notification access. Personal text and PendingIntents are never persisted. */
public final class PhoneNotificationListener extends NotificationListenerService {
    private static WeakReference<PhoneNotificationListener> current = new WeakReference<>(null);
    private final HashMap<String, StatusBarNotification> observed = new HashMap<>();
    private boolean connected;
    private static boolean dirty = true;

    static boolean granted(Context context) {
        if (Build.VERSION.SDK_INT >= 27) {
            NotificationManager manager = context.getSystemService(NotificationManager.class);
            return manager != null && manager.isNotificationListenerAccessGranted(new ComponentName(context, PhoneNotificationListener.class));
        }
        return NotificationManagerCompat.getEnabledListenerPackages(context).contains(context.getPackageName());
    }

    @Override public void onListenerConnected() { connected = true; current = new WeakReference<>(this); dirty = true; PhoneDevice.notificationAuthorityChanged(); }
    @Override public void onListenerDisconnected() { connected = false; observed.clear(); dirty = true; PhoneDevice.notificationAuthorityChanged(); }
    @Override public void onDestroy() {
        connected = false;
        observed.clear();
        if (current.get() == this) current.clear();
        dirty = true;
        PhoneDevice.notificationAuthorityChanged();
        super.onDestroy();
    }
    @Override public void onNotificationPosted(StatusBarNotification notification) { dirty = true; }
    @Override public void onNotificationRemoved(StatusBarNotification notification) { dirty = true; }
    @Override public void onNotificationRankingUpdate(RankingMap rankingMap) { dirty = true; }

    static boolean available(Context context) { return ready(context) != null; }

    static boolean takeDirty() { boolean result = dirty; dirty = false; return result; }

    private static PhoneNotificationListener ready(Context context) {
        PhoneNotificationListener service = current.get();
        return granted(context) && service != null && service.connected ? service : null;
    }

    private static String key(String raw) {
        try {
            byte[] digest = MessageDigest.getInstance("SHA-256").digest(raw.getBytes(StandardCharsets.UTF_8));
            StringBuilder id = new StringBuilder("notification:");
            for (byte b : digest) id.append(String.format(java.util.Locale.ROOT, "%02x", b & 255));
            return id.toString();
        } catch (java.security.NoSuchAlgorithmException impossible) { throw new IllegalStateException(impossible); }
    }

    private static String actionId(StatusBarNotification notification, int index) {
        // A replacement notification invalidates the old action identity even if its key is reused.
        Notification.Action action = notification.getNotification().actions[index];
        return index + ":" + notification.getPostTime() + ":" + Integer.toHexString(action.actionIntent.hashCode());
    }

    private static boolean replyAllowed(Notification.Action action) {
        RemoteInput[] inputs = action.getRemoteInputs();
        if (inputs != null) for (RemoteInput input : inputs) if (input.getAllowFreeFormInput()) return true;
        return false;
    }

    static JSONObject snapshot(Context context) {
        PhoneNotificationListener service = ready(context);
        if (service == null) return PhoneJson.error(granted(context)
                ? "Android has not connected the notification listener yet." : "Enable notification access in Android settings first.");
        StatusBarNotification[] all = service.getActiveNotifications();
        if (all == null) all = new StatusBarNotification[0];
        Arrays.sort(all, Comparator.comparingLong(StatusBarNotification::getPostTime).reversed());
        JSONArray rows = new JSONArray();
        HashMap<String, StatusBarNotification> next = new HashMap<>();
        int bytes = 0;
        for (StatusBarNotification item : all) {
            if (rows.length() >= 256) break;
            if (!Process.myUserHandle().equals(item.getUser())) continue;
            Notification notice = item.getNotification();
            if (notice == null) continue;
            String id = key(item.getKey());
            JSONObject row = new JSONObject();
            PhoneJson.put(row, "key", id);
            PhoneJson.put(row, "package_id", PhoneJson.text(item.getPackageName(), 1024, false));
            String application = item.getPackageName();
            try { application = context.getPackageManager().getApplicationLabel(context.getPackageManager().getApplicationInfo(application, 0)).toString(); }
            catch (Exception noLabel) { }
            application = PhoneJson.text(application, 512, false).trim();
            PhoneJson.put(row, "application", application.isEmpty() ? item.getPackageName() : application);
            PhoneJson.put(row, "title", PhoneJson.text(notice.extras.getCharSequence(Notification.EXTRA_TITLE), 2048, true));
            CharSequence body = notice.extras.getCharSequence(Notification.EXTRA_BIG_TEXT);
            if (body == null) body = notice.extras.getCharSequence(Notification.EXTRA_TEXT);
            PhoneJson.put(row, "body", PhoneJson.text(body, 8192, true));
            PhoneJson.put(row, "timestamp", item.getPostTime());
            PhoneJson.put(row, "clearable", item.isClearable());
            PhoneJson.put(row, "ongoing", item.isOngoing());
            JSONArray actions = new JSONArray();
            if (notice.actions != null) for (int index = 0; index < notice.actions.length && actions.length() < 8; index++) {
                Notification.Action action = notice.actions[index];
                if (action == null || action.actionIntent == null) continue;
                String title = PhoneJson.text(action.title, 256, false).trim();
                if (title.isEmpty()) continue;
                JSONObject entry = PhoneJson.put(new JSONObject(), "id", actionId(item, index));
                PhoneJson.put(entry, "title", title);
                PhoneJson.put(entry, "allows_reply", replyAllowed(action));
                actions.put(entry);
            }
            PhoneJson.put(row, "actions", actions);
            int size = PhoneJson.bytes(row) + 1;
            if (bytes + size > 850000) break;
            bytes += size;
            rows.put(row);
            next.put(id, item);
        }
        service.observed.clear();
        service.observed.putAll(next);
        return PhoneJson.device("notifications", rows);
    }

    private StatusBarNotification fresh(String id) {
        StatusBarNotification old = observed.get(id);
        if (old == null) return null;
        StatusBarNotification[] current = getActiveNotifications(new String[]{old.getKey()});
        return current != null && current.length == 1 && Process.myUserHandle().equals(current[0].getUser()) ? current[0] : null;
    }

    static JSONObject act(Context context, String id, String actionId, String reply) {
        PhoneNotificationListener service = ready(context);
        StatusBarNotification item = service == null ? null : service.fresh(id);
        if (item == null) return PhoneJson.error("That notification is no longer available. Refresh device notifications.");
        PendingIntent pending = item.getNotification().contentIntent;
        Intent fill = null;
        if (actionId != null) {
            Notification.Action chosen = null;
            Notification.Action[] actions = item.getNotification().actions;
            if (actions != null) for (int index = 0; index < actions.length; index++) {
                if (actions[index] != null && actions[index].actionIntent != null && actionId.equals(actionId(item, index))) chosen = actions[index];
            }
            if (chosen == null) return PhoneJson.error("That notification action changed. Refresh it before acting.");
            KeyguardManager keyguard = context.getSystemService(KeyguardManager.class);
            if (Build.VERSION.SDK_INT >= 31 && chosen.isAuthenticationRequired() && keyguard != null && keyguard.isDeviceLocked())
                return PhoneJson.error("Unlock Android before using this notification action.");
            pending = chosen.actionIntent;
            if (reply != null) {
                if (!replyAllowed(chosen) || reply.trim().isEmpty() || reply.getBytes(StandardCharsets.UTF_8).length > 4096)
                    return PhoneJson.error("That notification does not accept this text reply.");
                fill = new Intent();
                Bundle results = new Bundle();
                ArrayList<RemoteInput> allowed = new ArrayList<>();
                for (RemoteInput input : chosen.getRemoteInputs()) if (input.getAllowFreeFormInput()) {
                    allowed.add(input);
                    results.putCharSequence(input.getResultKey(), reply);
                }
                RemoteInput.addResultsToIntent(allowed.toArray(new RemoteInput[0]), fill, results);
                if (Build.VERSION.SDK_INT >= 28) RemoteInput.setResultsSource(fill, RemoteInput.SOURCE_FREE_FORM_INPUT);
            }
        } else if (reply != null) return PhoneJson.error("A reply requires a specific notification action.");
        if (pending == null) return PhoneJson.error("That notification has no open action.");
        try {
            ActivityOptions options = ActivityOptions.makeBasic();
            if (Build.VERSION.SDK_INT >= 34) options.setPendingIntentBackgroundActivityStartMode(ActivityOptions.MODE_BACKGROUND_ACTIVITY_START_ALLOWED);
            pending.send(context, 0, fill, null, null, null, options.toBundle());
            return PhoneJson.deviceAccepted("Android accepted the notification action. The originating application controls its outcome.");
        } catch (PendingIntent.CanceledException expired) { return PhoneJson.error("The originating application cancelled that notification action."); }
    }

    static JSONObject dismiss(Context context, String id) {
        PhoneNotificationListener service = ready(context);
        StatusBarNotification item = service == null ? null : service.fresh(id);
        if (item == null || !item.isClearable()) return PhoneJson.error("That notification is unavailable or cannot be dismissed.");
        service.cancelNotification(item.getKey());
        return PhoneJson.deviceAccepted("Android accepted the notification dismissal request.");
    }

    static void clearObserved() {
        PhoneNotificationListener service = current.get();
        if (service != null) service.observed.clear();
        dirty = true;
    }
}
