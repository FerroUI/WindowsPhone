package org.ferroui.phone;

import android.app.Service;
import android.content.Intent;
import android.content.pm.ServiceInfo;
import android.os.Build;
import android.os.IBinder;
import android.util.Log;
import java.lang.ref.WeakReference;

/** Keeps user-started playback eligible while the shared native decoder runs. */
public final class PhoneMediaService extends Service {
    private WeakReference<PhoneMediaSession> owner = new WeakReference<>(null);
    private final Runnable idleTimeout = () -> {
        PhoneMediaSession media = owner.get();
        if (media == null || !media.playing()) stopSelf();
    };

    @Override public int onStartCommand(Intent intent, int flags, int id) {
        PhoneMediaSession media = PhoneMediaSession.current.get();
        owner = new WeakReference<>(media);
        PhoneTasks.MAIN.removeCallbacks(idleTimeout);
        // Always fulfill startForegroundService's promise, even if a pause/close
        // arrived before this callback. Otherwise Android can terminate the app.
        try {
            if (Build.VERSION.SDK_INT >= 29) startForeground(PhoneMediaSession.NOTIFICATION,
                    media == null ? PhoneMediaSession.endedNotification(this) : media.notification(),
                    ServiceInfo.FOREGROUND_SERVICE_TYPE_MEDIA_PLAYBACK);
            else startForeground(PhoneMediaSession.NOTIFICATION,
                    media == null ? PhoneMediaSession.endedNotification(this) : media.notification());
        } catch (RuntimeException denied) {
            Log.w("windows-phone", "Android denied the media foreground service", denied);
            stopSelf();
            return START_NOT_STICKY;
        }
        // Queue Play only after foreground promotion. The Rust decoder may then
        // request audio focus even when this action came from the lock screen.
        boolean command = media != null && media.serviceCommand(intent);
        if (media == null) { stopForeground(STOP_FOREGROUND_REMOVE); stopSelf(); }
        else if (!media.playing()) {
            if (command && "play".equals(intent.getAction())) PhoneTasks.MAIN.postDelayed(idleTimeout, 5000);
            else stopSelf();
        }
        return START_NOT_STICKY;
    }

    @Override public void onTaskRemoved(Intent intent) {
        PhoneMediaSession media = owner.get();
        if (media != null) media.taskRemoved();
        stopForeground(STOP_FOREGROUND_REMOVE);
        stopSelf();
    }

    @Override public void onDestroy() {
        PhoneTasks.MAIN.removeCallbacks(idleTimeout);
        PhoneMediaSession media = owner.get();
        if (media != null) media.serviceStopped();
        stopForeground(STOP_FOREGROUND_DETACH);
        super.onDestroy();
    }

    @Override public IBinder onBind(Intent intent) { return null; }
}
