package org.ferroui.phone;

import android.Manifest;
import android.app.Activity;
import android.app.NotificationManager;
import android.bluetooth.BluetoothAdapter;
import android.bluetooth.BluetoothManager;
import android.content.ContentResolver;
import android.content.Context;
import android.content.Intent;
import android.location.LocationManager;
import android.media.AudioManager;
import android.media.MediaRouter;
import android.net.ConnectivityManager;
import android.net.NetworkCapabilities;
import android.net.wifi.WifiManager;
import android.nfc.NfcAdapter;
import android.os.Build;
import android.os.PowerManager;
import android.provider.Settings;
import android.telephony.TelephonyManager;
import org.json.JSONArray;
import org.json.JSONObject;

/** Values are observations. A system panel or permission dialog never supplies an optimistic value. */
final class PhoneHardware {
    static final String[] CONTROLS = {"wifi", "bluetooth", "cellular", "airplane", "location", "rotation",
            "brightness", "autoBrightness", "flashlight", "ringerVolume", "mediaVolume", "alarmVolume",
            "ringerMode", "doNotDisturb", "batterySaver", "hotspot", "nfc", "screenTimeout", "nightLight",
            "cast", "nearbyShare", "vpn", "audioOutput", "sync", "dataSaver"};
    private PhoneHardware() {}

    static JSONObject state(String name, String access, Object observed, String detail) {
        JSONObject value = PhoneJson.value(observed instanceof Boolean ? "toggle"
                : observed instanceof Number ? "level" : observed instanceof String ? "choice" : "unknown");
        if (observed != null) PhoneJson.put(value, "value", observed);
        JSONObject state = PhoneJson.put(new JSONObject(), "control", name);
        PhoneJson.put(state, "access", access);
        PhoneJson.put(state, "value", value);
        return PhoneJson.put(state, "detail", detail);
    }

    static JSONArray observe(Context context) {
        JSONArray result = new JSONArray();
        for (String name : CONTROLS) {
            try { result.put(observe(context, name)); }
            catch (SecurityException denied) { result.put(state(name, "permission", null, "Android permission or device policy restricts this observation.")); }
            catch (Exception unavailable) { result.put(state(name, "unavailable", null, "This control is unavailable on this device.")); }
        }
        return result;
    }

    private static JSONObject observe(Context context, String name) throws Exception {
        ContentResolver resolver = context.getContentResolver();
        String write = Settings.System.canWrite(context) ? "writable" : "permission";
        switch (name) {
            case "wifi": {
                WifiManager wifi = context.getSystemService(WifiManager.class);
                return state(name, wifi == null ? "unavailable" : "panel", wifi == null ? null : wifi.isWifiEnabled(), "Android owns Wi-Fi changes; use its connectivity panel.");
            }
            case "bluetooth": {
                BluetoothManager bluetooth = context.getSystemService(BluetoothManager.class);
                BluetoothAdapter adapter = bluetooth == null ? null : bluetooth.getAdapter();
                if (adapter == null) return state(name, "unavailable", null, "Bluetooth hardware is unavailable.");
                boolean permission = Build.VERSION.SDK_INT < 31 || PhonePermissions.granted(context, Manifest.permission.BLUETOOTH_CONNECT);
                return state(name, permission ? "panel" : "permission", permission ? adapter.isEnabled() : null,
                        "Android requires user consent to enable Bluetooth and owns disabling it.");
            }
            case "cellular": {
                TelephonyManager phone = context.getSystemService(TelephonyManager.class);
                boolean hardware = context.getPackageManager().hasSystemFeature("android.hardware.telephony");
                Object enabled = phone != null && PhonePermissions.granted(context, Manifest.permission.READ_PHONE_STATE)
                        ? phone.isDataEnabled() : null;
                return state(name, hardware ? "panel" : "unavailable", enabled, "Cellular data and SIM selection are controlled in Android settings.");
            }
            case "airplane": return state(name, "panel", Settings.Global.getInt(resolver, Settings.Global.AIRPLANE_MODE_ON, 0) != 0, "Android owns flight-mode changes.");
            case "location": {
                LocationManager location = context.getSystemService(LocationManager.class);
                Object enabled = location == null ? null : Build.VERSION.SDK_INT >= 28 ? location.isLocationEnabled()
                        : Settings.Secure.getInt(resolver, Settings.Secure.LOCATION_MODE, 0) != 0;
                return state(name, location == null ? "unavailable" : "panel", enabled, "Location access permission and the device location switch are separate.");
            }
            case "rotation": return state(name, write, Settings.System.getInt(resolver, Settings.System.ACCELEROMETER_ROTATION, 1) == 0, "Enabled means rotation is locked. Android change-settings access is required to write it.");
            case "brightness": return state(name, write, Math.max(0.0, Math.min(1.0, Settings.System.getInt(resolver, Settings.System.SCREEN_BRIGHTNESS) / 255.0)), "Brightness is a normalized system level; changing it selects manual brightness.");
            case "autoBrightness": return state(name, write, Settings.System.getInt(resolver, Settings.System.SCREEN_BRIGHTNESS_MODE, 0) == Settings.System.SCREEN_BRIGHTNESS_MODE_AUTOMATIC, "Android change-settings access is required.");
            case "screenTimeout": {
                String seconds = Integer.toString(Settings.System.getInt(resolver, Settings.System.SCREEN_OFF_TIMEOUT) / 1000);
                boolean standard = java.util.Arrays.asList("15", "30", "60", "120", "300", "600", "1800").contains(seconds);
                return state(name, standard ? write : "readOnly", seconds, "Screen timeout is expressed in seconds. Device administration can impose a shorter timeout.");
            }
            case "flashlight": return PhoneTorch.observation(context);
            case "ringerVolume": case "mediaVolume": case "alarmVolume": {
                AudioManager audio = context.getSystemService(AudioManager.class);
                if (audio == null) return state(name, "unavailable", null, "Audio management is unavailable.");
                int stream = stream(name), maximum = audio.getStreamMaxVolume(stream);
                int minimum = Build.VERSION.SDK_INT >= 28 ? audio.getStreamMinVolume(stream) : 0;
                double level = maximum <= minimum ? 0 : (audio.getStreamVolume(stream) - minimum) / (double) (maximum - minimum);
                return state(name, audio.isVolumeFixed() ? "readOnly" : "writable", Math.max(0, Math.min(1, level)), "Observed Android stream volume. Quiet-hours policy can restrict ringer changes.");
            }
            case "ringerMode": {
                AudioManager audio = context.getSystemService(AudioManager.class);
                if (audio == null) return state(name, "unavailable", null, "Audio management is unavailable.");
                int mode = audio.getRingerMode();
                return state(name, policyAccess(context) ? "writable" : "permission", mode == AudioManager.RINGER_MODE_NORMAL ? "normal" : mode == AudioManager.RINGER_MODE_VIBRATE ? "vibrate" : "silent", "Quiet-hours policy access is required before changing ringer mode.");
            }
            case "doNotDisturb": {
                NotificationManager manager = context.getSystemService(NotificationManager.class);
                int filter = manager == null ? NotificationManager.INTERRUPTION_FILTER_UNKNOWN : manager.getCurrentInterruptionFilter();
                return state(name, manager == null ? "unavailable" : "panel", filter == NotificationManager.INTERRUPTION_FILTER_UNKNOWN ? null : filter != NotificationManager.INTERRUPTION_FILTER_ALL,
                        "Observed global quiet-hours state. Android 15+ combines system rules; this application opens the rule/settings screen instead of claiming to override other rules.");
            }
            case "batterySaver": {
                PowerManager power = context.getSystemService(PowerManager.class);
                return state(name, power == null ? "unavailable" : "panel", power == null ? null : power.isPowerSaveMode(), "Android owns battery-saver changes.");
            }
            case "nfc": {
                NfcAdapter nfc = NfcAdapter.getDefaultAdapter(context);
                return state(name, nfc == null ? "unavailable" : "panel", nfc == null ? null : nfc.isEnabled(), "Android owns the NFC switch; no payment-card capability is implied.");
            }
            case "sync": return state(name, "writable", ContentResolver.getMasterSyncAutomatically(), "Android account sync master switch. Individual account and background restrictions still apply.");
            case "dataSaver": {
                ConnectivityManager network = context.getSystemService(ConnectivityManager.class);
                int status = network == null ? 0 : network.getRestrictBackgroundStatus();
                return state(name, network == null ? "unavailable" : "panel", status == 0 ? null : status != ConnectivityManager.RESTRICT_BACKGROUND_STATUS_DISABLED, "Android data saver is separate from this application's background-data exemption.");
            }
            case "vpn": {
                ConnectivityManager network = context.getSystemService(ConnectivityManager.class);
                NetworkCapabilities caps = network == null ? null : network.getNetworkCapabilities(network.getActiveNetwork());
                return state(name, "panel", caps == null ? null : caps.hasTransport(NetworkCapabilities.TRANSPORT_VPN) ? "connected" : "not connected", "VPN state describes the active network; Android owns VPN profiles and consent.");
            }
            case "audioOutput": {
                MediaRouter router = (MediaRouter) context.getSystemService(Context.MEDIA_ROUTER_SERVICE);
                MediaRouter.RouteInfo route = router == null ? null : router.getSelectedRoute(MediaRouter.ROUTE_TYPE_LIVE_AUDIO);
                return state(name, "panel", route == null ? null : PhoneJson.text(route.getName(), 512, false), "Observed Android audio route; choose devices through Android sound/Bluetooth settings.");
            }
            default: return state(name, PhoneSettings.resolves(context, PhoneSettings.intent(context, name)) ? "panel" : "unavailable", null, "Android or the device vendor owns this setting; no undocumented state is inferred.");
        }
    }

    static boolean policyAccess(Context context) {
        NotificationManager manager = context.getSystemService(NotificationManager.class);
        return manager != null && manager.isNotificationPolicyAccessGranted();
    }

    private static int stream(String name) {
        return "ringerVolume".equals(name) ? AudioManager.STREAM_RING : "alarmVolume".equals(name) ? AudioManager.STREAM_ALARM : AudioManager.STREAM_MUSIC;
    }

    static JSONObject open(Activity activity, String name) {
        switch (name) {
            case "flashlight": return PhonePermissions.request(activity, "camera");
            case "autoBrightness": return PhoneSettings.open(activity, "brightness");
            case "ringerVolume": case "mediaVolume": case "alarmVolume": case "ringerMode": return PhoneSettings.open(activity, "volume");
            default: return PhoneSettings.open(activity, name);
        }
    }

    static JSONObject set(Activity activity, String name, JSONObject value) {
        if (value == null) return PhoneJson.error("A typed control value is required.");
        String kind = value.optString("kind");
        boolean toggle = value.optBoolean("value");
        try {
            switch (name) {
                case "flashlight":
                    if (!"toggle".equals(kind)) break;
                    return PhoneTorch.set(activity, toggle);
                case "brightness": case "autoBrightness": case "rotation": case "screenTimeout": {
                    if (!Settings.System.canWrite(activity)) return PhoneJson.error("Grant Android change-settings access before changing this control.");
                    ContentResolver resolver = activity.getContentResolver();
                    boolean saved;
                    if ("brightness".equals(name) && "level".equals(kind)) {
                        double level = value.optDouble("value", Double.NaN);
                        if (!Double.isFinite(level) || level < 0 || level > 1) break;
                        saved = Settings.System.putInt(resolver, Settings.System.SCREEN_BRIGHTNESS_MODE, Settings.System.SCREEN_BRIGHTNESS_MODE_MANUAL)
                                && Settings.System.putInt(resolver, Settings.System.SCREEN_BRIGHTNESS, Math.max(1, (int) Math.round(level * 255)));
                    } else if ("autoBrightness".equals(name) && "toggle".equals(kind)) {
                        saved = Settings.System.putInt(resolver, Settings.System.SCREEN_BRIGHTNESS_MODE, toggle ? Settings.System.SCREEN_BRIGHTNESS_MODE_AUTOMATIC : Settings.System.SCREEN_BRIGHTNESS_MODE_MANUAL);
                    } else if ("rotation".equals(name) && "toggle".equals(kind)) {
                        saved = Settings.System.putInt(resolver, Settings.System.ACCELEROMETER_ROTATION, toggle ? 0 : 1);
                    } else if ("screenTimeout".equals(name) && "choice".equals(kind)) {
                        String seconds = value.optString("value");
                        if (!java.util.Arrays.asList("15", "30", "60", "120", "300", "600", "1800").contains(seconds)) break;
                        saved = Settings.System.putInt(resolver, Settings.System.SCREEN_OFF_TIMEOUT, Integer.parseInt(seconds) * 1000);
                    } else break;
                    return saved ? PhoneJson.deviceAccepted("Android saved the system setting; the next observation reads its effective value.") : PhoneJson.error("Android did not save the system setting.");
                }
                case "ringerVolume": case "mediaVolume": case "alarmVolume": {
                    double level = value.optDouble("value", Double.NaN);
                    if (!"level".equals(kind) || !Double.isFinite(level) || level < 0 || level > 1) break;
                    AudioManager audio = activity.getSystemService(AudioManager.class);
                    if (audio == null || audio.isVolumeFixed()) return PhoneJson.error("This device does not allow application volume changes.");
                    int stream = stream(name), maximum = audio.getStreamMaxVolume(stream);
                    int minimum = Build.VERSION.SDK_INT >= 28 ? audio.getStreamMinVolume(stream) : 0;
                    audio.setStreamVolume(stream, minimum + (int) Math.round(level * (maximum - minimum)), AudioManager.FLAG_SHOW_UI);
                    return PhoneJson.deviceAccepted("Android accepted the stream volume request.");
                }
                case "ringerMode": {
                    if (!"choice".equals(kind)) break;
                    if (!policyAccess(activity)) return PhoneJson.error("Android quiet-hours policy access is required to change ringer mode.");
                    String mode = value.optString("value");
                    int selected = "normal".equals(mode) ? AudioManager.RINGER_MODE_NORMAL : "vibrate".equals(mode) ? AudioManager.RINGER_MODE_VIBRATE : "silent".equals(mode) ? AudioManager.RINGER_MODE_SILENT : -1;
                    if (selected < 0) break;
                    AudioManager audio = activity.getSystemService(AudioManager.class);
                    if (audio == null) return PhoneJson.error("Audio management is unavailable.");
                    audio.setRingerMode(selected);
                    return PhoneJson.deviceAccepted("Android accepted the ringer-mode request.");
                }
                case "sync":
                    if (!"toggle".equals(kind)) break;
                    ContentResolver.setMasterSyncAutomatically(toggle);
                    return PhoneJson.deviceAccepted("Android accepted the account sync setting.");
                case "bluetooth":
                    if (!"toggle".equals(kind)) break;
                    if (!toggle) return PhoneSettings.open(activity, "bluetooth");
                    if (Build.VERSION.SDK_INT >= 31 && !PhonePermissions.granted(activity, Manifest.permission.BLUETOOTH_CONNECT))
                        return PhoneJson.error("Bluetooth permission is required before opening enable consent.");
                    activity.startActivity(new Intent(BluetoothAdapter.ACTION_REQUEST_ENABLE));
                    return PhoneJson.deviceAccepted("Android opened Bluetooth enable consent; no state change is assumed.");
                default: return open(activity, name);
            }
        } catch (SecurityException restricted) {
            return PhoneJson.error("Android permission or quiet-hours/device policy prevented this change.");
        } catch (Exception unavailable) {
            return PhoneJson.error("Android could not change this control.");
        }
        return PhoneJson.error("The value does not match this Android control.");
    }
}
