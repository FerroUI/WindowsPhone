package org.ferroui.phone;

import android.app.Activity;
import android.content.ClipData;
import android.content.Context;
import android.content.Intent;
import android.content.res.AssetFileDescriptor;
import android.database.Cursor;
import android.net.Uri;
import android.os.CancellationSignal;
import android.provider.OpenableColumns;
import java.io.FileOutputStream;
import java.io.InputStream;
import java.lang.ref.WeakReference;
import java.util.ArrayList;
import java.util.HashSet;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;
import org.json.JSONArray;
import org.json.JSONObject;

/** User-picked content becomes bounded private leases; arbitrary content/file paths are never accepted by the bridge. */
final class PhoneMessagePicker {
    static final int REQUEST = 0x1004;
    private static final ExecutorService COPY = Executors.newSingleThreadExecutor(r -> { Thread t = new Thread(r, "phone-message-picker"); t.setDaemon(true); return t; });
    private static Pending active;
    private static final class Pending {
        long ticket;
        String client;
        WeakReference<Activity> owner;
        Context application;
        CancellationSignal cancellation = new CancellationSignal();
        ArrayList<String> tokens = new ArrayList<>();
        volatile boolean committed;
    }
    private PhoneMessagePicker() {}

    static long begin(Activity activity, String client) {
        if (active != null) return 0;
        Pending pending = new Pending();
        pending.ticket = PhoneTasks.create(180000);
        if (pending.ticket == 0) return 0;
        pending.client = client;
        pending.application = activity.getApplicationContext();
        pending.owner = new WeakReference<>(activity);
        active = pending;
        PhoneTasks.cancellation(pending.ticket, () -> {
            pending.cancellation.cancel();
            if (active == pending) active = null;
            synchronized (pending.tokens) {
                if (!pending.committed) for (String token : pending.tokens) PhoneMessageFiles.remove(pending.application, token);
            }
        });
        Intent pick = new Intent(Intent.ACTION_OPEN_DOCUMENT).addCategory(Intent.CATEGORY_OPENABLE).setType("*/*")
                .putExtra(Intent.EXTRA_ALLOW_MULTIPLE, true)
                .putExtra(Intent.EXTRA_MIME_TYPES, new String[]{"image/*", "video/*", "audio/*", "text/plain", "text/vcard", "application/pdf"})
                .addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION);
        try { activity.startActivityForResult(pick, REQUEST); }
        catch (RuntimeException unavailable) { PhoneTasks.complete(pending.ticket, outcome(pending, null, "Android's document picker is unavailable.")); }
        return pending.ticket;
    }

    static void result(Activity activity, int result, Intent data) {
        Pending pending = active;
        if (pending == null || pending.owner.get() != activity) return;
        if (result != Activity.RESULT_OK || data == null) {
            PhoneTasks.complete(pending.ticket, outcome(pending, null, null)); return;
        }
        ArrayList<Uri> selected = new ArrayList<>();
        HashSet<Uri> seen = new HashSet<>();
        ClipData clips = data.getClipData();
        if (clips != null) for (int index = 0; index < clips.getItemCount() && index < 9; index++) {
            Uri uri = clips.getItemAt(index).getUri();
            if (uri != null && seen.add(uri)) selected.add(uri);
        }
        if (clips == null && data.getData() != null) selected.add(data.getData());
        if (selected.isEmpty() || selected.size() > 8 || selected.stream().anyMatch(uri -> !"content".equals(uri.getScheme()))) {
            PhoneTasks.complete(pending.ticket, outcome(pending, null, "Choose between one and eight document attachments.")); return;
        }
        COPY.execute(() -> copy(pending, selected));
    }

    private static void copy(Pending pending, ArrayList<Uri> selected) {
        JSONArray attachments = new JSONArray();
        long total = 0;
        try {
            for (Uri uri : selected) {
                pending.cancellation.throwIfCanceled();
                String name = "attachment", mime = pending.application.getContentResolver().getType(uri);
                try (Cursor cursor = pending.application.getContentResolver().query(uri,
                        new String[]{OpenableColumns.DISPLAY_NAME, OpenableColumns.SIZE}, null, null, null, pending.cancellation)) {
                    if (cursor != null && cursor.moveToFirst()) {
                        int column = cursor.getColumnIndex(OpenableColumns.DISPLAY_NAME);
                        if (column >= 0) name = cursor.getString(column);
                        column = cursor.getColumnIndex(OpenableColumns.SIZE);
                        if (column >= 0 && !cursor.isNull(column) && cursor.getLong(column) > PhoneMessageFiles.MAX_BYTES - total)
                            throw new IllegalArgumentException("The selected attachments exceed the 10 MiB private transfer limit.");
                    }
                }
                PhoneMessageFiles.Lease lease = PhoneMessageFiles.create(pending.application, "draft", name, mime);
                synchronized (pending.tokens) {
                    pending.tokens.add(lease.token);
                    pending.cancellation.throwIfCanceled();
                }
                try (AssetFileDescriptor descriptor = pending.application.getContentResolver().openAssetFileDescriptor(uri, "r", pending.cancellation)) {
                    if (descriptor == null) throw new java.io.IOException("That document is unavailable.");
                    try (InputStream input = descriptor.createInputStream(); FileOutputStream output = new FileOutputStream(lease.file)) {
                        byte[] buffer = new byte[16384]; int count;
                        while ((count = input.read(buffer)) >= 0) {
                            pending.cancellation.throwIfCanceled();
                            total += count;
                            if (total > PhoneMessageFiles.MAX_BYTES) throw new java.io.IOException("The selected attachments exceed the private transfer limit.");
                            output.write(buffer, 0, count);
                        }
                    }
                }
                attachments.put(lease.attachment());
            }
            synchronized (pending.tokens) {
                pending.cancellation.throwIfCanceled();
                pending.committed = PhoneTasks.complete(pending.ticket, outcome(pending, attachments, null));
            }
        } catch (Exception cancelledOrInvalid) {
            PhoneTasks.complete(pending.ticket, outcome(pending, null, "Android could not copy the selected attachments: " + PhoneJson.text(cancelledOrInvalid.getMessage(), 160, false)));
        } finally {
            synchronized (pending.tokens) {
                if (!pending.committed) for (String token : pending.tokens) PhoneMessageFiles.remove(pending.application, token);
            }
        }
    }

    private static JSONObject outcome(Pending pending, JSONArray attachments, String error) {
        JSONObject value = PhoneJson.put(new JSONObject(), "client_id", pending.client);
        PhoneJson.put(value, "attachments", attachments == null ? JSONObject.NULL : attachments);
        PhoneJson.put(value, "error", error == null ? JSONObject.NULL : PhoneJson.text(error, 4096, true));
        return PhoneJson.device("draftAttachments", value);
    }

    static JSONObject discard(Context context, JSONArray ids) {
        if (ids == null || ids.length() > 8) return PhoneJson.error("Invalid attachment selection.");
        for (int index = 0; index < ids.length(); index++) {
            try { PhoneMessageFiles.remove(context, PhoneMessageFiles.draft(context, ids.optString(index)).token); }
            catch (java.io.IOException expired) { /* Releasing an already expired draft lease is idempotent. */ }
        }
        return PhoneJson.deviceAccepted("The selected draft attachment leases were released.");
    }

    static void activityDestroyed(Activity activity) {
        Pending pending = active;
        if (pending != null && pending.owner.get() == activity) PhoneTasks.cancel(pending.ticket);
    }
    static void close() { Pending pending = active; if (pending != null) PhoneTasks.cancel(pending.ticket); }
}
