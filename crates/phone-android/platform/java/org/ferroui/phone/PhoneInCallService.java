package org.ferroui.phone;

import android.Manifest;
import android.app.Notification;
import android.content.Context;
import android.content.Intent;
import android.content.pm.ServiceInfo;
import android.net.Uri;
import android.os.Build;
import android.os.Bundle;
import android.os.OutcomeReceiver;
import android.telecom.Call;
import android.telecom.CallAudioState;
import android.telecom.CallEndpoint;
import android.telecom.CallEndpointException;
import android.telecom.InCallService;
import android.telecom.PhoneAccountHandle;
import android.telecom.TelecomManager;
import android.telecom.VideoProfile;
import java.lang.ref.WeakReference;
import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.UUID;
import org.json.JSONArray;
import org.json.JSONObject;

/** Telecom owns call lifetime. Closing the shell never hangs up or detaches a live system call. */
public final class PhoneInCallService extends InCallService {
    private static WeakReference<PhoneInCallService> current = new WeakReference<>(null);
    private static boolean dirty = true;
    private final LinkedHashMap<String, Entry> calls = new LinkedHashMap<>();
    private final ArrayList<CallEndpoint> endpoints = new ArrayList<>();
    private final PhoneCallNotifications notifications = new PhoneCallNotifications();
    private boolean muted, speaker, startedForCalls;

    static final class Entry {
        final String id = "call:" + UUID.randomUUID();
        final Call call;
        final boolean incoming;
        Call.Callback callback;
        Runnable stopTone;
        Entry(Call call) { this.call = call; incoming = Build.VERSION.SDK_INT >= 29
                ? call.getDetails().getCallDirection() == Call.Details.DIRECTION_INCOMING
                : state(call) == Call.STATE_RINGING; }
    }

    static PhoneInCallService current() { return current.get(); }
    static boolean takeDirty() { boolean result = dirty; dirty = false; return result; }
    static int state(Call call) { return Build.VERSION.SDK_INT >= 31 ? call.getDetails().getState() : call.getState(); }
    static JSONObject presentation(Context context) {
        PhoneInCallService service = current.get();
        return PhoneCallNotifications.observe(context, service, service == null ? null : service.notifications.error());
    }

    @Override public void onCreate() { super.onCreate(); current = new WeakReference<>(this); dirty = true; }
    @Override public int onStartCommand(Intent intent, int flags, int startId) {
        // The intent grants no call authority. A late start after the final call,
        // or a fresh process without Telecom callbacks, immediately releases it.
        startedForCalls = true;
        changed();
        return START_NOT_STICKY;
    }
    void startCallForeground(Notification notification) {
        if (!startedForCalls) {
            if (startForegroundService(new Intent(this, PhoneInCallService.class)) == null)
                throw new IllegalStateException("Android did not start the call service.");
            startedForCalls = true;
        }
        if (Build.VERSION.SDK_INT >= 29)
            startForeground(PhoneCallNotifications.ID, notification, ServiceInfo.FOREGROUND_SERVICE_TYPE_PHONE_CALL);
        else startForeground(PhoneCallNotifications.ID, notification);
    }
    void stopCallForeground() {
        try { stopForeground(STOP_FOREGROUND_REMOVE); }
        finally {
            if (startedForCalls) { startedForCalls = false; stopSelf(); }
        }
    }
    @Override public void onCallAdded(Call call) {
        super.onCallAdded(call);
        Entry entry = new Entry(call);
        entry.callback = new Call.Callback() {
            @Override public void onStateChanged(Call value, int state) { changed(); }
            @Override public void onDetailsChanged(Call value, Call.Details details) { changed(); }
            @Override public void onParentChanged(Call value, Call parent) { changed(); }
            @Override public void onChildrenChanged(Call value, List<Call> children) { changed(); }
        };
        calls.put(entry.id, entry);
        call.registerCallback(entry.callback, PhoneTasks.MAIN);
        changed();
        if (PhoneActivity.foreground() != null) show(false);
    }

    @Override public void onCallRemoved(Call call) {
        super.onCallRemoved(call);
        for (Entry entry : new ArrayList<>(calls.values())) if (entry.call == call) {
            release(entry);
            calls.remove(entry.id);
        }
        changed();
    }

    @Override public void onBringToForeground(boolean showDialpad) { show(showDialpad); }
    @Override public void onCallAudioStateChanged(CallAudioState audio) {
        if (audio != null) { muted = audio.isMuted(); speaker = audio.getRoute() == CallAudioState.ROUTE_SPEAKER; changed(); }
    }
    @Override public void onMuteStateChanged(boolean value) { muted = value; changed(); }
    @Override public void onCallEndpointChanged(CallEndpoint endpoint) {
        speaker = endpoint != null && endpoint.getEndpointType() == CallEndpoint.TYPE_SPEAKER;
        changed();
    }
    @Override public void onAvailableCallEndpointsChanged(List<CallEndpoint> available) {
        endpoints.clear();
        endpoints.addAll(available);
        changed();
    }
    private void clearCalls() {
        for (Entry entry : calls.values()) release(entry);
        calls.clear();
        endpoints.clear();
        notifications.clear(this);
        dirty = true;
        PhoneCallActivity.changed();
    }
    @Override public boolean onUnbind(Intent intent) {
        // A started lease must not keep this service alive after Telecom detaches.
        // Super still destroys its Phone and owns the Android binding contract.
        clearCalls();
        return super.onUnbind(intent);
    }
    @Override public void onDestroy() {
        clearCalls();
        if (current.get() == this) current.clear();
        super.onDestroy();
    }

    private void release(Entry entry) {
        entry.call.unregisterCallback(entry.callback);
        if (entry.stopTone != null) PhoneTasks.MAIN.removeCallbacks(entry.stopTone);
        entry.call.stopDtmfTone();
    }

    private void changed() {
        dirty = true;
        notifications.update(this, new ArrayList<>(calls.values()));
        PhoneCallActivity.changed();
    }

    void show(boolean dialpad) {
        Intent intent = new Intent(this, PhoneCallActivity.class)
                .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK | Intent.FLAG_ACTIVITY_SINGLE_TOP)
                .putExtra("dialpad", dialpad);
        try { startActivity(intent); }
        catch (RuntimeException restricted) { /* Telecom notification remains the system-owned entry if background UI is restricted. */ }
    }

    static JSONArray snapshot(Context context) {
        JSONArray rows = new JSONArray();
        PhoneInCallService service = current.get();
        if (service == null || !PhoneRoles.held(context, "dialer")) return rows;
        for (Entry entry : service.calls.values()) {
            if (rows.length() >= 32) break;
            Call.Details details = entry.call.getDetails();
            Uri address = details.getHandle();
            JSONObject row = PhoneJson.put(new JSONObject(), "id", entry.id);
            PhoneJson.put(row, "number", address == null ? "" : PhoneJson.text(address.getSchemeSpecificPart(), 256, false));
            PhoneJson.put(row, "name", PhoneJson.text(details.getCallerDisplayName(), 512, false));
            PhoneJson.put(row, "state", stateName(state(entry.call)));
            PhoneJson.put(row, "incoming", entry.incoming);
            PhoneJson.put(row, "muted", service.muted);
            PhoneJson.put(row, "speaker", service.speaker);
            PhoneJson.put(row, "can_hold", details.can(Call.Details.CAPABILITY_HOLD));
            PhoneJson.put(row, "connected_at", details.getConnectTimeMillis() > 0 ? details.getConnectTimeMillis() : null);
            PhoneJson.put(row, "account", details.getAccountHandle() == null ? null : PhoneAccounts.id(details.getAccountHandle()));
            rows.put(row);
        }
        return rows;
    }

    private static String stateName(int state) {
        switch (state) {
            case Call.STATE_DIALING: return "dialing";
            case Call.STATE_RINGING: case Call.STATE_SIMULATED_RINGING: return "ringing";
            case Call.STATE_ACTIVE: return "active";
            case Call.STATE_HOLDING: return "holding";
            case Call.STATE_DISCONNECTING: case Call.STATE_DISCONNECTED: return "disconnected";
            default: return "connecting";
        }
    }

    static JSONObject dial(Context context, String number, String account) {
        if (!PhoneNumbers.valid(number)) return PhoneJson.error("The phone number is invalid.");
        Uri address = Uri.fromParts("tel", number, null);
        if (!PhoneRoles.held(context, "dialer")) {
            Intent dial = new Intent(Intent.ACTION_DIAL, address);
            // Before the user selects this app as dialer, explicitly use Android's selected dialer.
            TelecomManager telecom = context.getSystemService(TelecomManager.class);
            String selected = telecom == null ? null : telecom.getDefaultDialerPackage();
            if (selected != null && !selected.equals(context.getPackageName())) dial.setPackage(selected);
            context.startActivity(dial.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK));
            return PhoneJson.deviceAccepted("Android opened its phone app for confirmation. No call is reported as connected.");
        }
        if (!PhonePermissions.granted(context, Manifest.permission.CALL_PHONE))
            return PhoneJson.error("Android call permission is required before placing the call.");
        TelecomManager telecom = context.getSystemService(TelecomManager.class);
        if (telecom == null) return PhoneJson.error("Android Telecom is unavailable.");
        Bundle extras = new Bundle();
        if (account != null) {
            PhoneAccountHandle handle = PhoneAccounts.resolveCall(context, account);
            if (handle == null) return PhoneJson.error("That phone account is unavailable; refresh SIM accounts.");
            extras.putParcelable(TelecomManager.EXTRA_PHONE_ACCOUNT_HANDLE, handle);
        }
        // Telecom routes emergency calls to the preinstalled emergency dialer.
        telecom.placeCall(address, extras);
        return PhoneJson.deviceAccepted("Android Telecom accepted the call request. Connection state comes from InCallService.");
    }

    static JSONObject action(String id, JSONObject action) {
        PhoneInCallService service = current.get();
        Entry entry = service == null ? null : service.calls.get(id);
        if (entry == null || !PhoneRoles.held(service, "dialer")) return PhoneJson.error("That call is no longer managed by this application.");
        Call call = entry.call;
        int state = state(call);
        String kind = action.optString("kind");
        switch (kind) {
            case "answer":
                if (state != Call.STATE_RINGING && state != Call.STATE_SIMULATED_RINGING) return PhoneJson.error("That call is not ringing.");
                call.answer(VideoProfile.STATE_AUDIO_ONLY); break;
            case "reject":
                if (state != Call.STATE_RINGING && state != Call.STATE_SIMULATED_RINGING) return PhoneJson.error("That call is not ringing.");
                call.reject(false, null); break;
            case "end": call.disconnect(); break;
            case "hold":
                if (state != Call.STATE_ACTIVE || !call.getDetails().can(Call.Details.CAPABILITY_HOLD)) return PhoneJson.error("That call cannot be held.");
                call.hold(); break;
            case "resume":
                if (state != Call.STATE_HOLDING) return PhoneJson.error("That call is not on hold.");
                call.unhold(); break;
            case "setMuted": service.setMuted(action.optBoolean("muted")); break;
            case "setSpeaker": return service.speaker(action.optBoolean("enabled"));
            case "dtmf": {
                String tone = action.optString("tone");
                if (state != Call.STATE_ACTIVE || tone.length() != 1 || "0123456789*#".indexOf(tone.charAt(0)) < 0)
                    return PhoneJson.error("A single dial-pad tone requires an active call.");
                if (entry.stopTone != null) PhoneTasks.MAIN.removeCallbacks(entry.stopTone);
                call.stopDtmfTone();
                call.playDtmfTone(tone.charAt(0));
                entry.stopTone = call::stopDtmfTone;
                PhoneTasks.MAIN.postDelayed(entry.stopTone, 180);
                break;
            }
            default: return PhoneJson.error("Unsupported call action.");
        }
        return PhoneJson.deviceAccepted("Android Telecom accepted the call action; callbacks supply the observed result.");
    }

    private JSONObject speaker(boolean enabled) {
        if (Build.VERSION.SDK_INT >= 34) {
            CallEndpoint choice = null;
            for (CallEndpoint endpoint : endpoints) {
                int type = endpoint.getEndpointType();
                if (enabled ? type == CallEndpoint.TYPE_SPEAKER : type == CallEndpoint.TYPE_EARPIECE || type == CallEndpoint.TYPE_WIRED_HEADSET) { choice = endpoint; break; }
            }
            if (choice == null) return PhoneJson.error("That call audio endpoint is unavailable.");
            requestCallEndpointChange(choice, getMainExecutor(), new OutcomeReceiver<Void, CallEndpointException>() {
                @Override public void onResult(Void result) { changed(); }
                @Override public void onError(CallEndpointException error) { PhoneDevice.event("accepted", PhoneJson.put(new JSONObject(), "message", "Android could not change the call audio endpoint.")); changed(); }
            });
        } else setAudioRoute(enabled ? CallAudioState.ROUTE_SPEAKER : CallAudioState.ROUTE_WIRED_OR_EARPIECE);
        return PhoneJson.deviceAccepted("Android accepted the audio-route request. The endpoint callback determines the active route.");
    }

    JSONArray pendingAccounts(String id) {
        Entry entry = calls.get(id);
        return entry != null && state(entry.call) == Call.STATE_SELECT_PHONE_ACCOUNT ? PhoneAccounts.calls(this) : new JSONArray();
    }

    JSONObject selectAccount(String id, String account) {
        Entry entry = calls.get(id);
        PhoneAccountHandle handle = PhoneAccounts.resolveCall(this, account);
        if (entry == null || state(entry.call) != Call.STATE_SELECT_PHONE_ACCOUNT || handle == null)
            return PhoneJson.error("This call is no longer waiting for that SIM.");
        entry.call.phoneAccountSelected(handle, false);
        return PhoneJson.deviceAccepted("Android accepted the selected phone account.");
    }
}
