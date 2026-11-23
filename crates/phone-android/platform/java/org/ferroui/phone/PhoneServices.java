package org.ferroui.phone;

import android.Manifest;
import android.app.Activity;
import android.app.Notification;
import android.app.NotificationChannel;
import android.app.NotificationManager;
import android.app.PendingIntent;
import android.content.Context;
import android.content.Intent;
import android.content.pm.PackageManager;
import android.net.Uri;
import android.os.Build;
import android.provider.Settings;
import android.view.HapticFeedbackConstants;
import org.json.JSONObject;

final class PhoneServices {
    private static final String CHANNEL = "phone-foreground-alerts";
    // The pinned FerroUI allocator starts at 20000; keep private activity codes below it.
    static final int NOTIFICATION_PERMISSION = 0x1001;
    private PhoneServices() {}

    static JSONObject settings(Activity activity, String page) {
        return PhoneSettings.open(activity, page);
    }

    static JSONObject haptic(Activity activity, String kind) {
        int feedback;
        switch (kind) {
            case "selection": feedback = HapticFeedbackConstants.CLOCK_TICK; break;
            case "light": feedback = HapticFeedbackConstants.VIRTUAL_KEY; break;
            case "medium": feedback = HapticFeedbackConstants.LONG_PRESS; break;
            case "heavy": feedback = Build.VERSION.SDK_INT >= 30
                    ? HapticFeedbackConstants.CONFIRM : HapticFeedbackConstants.LONG_PRESS; break;
            default: return PhoneJson.error("Unknown haptic feedback kind.");
        }
        // The View API honours the device/user policy and needs no vibration permission.
        return activity.getWindow().getDecorView().performHapticFeedback(feedback)
                ? PhoneJson.accepted("Android accepted the haptic feedback request.")
                : PhoneJson.error("Haptic feedback is disabled or unavailable on this device.");
    }

    static JSONObject share(Activity activity, JSONObject arguments) {
        String title = arguments.optString("title", "");
        String text = arguments.optString("text", "");
        String url = arguments.optString("url", "");
        if (title.length() > 512 || text.length() > 65536 || url.length() > 4096)
            return PhoneJson.error("The shared text exceeds the Android transfer limit.");
        if (!url.isEmpty()) text += (text.isEmpty() ? "" : "\n") + url;
        Intent send = new Intent(Intent.ACTION_SEND).setType("text/plain")
                .putExtra(Intent.EXTRA_TEXT, text).putExtra(Intent.EXTRA_SUBJECT, title);
        activity.startActivity(Intent.createChooser(send, title.isEmpty() ? null : title));
        return PhoneJson.accepted("Android opened the share chooser; no recipient or delivery is assumed.");
    }

    static String notificationPermission(Context context) {
        NotificationManager manager = context.getSystemService(NotificationManager.class);
        if (manager == null) return "unavailable";
        if (Build.VERSION.SDK_INT >= 33
                && context.checkSelfPermission(Manifest.permission.POST_NOTIFICATIONS) != PackageManager.PERMISSION_GRANTED) {
            return context.getSharedPreferences("phone-host", Context.MODE_PRIVATE)
                    .getBoolean("notification-permission-result", false) ? "denied" : "prompt";
        }
        return manager.areNotificationsEnabled() ? "granted" : "denied";
    }

    static JSONObject requestNotifications(Activity activity) {
        if ("granted".equals(notificationPermission(activity)))
            return PhoneJson.accepted("Android notifications are allowed.");
        if (Build.VERSION.SDK_INT >= 33) {
            boolean asked = activity.getSharedPreferences("phone-host", Context.MODE_PRIVATE)
                    .getBoolean("notification-permission-result", false);
            if (!asked || activity.shouldShowRequestPermissionRationale(Manifest.permission.POST_NOTIFICATIONS)) {
                activity.requestPermissions(new String[]{Manifest.permission.POST_NOTIFICATIONS}, NOTIFICATION_PERMISSION);
                return PhoneJson.accepted("Android opened notification permission consent. The result is observed when you return.");
            }
        }
        return settings(activity, "notifications");
    }

    static void notificationResult(Activity activity, String[] permissions, int[] grants) {
        // A dismissed/cancelled dialog is not a denial and remains eligible to prompt.
        for (int index = 0; index < permissions.length && index < grants.length; index++) {
            if (Manifest.permission.POST_NOTIFICATIONS.equals(permissions[index])) {
                activity.getSharedPreferences("phone-host", Context.MODE_PRIVATE).edit()
                        .putBoolean("notification-permission-result", true).apply();
                return;
            }
        }
    }

    static JSONObject notify(Context context, String title, String body) {
        if (!"granted".equals(notificationPermission(context)))
            return PhoneJson.error("Android notification permission has not been granted.");
        if (title.length() > 512 || body.length() > 4096)
            return PhoneJson.error("The notification exceeds the Android text limit.");
        NotificationManager manager = context.getSystemService(NotificationManager.class);
        NotificationChannel channel = new NotificationChannel(CHANNEL, "Phone alerts", NotificationManager.IMPORTANCE_DEFAULT);
        channel.setDescription("Foreground Windows Phone alarms and reminders");
        channel.setSound(null, null);
        channel.enableVibration(false);
        manager.createNotificationChannel(channel);
        Intent open = PhoneActivity.openIntent(context);
        PendingIntent content = open == null ? null : PendingIntent.getActivity(context, 0, open,
                PendingIntent.FLAG_UPDATE_CURRENT | PendingIntent.FLAG_IMMUTABLE);
        Notification notification = new Notification.Builder(context, CHANNEL)
                .setSmallIcon(android.R.drawable.ic_dialog_info).setContentTitle(title).setContentText(body)
                .setStyle(new Notification.BigTextStyle().bigText(body)).setContentIntent(content)
                .setAutoCancel(true).build();
        manager.notify(1, notification);
        return PhoneJson.accepted("Android accepted the notification. Device policy controls its presentation.");
    }
}
