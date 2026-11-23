package org.ferroui.phone;

import org.json.JSONException;
import org.json.JSONObject;

final class PhoneJson {
    private PhoneJson() {}

    static JSONObject value(String kind) {
        return put(new JSONObject(), "kind", kind);
    }

    static JSONObject put(JSONObject object, String key, Object value) {
        try { return object.put(key, value); }
        catch (JSONException impossible) { throw new IllegalStateException(impossible); }
    }

    static JSONObject accepted(String message) {
        return put(value("accepted"), "message", message);
    }

    static JSONObject error(String message) {
        return put(value("error"), "message", message);
    }

    static String bounded(String value, int maxCodePoints) {
        if (value == null) return "";
        String clean = value.replaceAll("[\\p{Cc}&&[^\\n\\t]]", "");
        int count = clean.codePointCount(0, clean.length());
        return count <= maxCodePoints ? clean : clean.substring(0, clean.offsetByCodePoints(0, maxCodePoints));
    }
}
