package org.ferroui.phone;

import android.app.Activity;
import android.content.ClipData;
import android.content.ContentUris;
import android.content.Context;
import android.content.Intent;
import android.content.pm.ResolveInfo;
import android.net.Uri;
import android.os.CancellationSignal;
import java.io.FileOutputStream;
import java.io.InputStream;
import java.lang.ref.WeakReference;
import java.util.ArrayList;
import java.util.HashSet;
import org.json.JSONArray;
import org.json.JSONObject;

final class PhoneAttachments {
    private PhoneAttachments() {}
    static long open(Activity activity, String message, String id) {
        WeakReference<Activity> owner = new WeakReference<>(activity);
        Context context = activity.getApplicationContext();
        return PhoneTasks.query(signal -> {
            if (!PhoneMessageProvider.readable(context)) return PhoneJson.error("SMS read access is required to open this MMS attachment.");
            JSONObject info = PhoneMmsReader.attachment(context, message, id, signal);
            long part = PhoneMessageProvider.identity(id, "mms-part:");
            PhoneMessageFiles.Lease file = PhoneMessageFiles.create(context, "view", info.optString("name"), info.optString("mime"));
            boolean handedOff = false;
            try {
                try (android.content.res.AssetFileDescriptor descriptor = context.getContentResolver().openAssetFileDescriptor(ContentUris.withAppendedId(PhoneMmsReader.PARTS, part), "r", signal)) {
                    if (descriptor == null) throw new java.io.IOException("The MMS attachment is unavailable.");
                    try (InputStream input = descriptor.createInputStream(); FileOutputStream output = new FileOutputStream(file.file)) { copy(input, output, signal); }
                }
                JSONObject result = PhoneTasks.onMain(() -> {
                    signal.throwIfCanceled();
                    Activity current = owner.get();
                    if (current == null || PhoneActivity.foreground() != current) return PhoneJson.error("Return to Windows Phone before opening the attachment.");
                    Uri uri = file.uri(context);
                    Intent open = new Intent(Intent.ACTION_VIEW).setDataAndType(uri, file.mime).addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION);
                    open.setClipData(ClipData.newRawUri(file.name, uri));
                    if (!PhoneSettings.resolves(current, open)) return PhoneJson.error("No installed application can open this attachment type.");
                    current.startActivity(open);
                    return PhoneJson.deviceAccepted("Android opened a temporary read-only copy of the selected MMS attachment.");
                });
                handedOff = "device".equals(result.optString("kind"));
                return result;
            } finally { if (!handedOff) PhoneMessageFiles.remove(context, file.token); }
        });
    }
    static boolean composerAvailable(Context context) {
        for (ResolveInfo row : context.getPackageManager().queryIntentActivities(new Intent(Intent.ACTION_SENDTO, Uri.parse("smsto:")), 0))
            if (row.activityInfo != null && row.activityInfo.enabled && row.activityInfo.exported && !context.getPackageName().equals(row.activityInfo.packageName)) return true;
        return false;
    }
    static long compose(Activity activity, JSONObject request) {
        WeakReference<Activity> owner = new WeakReference<>(activity);
        Context context = activity.getApplicationContext();
        return PhoneTasks.query(signal -> {
            ArrayList<PhoneMessageFiles.Lease> files = new ArrayList<>();
            boolean handedOff = false;
            try {
                JSONArray ids = request.optJSONArray("attachments");
                if (ids == null || ids.length() > 8) return PhoneJson.error("Invalid multimedia attachment selection.");
                ArrayList<Uri> uris = new ArrayList<>(); long total = 0;
                for (int index = 0; index < ids.length(); index++) {
                    signal.throwIfCanceled();
                    PhoneMessageFiles.Lease draft = PhoneMessageFiles.draft(context, ids.optString(index));
                    total += draft.file.length(); if (total > PhoneMessageFiles.MAX_BYTES) throw new java.io.IOException("Attachments exceed the message transfer limit.");
                    PhoneMessageFiles.Lease copy = PhoneMessageFiles.create(context, "view", draft.name, draft.mime); files.add(copy);
                    try (InputStream input = new java.io.FileInputStream(draft.file); FileOutputStream output = new FileOutputStream(copy.file)) { copy(input, output, signal); }
                    uris.add(copy.uri(context));
                }
                JSONObject response = PhoneTasks.onMain(() -> {
                    signal.throwIfCanceled();
                    Activity current = owner.get();
                    if (current == null || PhoneActivity.foreground() != current) return PhoneJson.error("Return to Windows Phone before opening the multimedia composer.");
                    ArrayList<Intent> choices = new ArrayList<>(); HashSet<String> seen = new HashSet<>();
                    StringBuilder recipients = new StringBuilder(); JSONArray to = request.optJSONArray("recipients");
                    if (to != null) for (int i = 0; i < to.length(); i++) { if (i > 0) recipients.append(';'); recipients.append(to.optString(i)); }
                    for (ResolveInfo candidate : context.getPackageManager().queryIntentActivities(new Intent(Intent.ACTION_SENDTO, Uri.parse("smsto:")), 0)) {
                        if (candidate.activityInfo == null || !candidate.activityInfo.enabled || !candidate.activityInfo.exported) continue;
                        String pkg = candidate.activityInfo.packageName;
                        if (context.getPackageName().equals(pkg) || !seen.add(pkg)) continue;
                        Intent send = new Intent(Intent.ACTION_SEND_MULTIPLE).setType("*/*").setPackage(pkg)
                                .putParcelableArrayListExtra(Intent.EXTRA_STREAM, uris).putExtra(Intent.EXTRA_SUBJECT, request.optString("subject"))
                                .putExtra(Intent.EXTRA_TEXT, request.optString("body")).putExtra("sms_body", request.optString("body"))
                                .putExtra("address", recipients.toString()).addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION);
                        if (!uris.isEmpty()) { ClipData clip = ClipData.newRawUri("Message attachments", uris.get(0)); for (int i = 1; i < uris.size(); i++) clip.addItem(new ClipData.Item(uris.get(i))); send.setClipData(clip); }
                        if (PhoneSettings.resolves(current, send)) choices.add(send);
                    }
                    if (choices.isEmpty()) return PhoneJson.error("No other installed SMS application accepts multimedia composition. Select Windows Phone's SMS role to use its native MMS transport.");
                    Intent chooser = Intent.createChooser(choices.remove(0), "Compose multimedia message");
                    if (!choices.isEmpty()) chooser.putExtra(Intent.EXTRA_INITIAL_INTENTS, choices.toArray(new Intent[0]));
                    current.startActivity(chooser);
                    return PhoneJson.deviceAccepted("Android opened a multimedia composer. Review recipients and attachments there; no send or delivery result is assumed.");
                });
                handedOff = "device".equals(response.optString("kind")); return response;
            } finally { if (!handedOff) for (PhoneMessageFiles.Lease file : files) PhoneMessageFiles.remove(context, file.token); }
        });
    }
    private static void copy(InputStream input, FileOutputStream output, CancellationSignal signal) throws java.io.IOException {
        byte[] bytes = new byte[16384]; int count; long size = 0;
        while ((count = input.read(bytes)) >= 0) {
            signal.throwIfCanceled(); size += count;
            if (size > PhoneMessageFiles.MAX_BYTES) throw new java.io.IOException("The attachment exceeds the 10 MiB transfer limit.");
            output.write(bytes, 0, count);
        }
    }
}
