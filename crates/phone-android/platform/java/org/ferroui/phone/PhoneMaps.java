package org.ferroui.phone;

import android.app.Activity;
import android.content.Context;
import android.content.Intent;
import android.net.Uri;
import org.json.JSONObject;

final class PhoneMaps {
    private PhoneMaps() {}
    static boolean available(Context context) {
        return PhoneSettings.resolves(context, new Intent(Intent.ACTION_VIEW, Uri.parse("geo:0,0?q=map")))
                || PhoneSettings.resolves(context, new Intent(Intent.ACTION_VIEW, Uri.parse("https://www.google.com/maps/")));
    }
    static JSONObject open(Activity activity, JSONObject request) {
        String destination = request.optString("query");
        if (!request.isNull("latitude") && !request.isNull("longitude")) {
            double latitude = request.optDouble("latitude", Double.NaN), longitude = request.optDouble("longitude", Double.NaN);
            if (!Double.isFinite(latitude) || !Double.isFinite(longitude) || Math.abs(latitude) > 90 || Math.abs(longitude) > 180) return PhoneJson.error("Invalid map coordinates.");
            destination = Double.toString(latitude) + "," + Double.toString(longitude);
        }
        boolean directions = request.optBoolean("destination");
        Uri uri = directions ? new Uri.Builder().scheme("https").authority("www.google.com").path("/maps/dir/")
                .appendQueryParameter("api", "1").appendQueryParameter("destination", destination)
                .appendQueryParameter("travelmode", "cycling".equals(request.optString("mode")) ? "bicycling" : request.optString("mode", "driving")).build()
                : Uri.parse("geo:0,0?q=" + Uri.encode(destination));
        Intent intent = new Intent(Intent.ACTION_VIEW, uri);
        if (!PhoneSettings.resolves(activity, intent) && !directions) {
            intent.setData(new Uri.Builder().scheme("https").authority("www.google.com").path("/maps/search/")
                    .appendQueryParameter("api", "1").appendQueryParameter("query", destination).build());
        }
        if (!PhoneSettings.resolves(activity, intent)) return PhoneJson.error("No map application or browser is available.");
        activity.startActivity(intent);
        return PhoneJson.deviceAccepted("Android opened the map destination. The selected map application owns navigation and its location consent.");
    }
}
