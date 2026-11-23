package org.ferroui.phone;

import android.app.Notification;
import android.app.NotificationChannel;
import android.app.NotificationManager;
import android.app.PendingIntent;
import android.content.Context;
import android.content.Intent;
import android.net.Uri;

final class PhoneMessageNotifications {
    private static final String CHANNEL = "phone-messages";
    private PhoneMessageNotifications() {}
    static void show(Context context, String title, String body, String thread) {
        NotificationManager manager = context.getSystemService(NotificationManager.class);
        if (manager == null || !"granted".equals(PhoneServices.notificationPermission(context))) return;
        android.content.SharedPreferences policy = PhoneNotificationPolicy.preferences(context);
        boolean center = policy.getBoolean("message_center", true), banner = policy.getBoolean("message_banners", true);
        if (!policy.getBoolean("enabled", true) || !center && !banner) return;
        boolean sound = policy.getBoolean("message_sound", true), vibration = policy.getBoolean("message_vibration", true);
        String channelId = CHANNEL + "-" + (banner ? "banner" : "center") + (sound ? "-sound" : "-quiet") + (vibration ? "-vibrate" : "");
        NotificationChannel channel = new NotificationChannel(channelId, "Messages: " + (banner ? "banners" : "notification center") + (sound ? ", sound" : ", quiet") + (vibration ? ", vibration" : ""), banner ? NotificationManager.IMPORTANCE_HIGH : NotificationManager.IMPORTANCE_DEFAULT);
        channel.enableVibration(vibration);
        if (!sound) channel.setSound(null, null);
        manager.createNotificationChannel(channel);
        // Channel settings chosen by the Android user are never overwritten after creation.
        Intent open = new Intent(context, PhoneActivity.class).setAction(Intent.ACTION_VIEW)
                .setData(PhoneIntents.app("phone.messaging", thread == null ? "compose" : "thread/" + PhoneIntents.encode(thread)))
                .addFlags(Intent.FLAG_ACTIVITY_CLEAR_TOP | Intent.FLAG_ACTIVITY_SINGLE_TOP);
        PendingIntent pending = PendingIntent.getActivity(context, 0, open, PendingIntent.FLAG_UPDATE_CURRENT | PendingIntent.FLAG_IMMUTABLE);
        Notification.Builder builder = new Notification.Builder(context, channelId).setSmallIcon(android.R.drawable.sym_action_chat)
                .setContentTitle(PhoneJson.text(title, 512, false)).setContentText(PhoneJson.text(body, 4096, true))
                .setStyle(new Notification.BigTextStyle().bigText(PhoneJson.text(body, 4096, true)))
                .setCategory(Notification.CATEGORY_MESSAGE).setVisibility(policy.getBoolean("lock_screen", true) ? Notification.VISIBILITY_PRIVATE : Notification.VISIBILITY_SECRET)
                .setContentIntent(pending).setAutoCancel(true);
        if (!center) builder.setTimeoutAfter(6000);
        Notification notification = builder.build();
        try { manager.notify(thread == null ? "messages" : thread, 1, notification); }
        catch (SecurityException denied) { }
    }
    static void read(Context context, String thread) {
        NotificationManager manager = context.getSystemService(NotificationManager.class);
        if (manager != null) manager.cancel(thread, 1);
    }
}
