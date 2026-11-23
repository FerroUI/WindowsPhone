package org.ferroui.phone;

import android.Manifest;
import android.app.Activity;
import android.content.BroadcastReceiver;
import android.content.Context;
import android.content.Intent;
import android.content.IntentFilter;
import android.database.ContentObserver;
import android.net.Uri;
import android.os.Build;
import android.os.CancellationSignal;
import android.os.Looper;
import android.provider.ContactsContract;
import android.provider.Telephony;
import android.telecom.TelecomManager;
import java.util.ArrayDeque;
import java.util.Iterator;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;
import java.util.concurrent.atomic.AtomicBoolean;
import java.util.concurrent.atomic.AtomicLong;
import org.json.JSONArray;
import org.json.JSONObject;

/** Main-thread capability/consent boundary. Personal provider reads run only after an explicit request. */
final class PhoneDevice {
    private static final String[] ROLES = {"home", "dialer", "sms", "notificationListener", "writeSettings", "doNotDisturb"};
    private static final String[] PERMISSIONS = {"contacts", "callLog", "readSms", "sendSms", "phone", "camera", "bluetooth", "location", "notifications"};
    private static final ArrayDeque<JSONObject> EVENTS = new ArrayDeque<>();
    private static final ExecutorService OBSERVATIONS = Executors.newSingleThreadExecutor(r -> { Thread t = new Thread(r, "phone-device-observations"); t.setDaemon(true); return t; });
    private static final AtomicBoolean messageDirty = new AtomicBoolean(), catalogDirty = new AtomicBoolean(true), refreshing = new AtomicBoolean();
    private static final AtomicLong generation = new AtomicLong();
    private static Context application;
    private static BroadcastReceiver packages;
    private static ContentObserver messages;
    private static CancellationSignal observationSignal;
    private static String lastSnapshot, currentThread, catalogResult;
    private static int threadLimit = 100, listLimit = 100;
    private static boolean observeList, observeNotifications, closed;
    private PhoneDevice() {}

    static void attach(Context context) {
        application = context.getApplicationContext(); closed = false;
        if (BuildConfig.DEBUG) PhoneDeviceSmoke.start(application);
        if (packages == null) {
            packages = new BroadcastReceiver() { @Override public void onReceive(Context c, Intent i) { catalogDirty.set(true); } };
            IntentFilter filter = new IntentFilter();
            filter.addAction(Intent.ACTION_PACKAGE_ADDED); filter.addAction(Intent.ACTION_PACKAGE_REMOVED);
            filter.addAction(Intent.ACTION_PACKAGE_CHANGED); filter.addDataScheme("package");
            if (Build.VERSION.SDK_INT >= 33) application.registerReceiver(packages, filter, Context.RECEIVER_NOT_EXPORTED);
            else application.registerReceiver(packages, filter);
        }
    }
    static void resumed(Context context) {
        attach(context); lastSnapshot = null; catalogDirty.set(true);
        if (observeList || currentThread != null) messageDirty.set(true);
        PhoneMessageJobService.wake(context);
    }
    static JSONObject snapshot(Context context) {
        JSONObject state = PhoneJson.put(new JSONObject(), "controls", PhoneHardware.observe(context));
        JSONArray roles = new JSONArray(), permissions = new JSONArray(), operations = new JSONArray();
        for (String role : ROLES) {
            JSONObject row = PhoneJson.put(new JSONObject(), "role", role);
            PhoneJson.put(row, "status", PhoneRoles.state(context, role));
            PhoneJson.put(row, "detail", "home".equals(role) && !BuildConfig.PHONE_LAUNCHER ? "Use the Launcher package to request Android HOME." : "Android owns role and special-access consent.");
            roles.put(row);
        }
        for (String permission : PERMISSIONS) permissions.put(PhoneJson.put(PhoneJson.put(new JSONObject(), "permission", permission), "status", PhonePermissions.groupState(context, permission)));
        PhoneJson.put(state, "roles", roles); PhoneJson.put(state, "permissions", permissions);
        PhoneJson.put(state, "phone_accounts", PhoneAccounts.calls(context));
        PhoneJson.put(state, "sms_subscriptions", PhoneAccounts.messages(context));
        PhoneJson.put(state, "settings", PhoneSettings.supported(context));
        if (PhonePermissions.granted(context, Manifest.permission.READ_CONTACTS)) operations.put("readContacts");
        if (PhoneContactEditor.available(context)) operations.put("writeContacts");
        if (PhonePermissions.granted(context, Manifest.permission.READ_CALL_LOG)) operations.put("callHistory");
        if (PhoneRoles.held(context, "dialer")) { operations.put("activeCalls"); operations.put("callControl"); }
        TelecomManager telecom = context.getSystemService(TelecomManager.class);
        if (telecom != null && (PhoneRoles.held(context, "dialer") && PhonePermissions.granted(context, Manifest.permission.CALL_PHONE)
                || telecom.getDefaultDialerPackage() != null && !context.getPackageName().equals(telecom.getDefaultDialerPackage()))) operations.put("dial");
        if (PhoneMessageProvider.readable(context)) { operations.put("readMessages"); operations.put("messageAttachments"); }
        if (PhoneMessageProvider.writable(context)) { operations.put("deleteMessages"); operations.put("downloadMessages"); }
        if (PhoneMessageProvider.writable(context) && PhonePermissions.granted(context, Manifest.permission.SEND_SMS)) {
            operations.put("sendMessages"); operations.put("sendMultimediaMessages");
        }
        if (PhoneSettings.resolves(context, new Intent(Intent.ACTION_SENDTO, Uri.parse("smsto:")))) operations.put("composeMessage");
        if (PhoneSettings.resolves(context, new Intent(Intent.ACTION_OPEN_DOCUMENT).setType("*/*").addCategory(Intent.CATEGORY_OPENABLE))) operations.put("pickMessageAttachments");
        if (PhoneNotificationListener.available(context)) { operations.put("readNotifications"); operations.put("notificationActions"); }
        if (PhoneShortcuts.available(context)) operations.put("appShortcuts");
        if (PhoneMaps.available(context)) operations.put("maps");
        Uri packageUri = Uri.fromParts("package", context.getPackageName(), null);
        if (PhoneSettings.resolves(context, new Intent(android.provider.Settings.ACTION_APPLICATION_DETAILS_SETTINGS, packageUri))) operations.put("applicationDetails");
        if (PhoneSettings.resolves(context, new Intent(Intent.ACTION_DELETE, packageUri))) operations.put("uninstallApplications");
        PhoneJson.put(state, "operations", operations);
        PhoneJson.put(state, "detail", "Android observations use current roles and permissions. SMS/MMS use the selected carrier; RCS is not exposed by Android's public messaging APIs. Protected toggles open Android-owned settings or consent.");
        return PhoneJson.device("snapshot", state);
    }
    static long begin(Context context, String json) {
        if (Looper.myLooper() != Looper.getMainLooper() || json == null || json.length() > 131072) return 0;
        attach(context);
        try {
            JSONObject request = new JSONObject(json); String kind = request.optString("kind");
            Activity activity = PhoneActivity.foreground();
            if ("snapshot".equals(kind)) return completed(snapshot(context));
            if (activity == null) return completed(PhoneJson.error("A foreground Android activity is required for this device request."));
            Context app = context.getApplicationContext();
            switch (kind) {
                case "setControl": return completed(PhoneHardware.set(activity, request.optString("control"), request.optJSONObject("value")));
                case "openControl": return completed(PhoneHardware.open(activity, request.optString("control")));
                case "requestRole": return completed(PhoneRoles.request(activity, request.optString("role")));
                case "requestPermission": return completed(PhonePermissions.request(activity, request.optString("permission")));
                case "contacts": return PhoneTasks.query(signal -> PhoneContacts.read(app, request.optString("query"), request.optInt("limit"), signal));
                case "saveContact": return completed(PhoneContactEditor.edit(activity, request.optJSONObject("contact")));
                case "deleteContact":
                    if (!PhoneContactEditor.canDelete(activity)) return completed(PhoneJson.deviceAccepted("Android opened contacts write consent. After granting it, choose Delete again to confirm the deletion."));
                    return PhoneTasks.work(() -> PhoneContactEditor.delete(app, request.optString("id")));
                case "callHistory": return PhoneTasks.query(signal -> PhoneCallHistory.read(app, request.optInt("limit"), signal));
                case "calls": return completed(PhoneJson.device("calls", PhoneInCallService.snapshot(app)));
                case "dial": return completed(PhoneInCallService.dial(activity, request.optString("number"), nullable(request, "account")));
                case "callAction": return completed(PhoneInCallService.action(request.optString("id"), request.optJSONObject("action")));
                case "messageThreads":
                    observeList = true; listLimit = request.optInt("limit"); observeMessages(app);
                    return PhoneTasks.query(signal -> PhoneMessageProvider.threads(app, request.optInt("limit"), signal));
                case "messages":
                    currentThread = request.optString("thread_id"); threadLimit = request.optInt("limit"); observeMessages(app);
                    return PhoneTasks.query(signal -> PhoneMessageProvider.messages(app, request.optString("thread_id"), request.optInt("limit"), signal));
                case "composeMessage": return completed(PhoneIntents.compose(activity, request));
                case "composeMultimediaMessage": return PhoneAttachments.compose(activity, request);
                case "pickMessageAttachments": return PhoneMessagePicker.begin(activity, request.optString("client_id"));
                case "discardMessageAttachments": return completed(PhoneMessagePicker.discard(app, request.optJSONArray("ids")));
                case "messageAttachments": return PhoneTasks.query(signal -> PhoneMmsReader.attachments(app, request.optString("message_id"), signal));
                case "openMessageAttachment": return PhoneAttachments.open(activity, request.optString("message_id"), request.optString("id"));
                case "downloadMessage": return PhoneTasks.work(() -> PhoneMmsTransport.download(app, request.optString("message_id")));
                case "sendMessage": return PhoneTasks.work(() -> PhoneSmsSender.send(app, request));
                case "sendMultimediaMessage": return PhoneTasks.work(() -> PhoneMmsTransport.send(app, request));
                case "markMessagesRead": return PhoneTasks.work(() -> { JSONObject result = PhoneMessageProvider.markRead(app, request.optString("thread_id")); if (!"error".equals(result.optString("kind"))) PhoneMessageNotifications.read(app, request.optString("thread_id")); return result; });
                case "deleteMessages": return PhoneTasks.work(() -> PhoneMessageProvider.delete(app, request.optString("thread_id"), request.optJSONArray("ids")));
                case "notifications": observeNotifications = true; return completed(PhoneNotificationListener.snapshot(app));
                case "notificationAction": return completed(PhoneNotificationListener.act(activity, request.optString("key"), nullable(request, "action"), nullable(request, "reply")));
                case "dismissNotification": return completed(PhoneNotificationListener.dismiss(app, request.optString("key")));
                case "dismissNotifications": {
                    JSONArray keys = request.optJSONArray("keys"); if (keys == null || keys.length() > 256) return completed(PhoneJson.error("Invalid notification selection."));
                    int accepted = 0; for (int index = 0; index < keys.length(); index++) if (!"error".equals(PhoneNotificationListener.dismiss(app, keys.optString(index)).optString("kind"))) accepted++;
                    return completed(PhoneJson.deviceAccepted("Android accepted " + accepted + " notification dismissal requests. Removed or ongoing notifications are left alone."));
                }
                case "shortcuts": return PhoneTasks.work(() -> PhoneShortcuts.read(app, request.optString("application")));
                case "launchShortcut": return completed(PhoneShortcuts.launch(activity, request.optString("application"), request.optString("id")));
                case "openApplicationSettings": return completed(PhoneLauncher.manage(activity, request.optString("application"), false));
                case "requestUninstallApplication": return completed(PhoneLauncher.manage(activity, request.optString("application"), true));
                case "openMap": return completed(PhoneMaps.open(activity, request));
                default: return completed(PhoneJson.error("Unsupported Android device request."));
            }
        } catch (Exception rejected) { return completed(PhoneJson.error("Android rejected this device operation: " + PhoneJson.text(rejected.getMessage(), 256, false))); }
    }
    private static String nullable(JSONObject object, String key) { return object.isNull(key) ? null : object.optString(key); }
    private static long completed(JSONObject result) {
        if ("accepted".equals(result.optString("kind"))) result = PhoneJson.deviceAccepted(result.optString("message"));
        long id = PhoneTasks.create(); if (id != 0) PhoneTasks.complete(id, result); return id;
    }
    static synchronized void event(String kind, Object value) { event(PhoneJson.put(PhoneJson.value(kind), "value", value)); }
    static synchronized void event(JSONObject response) {
        if (closed || response == null || PhoneJson.bytes(response) > 900000) return;
        String key = eventKey(response);
        for (Iterator<JSONObject> it = EVENTS.iterator(); it.hasNext();) if (key.equals(eventKey(it.next()))) it.remove();
        if (EVENTS.size() == 32) EVENTS.removeFirst(); EVENTS.addLast(response);
    }
    private static String eventKey(JSONObject response) {
        JSONObject value = response.optJSONObject("value"); String kind = response.optString("kind");
        return kind + ":" + (value == null ? "" : value.optString("client_id", value.optString("thread_id", value.optString("message_id", ""))));
    }
    static void messagesChanged() { messageDirty.set(true); }
    private static void observeMessages(Context context) {
        if (messages != null || !PhoneMessageProvider.readable(context)) return;
        messages = new ContentObserver(PhoneTasks.MAIN) { @Override public void onChange(boolean self, Uri uri) { messagesChanged(); } };
        context.getContentResolver().registerContentObserver(Telephony.MmsSms.CONTENT_URI, true, messages);
        context.getContentResolver().registerContentObserver(Telephony.Sms.CONTENT_URI, true, messages);
        context.getContentResolver().registerContentObserver(Telephony.Mms.CONTENT_URI, true, messages);
    }
    static String events(Context context) {
        if (Looper.myLooper() != Looper.getMainLooper()) return "[]";
        attach(context);
        try {
            JSONObject state = snapshot(context).optJSONObject("response");
            String encoded = state.toString();
            if (!encoded.equals(lastSnapshot)) { lastSnapshot = encoded; event(state); }
            if (PhoneInCallService.takeDirty()) event("calls", PhoneInCallService.snapshot(context));
            if (observeNotifications && PhoneNotificationListener.takeDirty()) {
                JSONObject notices = PhoneNotificationListener.snapshot(context).optJSONObject("response");
                event(notices == null ? PhoneJson.put(PhoneJson.value("notifications"), "value", new JSONArray()) : notices);
            }
            refresh(context);
        } catch (RuntimeException revokedOrUnavailable) { lastSnapshot = null; }
        JSONArray result = new JSONArray(); int bytes = 2;
        synchronized (PhoneDevice.class) {
            while (!EVENTS.isEmpty() && result.length() < 16) {
                JSONObject next = EVENTS.peekFirst(); int size = PhoneJson.bytes(next) + 1;
                if (bytes + size > 1000000) break;
                bytes += size; result.put(EVENTS.removeFirst());
            }
        }
        return result.toString();
    }
    private static void refresh(Context context) {
        boolean readMessages = messageDirty.get() && (observeList || currentThread != null) && PhoneMessageProvider.readable(context);
        if (!(catalogDirty.get() || readMessages) || !refreshing.compareAndSet(false, true)) return;
        final boolean catalog = catalogDirty.getAndSet(false), lists = readMessages && observeList;
        final String thread = readMessages ? currentThread : null;
        final int threads = listLimit, messages = threadLimit;
        if (readMessages) messageDirty.set(false);
        final long lease = generation.get();
        final CancellationSignal signal = new CancellationSignal(); observationSignal = signal;
        Context app = context.getApplicationContext();
        OBSERVATIONS.execute(() -> {
            try {
                if (catalog) { JSONObject response = PhoneLauncher.applications(app); synchronized (PhoneDevice.class) { if (generation.get() == lease) catalogResult = response.toString(); } }
                if (lists) { JSONObject response = PhoneMessageProvider.threads(app, threads, signal); if (generation.get() == lease) event(response.optJSONObject("response")); }
                if (thread != null) { JSONObject response = PhoneMessageProvider.messages(app, thread, messages, signal); if (generation.get() == lease) event(response.optJSONObject("response")); }
            } catch (RuntimeException cancelledOrRevoked) { }
            finally { refreshing.set(false); }
        });
    }
    static synchronized String catalogEvents() { String result = catalogResult; catalogResult = null; return result == null ? "" : result; }
    static void close(Context context) {
        closed = true; generation.incrementAndGet();
        if (observationSignal != null) observationSignal.cancel(); observationSignal = null;
        if (packages != null) { try { context.getApplicationContext().unregisterReceiver(packages); } catch (RuntimeException ignored) { } packages = null; }
        if (messages != null) { context.getContentResolver().unregisterContentObserver(messages); messages = null; }
        synchronized (PhoneDevice.class) { EVENTS.clear(); catalogResult = null; }
        observeList = false; observeNotifications = false; currentThread = null; lastSnapshot = null;
        PhoneMessagePicker.close(); PhoneMessageFiles.discardDrafts(context); PhoneTorch.close(); PhoneNotificationListener.clearObserved();
        // InCallService and accepted carrier requests are Android-owned and outlive the shell.
    }
}
