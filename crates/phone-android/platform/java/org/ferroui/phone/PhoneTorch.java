package org.ferroui.phone;

import android.Manifest;
import android.content.Context;
import android.hardware.camera2.CameraCharacteristics;
import android.hardware.camera2.CameraManager;
import org.json.JSONObject;

/** A foreground-owned torch lease. Callbacks, not a requested value, determine the observed state. */
final class PhoneTorch {
    private static CameraManager manager;
    private static String camera;
    private static Boolean enabled;
    private static boolean available;
    private static boolean owned;
    private static final CameraManager.TorchCallback callback = new CameraManager.TorchCallback() {
        @Override public void onTorchModeChanged(String id, boolean value) {
            if (!id.equals(camera)) return;
            enabled = value;
            available = true;
            if (!value) owned = false;
        }
        @Override public void onTorchModeUnavailable(String id) {
            if (!id.equals(camera)) return;
            enabled = null;
            available = false;
            owned = false;
        }
    };
    private PhoneTorch() {}

    private static void attach(Context context) {
        if (manager != null) return;
        if (!PhonePermissions.granted(context, Manifest.permission.CAMERA)) return;
        CameraManager candidate = context.getSystemService(CameraManager.class);
        if (candidate == null) return;
        try {
            for (String id : candidate.getCameraIdList()) {
                CameraCharacteristics info = candidate.getCameraCharacteristics(id);
                if (!Boolean.TRUE.equals(info.get(CameraCharacteristics.FLASH_INFO_AVAILABLE))) continue;
                camera = id;
                if (Integer.valueOf(CameraCharacteristics.LENS_FACING_BACK).equals(info.get(CameraCharacteristics.LENS_FACING))) break;
            }
            if (camera == null) return;
            candidate.registerTorchCallback(callback, PhoneTasks.MAIN);
            manager = candidate;
        } catch (Exception unavailable) { camera = null; }
    }

    static JSONObject observation(Context context) {
        boolean hardware = context.getPackageManager().hasSystemFeature("android.hardware.camera.flash");
        if (!hardware) return PhoneHardware.state("flashlight", "unavailable", null, "This device has no camera flash.");
        if (!PhonePermissions.granted(context, Manifest.permission.CAMERA))
            return PhoneHardware.state("flashlight", "permission", null, "Camera permission is required to control the flash.");
        attach(context);
        return PhoneHardware.state("flashlight", manager != null && available ? "writable" : "readOnly", enabled,
                "The observed torch state comes from Android camera callbacks. It is released when this host closes.");
    }

    static JSONObject set(Context context, boolean value) {
        if (!PhonePermissions.granted(context, Manifest.permission.CAMERA))
            return PhoneJson.error("Camera permission is required to control the flashlight.");
        attach(context);
        if (manager == null || camera == null || !available)
            return PhoneJson.error("The flashlight is unavailable or another application is using the camera.");
        try {
            manager.setTorchMode(camera, value);
            owned = value;
            return PhoneJson.deviceAccepted("Android accepted the flashlight request. Its callback supplies the observed result.");
        } catch (Exception unavailable) {
            return PhoneJson.error("Android could not change the flashlight state.");
        }
    }

    static void close() {
        if (manager == null) return;
        if (owned && camera != null) {
            try { manager.setTorchMode(camera, false); } catch (Exception ignored) { }
        }
        manager.unregisterTorchCallback(callback);
        manager = null;
        camera = null;
        enabled = null;
        available = owned = false;
    }
}
