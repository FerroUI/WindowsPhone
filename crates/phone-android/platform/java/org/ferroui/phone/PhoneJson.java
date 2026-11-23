package org.ferroui.phone;

import org.json.JSONException;
import org.json.JSONObject;
import java.nio.charset.StandardCharsets;

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

    static JSONObject device(String kind, Object value) {
        JSONObject response = put(value(kind), "value", value);
        return put(value("device"), "response", response);
    }

    static JSONObject deviceAccepted(String message) {
        return device("accepted", put(new JSONObject(), "message", text(message, 4096, true)));
    }

    /** Shared Rust bounds are UTF-8 bytes, not UTF-16 units or code points. */
    static String text(CharSequence input, int maximumBytes, boolean multiline) {
        if (input == null) return "";
        String value = input.toString();
        StringBuilder result = new StringBuilder();
        int bytes = 0;
        for (int offset = 0; offset < value.length();) {
            int point = value.codePointAt(offset);
            offset += Character.charCount(point);
            if (point == 0 || (Character.isISOControl(point) && !(multiline && (point == '\n' || point == '\t')))) continue;
            if (point >= 0xd800 && point <= 0xdfff) continue;
            int size = point < 0x80 ? 1 : point < 0x800 ? 2 : point < 0x10000 ? 3 : 4;
            if (bytes + size > maximumBytes) break;
            bytes += size;
            result.appendCodePoint(point);
        }
        return result.toString();
    }

    static int bytes(JSONObject value) { return value.toString().getBytes(StandardCharsets.UTF_8).length; }

    static String bounded(String value, int maxCodePoints) {
        if (value == null) return "";
        String clean = value.replaceAll("[\\p{Cc}&&[^\\n\\t]]", "");
        int count = clean.codePointCount(0, clean.length());
        return count <= maxCodePoints ? clean : clean.substring(0, clean.offsetByCodePoints(0, maxCodePoints));
    }
}
