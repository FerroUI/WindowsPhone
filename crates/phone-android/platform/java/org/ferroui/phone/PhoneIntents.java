package org.ferroui.phone;

import android.app.Activity;
import android.content.Context;
import android.content.Intent;
import android.net.Uri;
import android.util.Base64;
import java.nio.charset.StandardCharsets;
import java.util.HashSet;
import java.util.UUID;
import org.json.JSONArray;
import org.json.JSONObject;

/** Platform intents populate transient drafts or validated routes; they never trigger a send or call. */
final class PhoneIntents {
    private PhoneIntents() {}
    static String encode(String text) { return Base64.encodeToString(text.getBytes(StandardCharsets.UTF_8), Base64.URL_SAFE | Base64.NO_WRAP | Base64.NO_PADDING); }
    static Uri app(String id, String path) {
        return new Uri.Builder().scheme("ferroui-phone").authority("app").appendPath(id).appendQueryParameter("path", path).build();
    }
    static JSONArray recipients(Uri uri) {
        JSONArray result = new JSONArray();
        if (uri == null || !java.util.Arrays.asList("sms", "smsto", "mms", "mmsto").contains(uri.getScheme())) return result;
        String value = uri.getSchemeSpecificPart();
        if (value == null || value.length() > 4096) return result;
        int query = value.indexOf('?'); if (query >= 0) value = value.substring(0, query);
        HashSet<String> seen = new HashSet<>();
        for (String part : value.split("[;,]")) {
            if (part.trim().isEmpty()) continue;
            String number = PhoneNumbers.messageAddress(part);
            if (number == null || result.length() >= 20) return new JSONArray();
            if (seen.add(number)) result.put(number);
        }
        return result;
    }
    static Intent activation(Intent intent) {
        if (intent == null) return null;
        if (Intent.ACTION_MAIN.equals(intent.getAction()) && intent.hasCategory(Intent.CATEGORY_HOME)) {
            return new Intent(intent).setData(Uri.parse("ferroui-phone://home"));
        }
        Uri data = intent.getData();
        if (Intent.ACTION_DIAL.equals(intent.getAction()) || Intent.ACTION_VIEW.equals(intent.getAction()) && data != null && "tel".equals(data.getScheme())) {
            String number = data == null ? "" : data.getSchemeSpecificPart();
            String path = PhoneNumbers.valid(number) ? "dial/" + encode(number) : "calls";
            return new Intent(intent).setAction(Intent.ACTION_VIEW).setData(app("phone.phone", path));
        }
        if (Intent.ACTION_SENDTO.equals(intent.getAction()) && data != null
                && java.util.Arrays.asList("sms", "smsto", "mms", "mmsto").contains(data.getScheme())) {
            JSONArray recipients = recipients(data);
            CharSequence extra = intent.getCharSequenceExtra("sms_body");
            if (extra == null) extra = intent.getCharSequenceExtra(Intent.EXTRA_TEXT);
            String body = extra == null ? "" : extra.toString();
            if (body.isEmpty()) {
                String encoded = data.getEncodedSchemeSpecificPart(); int at = encoded == null ? -1 : encoded.indexOf('?');
                if (at >= 0 && encoded.length() <= 20000) body = Uri.parse("https://draft.invalid/?" + encoded.substring(at + 1)).getQueryParameter("body");
            }
            if (body == null) body = "";
            if (body.getBytes(StandardCharsets.UTF_8).length > 16000) body = "";
            JSONObject draft = PhoneJson.put(new JSONObject(), "id", "android-compose:" + UUID.randomUUID());
            PhoneJson.put(draft, "recipients", recipients); PhoneJson.put(draft, "body", PhoneJson.text(body, 16000, true));
            PhoneDevice.event("composeDraft", draft);
            return new Intent(intent).setAction(Intent.ACTION_VIEW).setData(app("phone.messaging", "compose"));
        }
        return intent;
    }
    static JSONObject compose(Activity activity, JSONObject request) {
        JSONArray recipients = request.optJSONArray("recipients");
        StringBuilder to = new StringBuilder();
        if (recipients != null) for (int index = 0; index < recipients.length(); index++) {
            if (index > 0) to.append(';'); to.append(recipients.optString(index));
        }
        Intent intent = new Intent(Intent.ACTION_SENDTO, Uri.fromParts("smsto", to.toString(), null))
                .putExtra("sms_body", request.optString("body")).putExtra(Intent.EXTRA_TEXT, request.optString("body"));
        // As default SMS app, retain the draft in this app instead of recursively opening ourselves.
        if (PhoneRoles.held(activity, "sms")) {
            activity.startActivity(new Intent(activation(intent)).setComponent(activity.getComponentName()));
            return PhoneJson.deviceAccepted("The editable message draft is open. It has not been sent.");
        }
        if (!PhoneSettings.resolves(activity, intent)) return PhoneJson.error("No Android SMS composer is available.");
        activity.startActivity(intent);
        return PhoneJson.deviceAccepted("Android opened the SMS composer. Sending and delivery are controlled by that application.");
    }
}
