package org.ferroui.phone;

import android.Manifest;
import android.app.Activity;
import android.content.Context;
import android.content.pm.PackageManager;
import android.os.Build;
import java.util.ArrayList;
import org.json.JSONObject;

/** Runtime permission state stays separate from permission requests and role grants. */
final class PhonePermissions {
    static final int REQUEST = 0x1003;
    private PhonePermissions() {}

    static boolean granted(Context context, String permission) {
        return context.checkSelfPermission(permission) == PackageManager.PERMISSION_GRANTED;
    }

    static String state(Context context, String permission) {
        if (granted(context, permission)) return "granted";
        return context.getSharedPreferences("phone-host", Context.MODE_PRIVATE)
                .getBoolean("permission-result:" + permission, false) ? "denied" : "prompt";
    }

    static String[] names(String permission) {
        switch (permission) {
            case "contacts": return new String[]{Manifest.permission.READ_CONTACTS};
            case "callLog": return new String[]{Manifest.permission.READ_CALL_LOG};
            case "phone": return new String[]{Manifest.permission.CALL_PHONE, Manifest.permission.READ_PHONE_STATE};
            case "readSms": return new String[]{Manifest.permission.READ_SMS, Manifest.permission.RECEIVE_SMS,
                    Manifest.permission.RECEIVE_MMS, Manifest.permission.RECEIVE_WAP_PUSH};
            case "sendSms": return new String[]{Manifest.permission.SEND_SMS};
            case "camera": return new String[]{Manifest.permission.CAMERA};
            case "bluetooth": return Build.VERSION.SDK_INT >= 31
                    ? new String[]{Manifest.permission.BLUETOOTH_CONNECT} : new String[]{Manifest.permission.BLUETOOTH};
            case "notifications": return Build.VERSION.SDK_INT >= 33
                    ? new String[]{Manifest.permission.POST_NOTIFICATIONS} : new String[0];
            case "location": return new String[]{Manifest.permission.ACCESS_COARSE_LOCATION, Manifest.permission.ACCESS_FINE_LOCATION};
            default: return null;
        }
    }

    static String groupState(Context context, String permission) {
        if ("notifications".equals(permission)) { String state = PhoneServices.notificationPermission(context); return "unavailable".equals(state) ? "unsupported" : state; }
        String[] names = names(permission);
        if (names == null) return "unsupported";
        if ("location".equals(permission) && granted(context, Manifest.permission.ACCESS_COARSE_LOCATION)
                && !granted(context, Manifest.permission.ACCESS_FINE_LOCATION)) return "limited";
        String state = "granted";
        for (String name : names) {
            String next = state(context, name);
            if ("denied".equals(next)) return "denied";
            if ("prompt".equals(next)) state = "prompt";
        }
        return state;
    }

    static JSONObject request(Activity activity, String permission) {
        if ("notifications".equals(permission)) return PhoneServices.requestNotifications(activity);
        if (("readSms".equals(permission) || "sendSms".equals(permission)) && !PhoneRoles.held(activity, "sms"))
            return PhoneJson.error("Select Windows Phone as the default SMS application before requesting SMS access.");
        if ("callLog".equals(permission) && !PhoneRoles.held(activity, "dialer"))
            return PhoneJson.error("Select Windows Phone as the default phone application before requesting call history access.");
        String[] names = names(permission);
        if (names == null) return PhoneJson.error("Unknown Android runtime permission.");
        ArrayList<String> missing = new ArrayList<>();
        for (String name : names) {
            if (granted(activity, name)) continue;
            boolean asked = activity.getSharedPreferences("phone-host", Context.MODE_PRIVATE)
                    .getBoolean("permission-result:" + name, false);
            if (asked && !activity.shouldShowRequestPermissionRationale(name))
                return PhoneSettings.open(activity, "application");
            missing.add(name);
        }
        if (missing.isEmpty()) return PhoneJson.accepted("The requested Android permission is already granted.");
        activity.requestPermissions(missing.toArray(new String[0]), REQUEST);
        return PhoneJson.accepted("Android opened permission consent. No protected operation is retried automatically.");
    }

    static void result(Activity activity, String[] permissions, int[] results) {
        android.content.SharedPreferences.Editor editor = activity.getSharedPreferences("phone-host", Context.MODE_PRIVATE).edit();
        for (int i = 0; i < permissions.length && i < results.length; i++)
            editor.putBoolean("permission-result:" + permissions[i], true);
        editor.apply();
    }
}
