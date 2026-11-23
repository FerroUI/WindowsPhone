package org.ferroui.phone;

import android.content.ComponentName;
import android.content.Context;
import android.content.Intent;
import android.content.pm.LauncherApps;
import android.content.pm.ResolveInfo;
import android.content.pm.ShortcutInfo;
import android.os.Process;
import java.util.ArrayList;
import java.util.List;
import org.json.JSONArray;
import org.json.JSONObject;

/** Shortcuts are scoped to a freshly verified catalog component and the current Android user. */
final class PhoneShortcuts {
    private PhoneShortcuts() {}
    static boolean available(Context context) {
        LauncherApps service = context.getSystemService(LauncherApps.class);
        return PhoneLauncher.held(context) && service != null && service.hasShortcutHostPermission();
    }
    private static ComponentName component(Context context, String application) {
        ComponentName selected = ComponentName.unflattenFromString(application);
        if (selected == null || context.getPackageName().equals(selected.getPackageName())) throw new IllegalArgumentException("Invalid shortcut application.");
        Intent query = new Intent(Intent.ACTION_MAIN).addCategory(Intent.CATEGORY_LAUNCHER).setPackage(selected.getPackageName());
        for (ResolveInfo row : context.getPackageManager().queryIntentActivities(query, 0))
            if (row.activityInfo != null && row.activityInfo.enabled && row.activityInfo.exported && row.activityInfo.applicationInfo.enabled
                    && (row.activityInfo.permission == null || PhonePermissions.granted(context, row.activityInfo.permission))
                    && selected.equals(new ComponentName(row.activityInfo.packageName, row.activityInfo.name))) return selected;
        throw new IllegalArgumentException("That application is no longer launchable in this profile.");
    }
    private static List<ShortcutInfo> query(Context context, String application) {
        if (!available(context)) throw new SecurityException("The active HOME role and Android shortcut-host permission are required.");
        ComponentName component = component(context, application);
        LauncherApps.ShortcutQuery query = new LauncherApps.ShortcutQuery().setPackage(component.getPackageName())
                .setQueryFlags(LauncherApps.ShortcutQuery.FLAG_MATCH_DYNAMIC | LauncherApps.ShortcutQuery.FLAG_MATCH_MANIFEST | LauncherApps.ShortcutQuery.FLAG_MATCH_PINNED);
        List<ShortcutInfo> rows = context.getSystemService(LauncherApps.class).getShortcuts(query, Process.myUserHandle());
        return rows == null ? new ArrayList<>() : rows;
    }
    static JSONObject read(Context context, String application) {
        JSONArray result = new JSONArray();
        for (ShortcutInfo shortcut : query(context, application)) {
            if (shortcut.getId().getBytes(java.nio.charset.StandardCharsets.UTF_8).length > 1024) continue;
            String title = PhoneJson.text(shortcut.getShortLabel(), 512, false); if (title.trim().isEmpty()) continue;
            JSONObject row = PhoneJson.put(new JSONObject(), "id", shortcut.getId()); PhoneJson.put(row, "title", title);
            PhoneJson.put(row, "enabled", shortcut.isEnabled()); PhoneJson.put(row, "pinned", shortcut.isPinned()); result.put(row);
            if (result.length() == 64) break;
        }
        JSONObject value = PhoneJson.put(new JSONObject(), "application", application); PhoneJson.put(value, "shortcuts", result);
        return PhoneJson.device("shortcuts", value);
    }
    static JSONObject launch(Context context, String application, String id) {
        for (ShortcutInfo shortcut : query(context, application)) if (id.equals(shortcut.getId()) && shortcut.isEnabled()) {
            context.getSystemService(LauncherApps.class).startShortcut(shortcut.getPackage(), id, null, null, Process.myUserHandle());
            return PhoneJson.deviceAccepted("Android accepted the current-profile shortcut launch.");
        }
        return PhoneJson.error("That shortcut is disabled or is no longer published by this application.");
    }
}
