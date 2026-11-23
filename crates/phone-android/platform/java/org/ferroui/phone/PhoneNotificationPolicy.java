package org.ferroui.phone;

import android.content.Context;
import android.content.SharedPreferences;
import org.json.JSONObject;

/** App-owned message presentation preferences survive activity/process death. Android's user policy remains final. */
final class PhoneNotificationPolicy {
    private static final String[] FIELDS = {"enabled", "message_center", "message_banners", "message_sound", "message_vibration", "lock_screen"};
    private PhoneNotificationPolicy() {}
    static SharedPreferences preferences(Context context) { return context.getSharedPreferences("phone-message-notification-policy", Context.MODE_PRIVATE); }
    static void save(Context context, String json) {
        if (json == null || json.length() > 4096) return;
        try {
            JSONObject value = new JSONObject(json);
            for (String field : FIELDS) if (!(value.opt(field) instanceof Boolean)) return;
            SharedPreferences.Editor editor = preferences(context).edit();
            for (String field : FIELDS) editor.putBoolean(field, value.getBoolean(field));
            editor.apply();
        } catch (org.json.JSONException invalid) { }
    }
}
