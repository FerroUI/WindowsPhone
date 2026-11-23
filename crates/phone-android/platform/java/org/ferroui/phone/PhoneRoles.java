package org.ferroui.phone;

import android.app.Activity;
import android.app.role.RoleManager;
import android.content.Context;
import android.content.Intent;
import android.content.pm.PackageManager;
import android.os.Build;
import android.provider.Telephony;
import android.telecom.TelecomManager;
import org.json.JSONObject;

/** Role selection is always owned by Android and initiated by an explicit shell command. */
final class PhoneRoles {
    private PhoneRoles() {}

    private static String androidRole(String role) {
        switch (role) {
            case "home": return "android.app.role.HOME";
            case "dialer": return "android.app.role.DIALER";
            case "sms": return "android.app.role.SMS";
            default: return null;
        }
    }

    static boolean available(Context context, String role) {
        if ("home".equals(role)) return PhoneLauncher.available(context);
        if ("notificationListener".equals(role)) return PhoneSettings.resolves(context, PhoneSettings.intent(context, "notificationAccess"));
        if ("writeSettings".equals(role)) return PhoneSettings.resolves(context, PhoneSettings.intent(context, "writeSettings"));
        if ("doNotDisturb".equals(role)) return PhoneSettings.resolves(context, PhoneSettings.intent(context, "notificationPolicy"));
        String name = androidRole(role);
        if (name == null) return false;
        if (Build.VERSION.SDK_INT >= 29) {
            RoleManager manager = context.getSystemService(RoleManager.class);
            return manager != null && manager.isRoleAvailable(name);
        }
        return context.getPackageManager().hasSystemFeature(PackageManager.FEATURE_TELEPHONY);
    }

    static boolean held(Context context, String role) {
        if ("home".equals(role)) return PhoneLauncher.held(context);
        if ("notificationListener".equals(role)) return PhoneNotificationListener.granted(context);
        if ("writeSettings".equals(role)) return android.provider.Settings.System.canWrite(context);
        if ("doNotDisturb".equals(role)) return PhoneHardware.policyAccess(context);
        if (!available(context, role)) return false;
        if (Build.VERSION.SDK_INT >= 29) {
            RoleManager manager = context.getSystemService(RoleManager.class);
            return manager != null && manager.isRoleHeld(androidRole(role));
        }
        if ("sms".equals(role))
            return context.getPackageName().equals(Telephony.Sms.getDefaultSmsPackage(context));
        TelecomManager telecom = context.getSystemService(TelecomManager.class);
        return "dialer".equals(role) && telecom != null
                && context.getPackageName().equals(telecom.getDefaultDialerPackage());
    }

    static String state(Context context, String role) {
        return held(context, role) ? "active" : available(context, role) ? "available" : "unsupported";
    }

    static JSONObject request(Activity activity, String role) {
        if ("home".equals(role)) return PhoneLauncher.request(activity);
        if ("notificationListener".equals(role)) return PhoneSettings.open(activity, "notificationAccess");
        if ("writeSettings".equals(role)) return PhoneSettings.open(activity, "writeSettings");
        if ("doNotDisturb".equals(role)) return PhoneSettings.open(activity, "notificationPolicy");
        if (!available(activity, role)) return PhoneJson.error("This device does not offer that system role.");
        if (held(activity, role)) return PhoneJson.accepted("Android already selected this application for that role.");
        Intent consent;
        if (Build.VERSION.SDK_INT >= 29) {
            consent = activity.getSystemService(RoleManager.class).createRequestRoleIntent(androidRole(role));
        } else if ("dialer".equals(role)) {
            consent = new Intent(TelecomManager.ACTION_CHANGE_DEFAULT_DIALER)
                    .putExtra(TelecomManager.EXTRA_CHANGE_DEFAULT_DIALER_PACKAGE_NAME, activity.getPackageName());
        } else {
            consent = new Intent(Telephony.Sms.Intents.ACTION_CHANGE_DEFAULT)
                    .putExtra(Telephony.Sms.Intents.EXTRA_PACKAGE_NAME, activity.getPackageName());
        }
        activity.startActivity(consent);
        return PhoneJson.accepted("Android opened role consent. The selected role is observed after you return.");
    }
}
