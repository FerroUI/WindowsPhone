package org.ferroui.phone;

import android.app.Activity;
import android.app.role.RoleManager;
import android.content.ComponentName;
import android.content.Context;
import android.content.Intent;
import android.content.pm.ActivityInfo;
import android.content.pm.PackageManager;
import android.content.pm.ResolveInfo;
import android.os.Build;
import android.provider.Settings;
import java.util.ArrayList;
import java.util.Comparator;
import java.util.HashSet;
import java.nio.charset.StandardCharsets;
import org.json.JSONArray;
import org.json.JSONObject;

final class PhoneLauncher {
    private static final int MAX_APPLICATIONS = 1024;
    private PhoneLauncher() {}

    static boolean available(Context context) {
        if (!BuildConfig.PHONE_LAUNCHER) return false;
        if (Build.VERSION.SDK_INT < 29) return true;
        RoleManager roles = context.getSystemService(RoleManager.class);
        return roles != null && roles.isRoleAvailable(RoleManager.ROLE_HOME);
    }

    static boolean held(Context context) {
        if (!BuildConfig.PHONE_LAUNCHER) return false;
        if (Build.VERSION.SDK_INT >= 29) {
            RoleManager roles = context.getSystemService(RoleManager.class);
            return roles != null && roles.isRoleAvailable(RoleManager.ROLE_HOME)
                    && roles.isRoleHeld(RoleManager.ROLE_HOME);
        }
        Intent home = new Intent(Intent.ACTION_MAIN).addCategory(Intent.CATEGORY_HOME);
        ResolveInfo resolved = context.getPackageManager().resolveActivity(home, PackageManager.MATCH_DEFAULT_ONLY);
        return resolved != null && resolved.activityInfo != null
                && context.getPackageName().equals(resolved.activityInfo.packageName);
    }

    static JSONObject request(Activity activity) {
        if (!available(activity)) return PhoneJson.error("This package is not a HOME launcher, or the device does not offer that role.");
        if (held(activity)) return PhoneJson.accepted("Windows Phone is already the default home app.");
        Intent request;
        if (Build.VERSION.SDK_INT >= 29) {
            request = activity.getSystemService(RoleManager.class).createRequestRoleIntent(RoleManager.ROLE_HOME);
        } else { request = new Intent(Settings.ACTION_HOME_SETTINGS); }
        activity.startActivity(request);
        return PhoneJson.accepted("Android opened the home-app choice. Your selection is checked when you return.");
    }

    private static boolean admitted(Context context, ActivityInfo activity) {
        return activity != null && activity.enabled && activity.exported
                && activity.applicationInfo != null && activity.applicationInfo.enabled
                && !context.getPackageName().equals(activity.packageName)
                && (activity.permission == null
                    || context.checkSelfPermission(activity.permission) == PackageManager.PERMISSION_GRANTED);
    }

    static JSONObject applications(Context context) {
        PackageManager manager = context.getPackageManager();
        Intent launcher = new Intent(Intent.ACTION_MAIN).addCategory(Intent.CATEGORY_LAUNCHER);
        ArrayList<JSONObject> rows = new ArrayList<>();
        HashSet<String> seen = new HashSet<>();
        int bytes = 0;
        for (ResolveInfo resolved : manager.queryIntentActivities(launcher, 0)) {
            if (Thread.currentThread().isInterrupted()) return PhoneJson.error("Application discovery was cancelled.");
            ActivityInfo activity = resolved.activityInfo;
            if (!admitted(context, activity)) continue;
            String id = new ComponentName(activity.packageName, activity.name).flattenToString();
            if (id.length() > 512 || id.getBytes(StandardCharsets.UTF_8).length > 1024 || !seen.add(id)) continue;
            CharSequence label = resolved.loadLabel(manager);
            String title = PhoneJson.bounded(label == null ? activity.packageName
                    : label.toString().replaceAll("[\\p{Cc}\\p{Cf}]", " "), 128).trim();
            if (title.isEmpty()) title = activity.packageName;
            JSONObject row = PhoneJson.put(PhoneJson.put(new JSONObject(), "id", id), "title", title);
            int size = row.toString().getBytes(StandardCharsets.UTF_8).length + 1;
            if (bytes + size > 900000) break;
            rows.add(row);
            bytes += size;
            if (rows.size() == MAX_APPLICATIONS) break;
        }
        rows.sort(Comparator.comparing(row -> row.optString("title"), String.CASE_INSENSITIVE_ORDER));
        return PhoneJson.put(PhoneJson.value("applications"), "applications", new JSONArray(rows));
    }

    static JSONObject launch(Context context, String id) {
        if (id == null || id.length() > 512) return PhoneJson.error("Invalid Android application identity.");
        ComponentName selected = ComponentName.unflattenFromString(id);
        if (selected == null) return PhoneJson.error("Invalid Android application identity.");
        Intent lookup = new Intent(Intent.ACTION_MAIN).addCategory(Intent.CATEGORY_LAUNCHER)
                .setPackage(selected.getPackageName());
        // Re-query launcher-filter membership instead of trusting a saved arbitrary component.
        for (ResolveInfo resolved : context.getPackageManager().queryIntentActivities(lookup, 0)) {
            ActivityInfo activity = resolved.activityInfo;
            if (admitted(context, activity) && selected.equals(new ComponentName(activity.packageName, activity.name))) {
                context.startActivity(Intent.makeMainActivity(selected)
                        .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK | Intent.FLAG_ACTIVITY_RESET_TASK_IF_NEEDED));
                return PhoneJson.accepted("Android accepted the application launch request.");
            }
        }
        return PhoneJson.error("That launchable application is no longer available in this profile.");
    }
}
