package org.ferroui.phone;

import android.content.Intent;
import android.content.Context;
import android.net.Uri;
import android.os.Bundle;
import android.view.KeyEvent;
import java.lang.ref.WeakReference;
import org.ferroui.android.FerroMainActivity;

/** The framework retains rendering, input, storage, permissions and activity lifetime. */
public class PhoneActivity extends FerroMainActivity {
    private static WeakReference<PhoneActivity> current = new WeakReference<>(null);
    private boolean resumed;
    private final PhoneBackKey backKey = new PhoneBackKey();

    public static PhoneActivity current() {
        PhoneActivity activity = current.get();
        return activity == null || activity.isFinishing() || activity.isDestroyed() ? null : activity;
    }

    public static PhoneActivity foreground() {
        PhoneActivity activity = current();
        return activity != null && activity.resumed ? activity : null;
    }

    static Intent openIntent(Context context) {
        PhoneActivity activity = current();
        Intent intent = activity == null ? context.getPackageManager().getLaunchIntentForPackage(context.getPackageName())
                : new Intent(Intent.ACTION_MAIN).setComponent(activity.getComponentName());
        return intent == null ? null : intent.addFlags(Intent.FLAG_ACTIVITY_CLEAR_TOP | Intent.FLAG_ACTIVITY_SINGLE_TOP);
    }

    @Override protected void onCreate(Bundle saved) {
        current = new WeakReference<>(this);
        PhoneDevice.attach(this);
        setIntent(PhoneIntents.activation(getIntent()));
        super.onCreate(saved);
    }

    @Override protected void onNewIntent(Intent intent) {
        Intent routed = PhoneIntents.activation(intent);
        setIntent(routed);
        super.onNewIntent(routed);
    }

    @Override protected void onResume() {
        current = new WeakReference<>(this);
        resumed = true;
        super.onResume();
        PhoneDevice.resumed(this);
        PhoneLauncherAppearance.apply(this);
    }

    @Override protected void onPause() {
        resumed = false;
        backKey.cancel();
        super.onPause();
    }

    @Override @SuppressWarnings("deprecation")
    public boolean dispatchKeyEvent(KeyEvent event) {
        if (event.getKeyCode() != KeyEvent.KEYCODE_BACK) return super.dispatchKeyEvent(event);
        // The pinned Android keyboard adapter also maps BACK to Escape. Keep a
        // physical pair out of that path, then let FerroActivity raise the normal
        // BackRequested callback once. Its shared App/HOME policy stays authoritative.
        // IME handling precedes this Activity callback; predictive gestures keep
        // using FerroActivity's existing window OnBackInvokedDispatcher callback.
        if (event.getAction() == KeyEvent.ACTION_DOWN) {
            backKey.down(event.getDeviceId(), event.getDownTime(), event.getRepeatCount());
        } else if (event.getAction() == KeyEvent.ACTION_UP
                && backKey.up(event.getDeviceId(), event.getDownTime(), event.isCanceled())) {
            super.onBackPressed();
        }
        return true;
    }

    @Override protected void onDestroy() {
        if (current.get() == this) current.clear();
        PhoneLocation.activityDestroyed(this);
        PhoneMessagePicker.activityDestroyed(this);
        super.onDestroy();
    }

    @Override protected void onActivityResult(int request, int result, Intent data) {
        super.onActivityResult(request, result, data);
        if (request == PhoneMessagePicker.REQUEST) PhoneMessagePicker.result(this, result, data);
    }

    @Override public void onRequestPermissionsResult(int request, String[] permissions, int[] grants) {
        super.onRequestPermissionsResult(request, permissions, grants);
        if (request == PhonePermissions.REQUEST) PhonePermissions.result(this, permissions, grants);
        if (request == PhoneLocation.PERMISSION_REQUEST) PhoneLocation.permissionResult(this);
        if (request == PhoneServices.NOTIFICATION_PERMISSION) PhoneServices.notificationResult(this, permissions, grants);
    }
}
