package org.ferroui.phone;

import android.content.Context;
import android.os.Looper;
import org.json.JSONObject;

/** Small UTF-8/JSON JNI boundary; no Rust object pointers cross into Java callbacks. */
public final class PhoneBridge {
    private PhoneBridge() {}

    public static String state(Context context) {
        try {
            JSONObject state = PhoneJson.value("state");
            PhoneJson.put(state, "launcher_package", BuildConfig.PHONE_LAUNCHER);
            PhoneJson.put(state, "home_available", PhoneLauncher.available(context));
            PhoneJson.put(state, "home_held", PhoneLauncher.held(context));
            PhoneJson.put(state, "notifications", PhoneServices.notificationPermission(context));
            PhoneEnvironment.observe(context, state);
            return state.toString();
        } catch (RuntimeException error) { return PhoneJson.error("Android host state is unavailable.").toString(); }
    }

    public static String command(Context context, String action, String json) {
        if (Looper.myLooper() != Looper.getMainLooper())
            return PhoneJson.error("Android UI commands must start on the activity thread.").toString();
        if (json == null || json.length() > 131072)
            return PhoneJson.error("The Android command exceeds the transfer limit.").toString();
        try {
            JSONObject arguments = new JSONObject(json);
            PhoneActivity activity = PhoneActivity.foreground();
            if (activity == null) return PhoneJson.error("No foreground Android activity is available.").toString();
            JSONObject result;
            switch (action) {
                case "launch": result = PhoneLauncher.launch(context, arguments.optString("id")); break;
                case "request_home": result = PhoneLauncher.request(activity); break;
                case "settings": result = PhoneServices.settings(activity, arguments.optString("page")); break;
                case "haptic": result = PhoneServices.haptic(activity, arguments.optString("kind")); break;
                case "share": result = PhoneServices.share(activity, arguments); break;
                // Only the NativeHost's validated asset handoff calls this private
                // bridge operation; PlatformRequest never accepts filesystem paths.
                case "open_file": result = PhoneAssetProvider.open(activity, arguments.optString("path"), arguments.optString("mime")); break;
                case "request_notifications": result = PhoneServices.requestNotifications(activity); break;
                case "notify": result = PhoneServices.notify(context, arguments.optString("title"), arguments.optString("body")); break;
                default: result = PhoneJson.error("Unsupported Android host command.");
            }
            return result.toString();
        } catch (Exception error) {
            return PhoneJson.error("Android rejected the command: " + PhoneJson.bounded(error.getMessage(), 160)).toString();
        }
    }

    public static long applications(Context context) {
        Context application = context.getApplicationContext();
        return PhoneTasks.work(() -> PhoneLauncher.applications(application));
    }

    public static long locate() {
        if (Looper.myLooper() != Looper.getMainLooper()) return 0;
        PhoneActivity activity = PhoneActivity.foreground();
        if (activity == null) return 0;
        try { return PhoneLocation.begin(activity); }
        catch (RuntimeException error) { return 0; }
    }

    public static String poll(long id) { return PhoneTasks.poll(id); }
    public static void cancel(long id) { PhoneTasks.cancel(id); }

    public static void close(Context context) {
        // NativeHost calls this on its UI thread, including embedding teardown.
        PhoneTasks.close();
        PhoneLocation.close();
        PhoneAssetProvider.close(context);
        PhoneMediaSession media = PhoneMediaSession.current.get();
        if (media != null) media.close();
    }
}
