package org.ferroui.phone;

import android.Manifest;
import android.app.Activity;
import android.content.Context;
import android.content.pm.PackageManager;
import android.location.Location;
import android.location.LocationListener;
import android.location.LocationManager;
import android.os.Bundle;
import android.os.Looper;
import java.lang.ref.WeakReference;
import java.util.HashMap;
import org.json.JSONObject;

/** Explicit, permission-gated one-shot location; no IP-derived or fabricated fallback. */
final class PhoneLocation {
    // The pinned FerroUI allocator starts at 20000; this belongs to our activity.
    static final int PERMISSION_REQUEST = 0x1002;
    private static final HashMap<Long, Query> queries = new HashMap<>();
    private static long permissionOwner;

    private static final class Query implements LocationListener {
        final long id;
        final WeakReference<Activity> activity;
        LocationManager manager;
        Runnable timeout;
        Query(long id, Activity activity) { this.id = id; this.activity = new WeakReference<>(activity); }
        @Override public void onLocationChanged(Location location) {
            if (location == null || !Double.isFinite(location.getLatitude()) || !Double.isFinite(location.getLongitude())
                    || Math.abs(location.getLatitude()) > 90 || Math.abs(location.getLongitude()) > 180) {
                finish(PhoneJson.error("Android returned an invalid location.")); return;
            }
            long age = android.os.SystemClock.elapsedRealtimeNanos() - location.getElapsedRealtimeNanos();
            if (age < 0 || age > 120000000000L) {
                finish(PhoneJson.error("The location observation is stale. Please retry.")); return;
            }
            JSONObject result = PhoneJson.put(PhoneJson.value("location"), "latitude", location.getLatitude());
            finish(PhoneJson.put(result, "longitude", location.getLongitude()));
        }
        void finish(JSONObject result) { PhoneTasks.complete(id, result); stop(); }
        void stop() {
            queries.remove(id);
            if (permissionOwner == id) permissionOwner = 0;
            if (timeout != null) { PhoneTasks.MAIN.removeCallbacks(timeout); timeout = null; }
            if (manager != null) {
                try { manager.removeUpdates(this); } catch (RuntimeException ignored) {}
                manager = null;
            }
        }
        @Override public void onProviderDisabled(String provider) { finish(PhoneJson.error("The location provider was disabled.")); }
        @Override public void onProviderEnabled(String provider) {}
        @Override public void onStatusChanged(String provider, int status, Bundle extras) {}
    }

    static long begin(Activity activity) {
        long id = PhoneTasks.create();
        if (id == 0) return 0;
        Query query = new Query(id, activity);
        queries.put(id, query);
        PhoneTasks.cancellation(id, query::stop);
        if (!allowed(activity)) {
            if (permissionOwner != 0) { query.finish(PhoneJson.error("Another location consent request is already open.")); return id; }
            permissionOwner = id;
            try {
                activity.requestPermissions(new String[]{Manifest.permission.ACCESS_COARSE_LOCATION,
                        Manifest.permission.ACCESS_FINE_LOCATION}, PERMISSION_REQUEST);
            } catch (RuntimeException error) { query.finish(PhoneJson.error("Android could not open location consent.")); }
        } else { locate(query); }
        return id;
    }

    private static boolean allowed(Activity activity) {
        return activity.checkSelfPermission(Manifest.permission.ACCESS_COARSE_LOCATION) == PackageManager.PERMISSION_GRANTED
                || activity.checkSelfPermission(Manifest.permission.ACCESS_FINE_LOCATION) == PackageManager.PERMISSION_GRANTED;
    }

    static void permissionResult(Activity activity) {
        Query query = queries.get(permissionOwner);
        permissionOwner = 0;
        if (query == null || query.activity.get() != activity) return;
        if (!allowed(activity)) { query.finish(PhoneJson.error("Location permission was not granted.")); return; }
        locate(query);
    }

    @SuppressWarnings("deprecation")
    private static void locate(Query query) {
        Activity activity = query.activity.get();
        if (activity == null || activity.isDestroyed()) { query.finish(PhoneJson.error("The Android activity closed.")); return; }
        try {
            LocationManager manager = activity.getSystemService(LocationManager.class);
            if (manager == null) { query.finish(PhoneJson.error("No Android location service is available.")); return; }
            String provider = null;
            if (manager.isProviderEnabled(LocationManager.NETWORK_PROVIDER)) provider = LocationManager.NETWORK_PROVIDER;
            else if (activity.checkSelfPermission(Manifest.permission.ACCESS_FINE_LOCATION) == PackageManager.PERMISSION_GRANTED
                    && manager.isProviderEnabled(LocationManager.GPS_PROVIDER)) provider = LocationManager.GPS_PROVIDER;
            if (provider == null) { query.finish(PhoneJson.error("Enable a permitted location provider in Android settings.")); return; }
            query.manager = manager;
            manager.requestSingleUpdate(provider, query, Looper.getMainLooper());
            query.timeout = () -> {
                if (queries.containsKey(query.id)) query.finish(PhoneJson.error("Android location timed out after 15 seconds."));
            };
            PhoneTasks.MAIN.postDelayed(query.timeout, 15000);
        } catch (RuntimeException error) { query.finish(PhoneJson.error("Android could not start the location request.")); }
    }

    static void activityDestroyed(Activity activity) {
        for (Query query : queries.values().toArray(new Query[0])) {
            if (query.activity.get() == activity) query.finish(PhoneJson.error("The Android activity closed during location acquisition."));
        }
    }

    static void close() {
        for (Query query : queries.values().toArray(new Query[0])) query.stop();
        permissionOwner = 0;
    }
}
