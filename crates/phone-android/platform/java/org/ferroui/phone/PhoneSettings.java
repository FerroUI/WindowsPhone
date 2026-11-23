package org.ferroui.phone;

import android.app.Activity;
import android.content.Context;
import android.content.Intent;
import android.net.Uri;
import android.os.Build;
import android.provider.Settings;
import android.telecom.TelecomManager;
import org.json.JSONArray;
import org.json.JSONObject;

/** Public Android settings surfaces, advertised only when this profile can resolve them. */
final class PhoneSettings {
    static final String[] PAGES = {"application", "network", "notifications", "defaultApps", "wifi", "bluetooth",
            "cellular", "airplane", "hotspot", "vpn", "nfc", "location", "display", "brightness", "rotation", "screenTimeout",
            "sound", "audioOutput", "doNotDisturb", "battery", "batterySaver", "storage", "security", "biometrics", "accounts",
            "sync", "dateTime", "language", "keyboard", "accessibility", "privacy", "permissions", "notificationAccess",
            "writeSettings", "phone", "messaging", "dataUsage", "dataSaver", "cast", "nearbyShare", "nightLight", "systemUpdate", "about"};
    private PhoneSettings() {}

    static Intent intent(Context context, String page) {
        String action;
        switch (page) {
            case "network": action = Build.VERSION.SDK_INT >= 29 ? Settings.Panel.ACTION_INTERNET_CONNECTIVITY : Settings.ACTION_WIRELESS_SETTINGS; break;
            case "wifi": action = Build.VERSION.SDK_INT >= 29 ? Settings.Panel.ACTION_WIFI : Settings.ACTION_WIFI_SETTINGS; break;
            case "bluetooth": action = Settings.ACTION_BLUETOOTH_SETTINGS; break;
            case "cellular": case "mobileNetwork": action = Settings.ACTION_DATA_ROAMING_SETTINGS; break;
            case "airplane": action = Settings.ACTION_AIRPLANE_MODE_SETTINGS; break;
            // Android has no public dedicated hotspot settings contract. The public wireless screen owns it.
            case "hotspot": action = Settings.ACTION_WIRELESS_SETTINGS; break;
            case "location": action = Settings.ACTION_LOCATION_SOURCE_SETTINGS; break;
            case "display": case "brightness": case "rotation": case "screenTimeout": action = Settings.ACTION_DISPLAY_SETTINGS; break;
            case "sound": action = Settings.ACTION_SOUND_SETTINGS; break;
            case "volume": case "audioOutput": action = Build.VERSION.SDK_INT >= 29 ? Settings.Panel.ACTION_VOLUME : Settings.ACTION_SOUND_SETTINGS; break;
            case "battery": case "batterySaver": action = Settings.ACTION_BATTERY_SAVER_SETTINGS; break;
            case "nfc": action = Build.VERSION.SDK_INT >= 29 ? Settings.Panel.ACTION_NFC : Settings.ACTION_NFC_SETTINGS; break;
            case "storage": action = Settings.ACTION_INTERNAL_STORAGE_SETTINGS; break;
            case "security": action = Settings.ACTION_SECURITY_SETTINGS; break;
            case "biometrics": action = Build.VERSION.SDK_INT >= 30 ? Settings.ACTION_BIOMETRIC_ENROLL : Settings.ACTION_SECURITY_SETTINGS; break;
            case "privacy": action = Settings.ACTION_PRIVACY_SETTINGS; break;
            case "accessibility": action = Settings.ACTION_ACCESSIBILITY_SETTINGS; break;
            case "accounts": case "sync": action = Settings.ACTION_SYNC_SETTINGS; break;
            case "language": action = Settings.ACTION_LOCALE_SETTINGS; break;
            case "dateTime": action = Settings.ACTION_DATE_SETTINGS; break;
            case "keyboard": action = Settings.ACTION_INPUT_METHOD_SETTINGS; break;
            case "vpn": action = Settings.ACTION_VPN_SETTINGS; break;
            case "notifications": return new Intent(Settings.ACTION_APP_NOTIFICATION_SETTINGS).putExtra(Settings.EXTRA_APP_PACKAGE, context.getPackageName());
            case "notificationAccess": action = Settings.ACTION_NOTIFICATION_LISTENER_SETTINGS; break;
            case "doNotDisturb": action = Settings.ACTION_SOUND_SETTINGS; break;
            case "notificationPolicy": action = Settings.ACTION_NOTIFICATION_POLICY_ACCESS_SETTINGS; break;
            case "writeSettings": return new Intent(Settings.ACTION_MANAGE_WRITE_SETTINGS, Uri.fromParts("package", context.getPackageName(), null));
            case "defaultApps": case "messaging": action = Settings.ACTION_MANAGE_DEFAULT_APPS_SETTINGS; break;
            case "home": action = Settings.ACTION_HOME_SETTINGS; break;
            case "phone": action = TelecomManager.ACTION_SHOW_CALL_SETTINGS; break;
            case "dataUsage": action = Settings.ACTION_DATA_USAGE_SETTINGS; break;
            case "dataSaver": return new Intent(Settings.ACTION_IGNORE_BACKGROUND_DATA_RESTRICTIONS_SETTINGS, Uri.fromParts("package", context.getPackageName(), null));
            case "cast": action = Settings.ACTION_CAST_SETTINGS; break;
            case "nightLight": action = Settings.ACTION_NIGHT_DISPLAY_SETTINGS; break;
            case "systemUpdate": return null; // No public dedicated system-update settings action.
            case "about": action = Settings.ACTION_DEVICE_INFO_SETTINGS; break;
            case "application": case "permissions": return new Intent(Settings.ACTION_APPLICATION_DETAILS_SETTINGS, Uri.fromParts("package", context.getPackageName(), null));
            // Nearby Share has no portable public Android Settings action.
            default: return null;
        }
        return new Intent(action);
    }

    static boolean resolves(Context context, Intent intent) {
        return intent != null && context.getPackageManager().resolveActivity(intent, android.content.pm.PackageManager.MATCH_DEFAULT_ONLY) != null;
    }
    static JSONArray supported(Context context) {
        JSONArray pages = new JSONArray();
        for (String page : PAGES) if (resolves(context, intent(context, page))) pages.put(page);
        return pages;
    }
    static JSONObject open(Activity activity, String page) {
        Intent intent = intent(activity, page);
        if (!resolves(activity, intent)) return PhoneJson.error("This device does not expose that public Android settings page.");
        try {
            activity.startActivity(intent);
            return PhoneJson.accepted("Android opened settings. Device state is observed after you return.");
        } catch (RuntimeException restricted) { return PhoneJson.error("Android could not open that settings page."); }
    }
}
