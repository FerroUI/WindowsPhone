package org.ferroui.phone;

import android.app.Activity;
import android.os.Build;
import android.view.View;
import android.view.WindowInsets;
import android.view.WindowInsetsController;
import java.lang.ref.WeakReference;

/** User-selected immersive presentation is limited to this app's window while it is the active HOME. */
final class PhoneLauncherAppearance {
    private static WeakReference<Activity> owner = new WeakReference<>(null);
    private static Boolean previous;
    private PhoneLauncherAppearance() {}
    static void apply(PhoneActivity activity) {
        if (activity == null) return;
        boolean enabled = PhoneLauncher.held(activity) && activity.getSharedPreferences("phone-launcher", 0).getBoolean("immersive", false);
        if (owner.get() == activity && previous != null && previous == enabled) return;
        owner = new WeakReference<>(activity); previous = enabled;
        if (Build.VERSION.SDK_INT >= 30) {
            WindowInsetsController insets = activity.getWindow().getInsetsController();
            if (insets == null) { previous = null; return; }
            if (enabled) {
                insets.setSystemBarsBehavior(WindowInsetsController.BEHAVIOR_SHOW_TRANSIENT_BARS_BY_SWIPE);
                insets.hide(WindowInsets.Type.systemBars());
            } else insets.show(WindowInsets.Type.systemBars());
        } else {
            View decor = activity.getWindow().getDecorView();
            int flags = View.SYSTEM_UI_FLAG_FULLSCREEN | View.SYSTEM_UI_FLAG_HIDE_NAVIGATION | View.SYSTEM_UI_FLAG_IMMERSIVE_STICKY;
            decor.setSystemUiVisibility(enabled ? decor.getSystemUiVisibility() | flags : decor.getSystemUiVisibility() & ~flags);
        }
    }
}
