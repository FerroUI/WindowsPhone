package org.ferroui.phone;

import android.content.Context;
import android.content.Intent;
import android.content.IntentFilter;
import android.net.ConnectivityManager;
import android.net.Network;
import android.net.NetworkCapabilities;
import android.os.BatteryManager;
import android.os.PowerManager;
import org.json.JSONObject;

final class PhoneEnvironment {
    private PhoneEnvironment() {}
    static void observe(Context context, JSONObject state) {
        try {
            Intent battery = context.registerReceiver(null, new IntentFilter(Intent.ACTION_BATTERY_CHANGED));
            if (battery != null) {
                int level = battery.getIntExtra(BatteryManager.EXTRA_LEVEL, -1);
                int scale = battery.getIntExtra(BatteryManager.EXTRA_SCALE, -1);
                if (level >= 0 && scale > 0) PhoneJson.put(state, "battery_percent", Math.min(100, (int)(100L * level / scale)));
                int status = battery.getIntExtra(BatteryManager.EXTRA_STATUS, BatteryManager.BATTERY_STATUS_UNKNOWN);
                if (status != BatteryManager.BATTERY_STATUS_UNKNOWN)
                    PhoneJson.put(state, "charging", status == BatteryManager.BATTERY_STATUS_CHARGING
                            || status == BatteryManager.BATTERY_STATUS_FULL);
                int plugged = battery.getIntExtra(BatteryManager.EXTRA_PLUGGED, -1);
                if (plugged >= 0) PhoneJson.put(state, "external_power", plugged != 0);
            }
        } catch (RuntimeException unavailable) { /* Keep unavailable facts absent. */ }
        try {
            PowerManager power = context.getSystemService(PowerManager.class);
            if (power != null) PhoneJson.put(state, "power_saver", power.isPowerSaveMode());
        } catch (RuntimeException unavailable) {}
        try {
            ConnectivityManager connectivity = context.getSystemService(ConnectivityManager.class);
            if (connectivity != null) {
                Network active = connectivity.getActiveNetwork();
                NetworkCapabilities capabilities = active == null ? null : connectivity.getNetworkCapabilities(active);
                PhoneJson.put(state, "online", capabilities != null
                        && capabilities.hasCapability(NetworkCapabilities.NET_CAPABILITY_INTERNET)
                        && capabilities.hasCapability(NetworkCapabilities.NET_CAPABILITY_VALIDATED));
            }
        } catch (RuntimeException unavailable) {}
    }
}
