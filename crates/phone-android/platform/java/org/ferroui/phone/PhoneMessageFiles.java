package org.ferroui.phone;

import android.content.Context;
import android.content.SharedPreferences;
import android.net.Uri;
import androidx.core.content.FileProvider;
import java.io.File;
import java.io.IOException;
import java.util.Map;
import java.util.UUID;
import org.json.JSONObject;

/** Random, expiring leases over one private cache subtree, distinct from portable media assets. */
final class PhoneMessageFiles {
    static final long MAX_BYTES = 10L * 1024 * 1024;
    static final class Lease {
        final String token, kind, name, mime;
        final File file;
        final long expires;
        Lease(String token, String kind, String name, String mime, File file, long expires) {
            this.token = token; this.kind = kind; this.name = name; this.mime = mime; this.file = file; this.expires = expires;
        }
        Uri uri(Context context) { return FileProvider.getUriForFile(context, context.getPackageName() + ".message-files", file); }
        JSONObject attachment() {
            JSONObject row = PhoneJson.put(new JSONObject(), "id", "draft-attachment:" + token);
            PhoneJson.put(row, "name", name); PhoneJson.put(row, "mime", mime); PhoneJson.put(row, "bytes", file.length());
            return row;
        }
    }
    private PhoneMessageFiles() {}

    private static SharedPreferences metadata(Context context) { return context.getSharedPreferences("phone-message-files", Context.MODE_PRIVATE); }
    private static File root(Context context) { return new File(context.getCacheDir(), "phone-message-transfers"); }

    static String mime(String value) {
        return value != null && value.length() <= 128 && value.matches("[a-zA-Z0-9_.+-]+/[a-zA-Z0-9_.+-]+") ? value : "application/octet-stream";
    }

    static synchronized Lease create(Context context, String kind, String name, String mime) throws IOException {
        clean(context);
        if (!java.util.Arrays.asList("draft", "send", "download", "view").contains(kind)) throw new IOException("Invalid message lease purpose.");
        if (metadata(context).getAll().size() >= 128) throw new IOException("Too many message attachments are still in use.");
        File root = root(context);
        long reserved = 0;
        for (String active : metadata(context).getAll().keySet()) reserved += Math.max(MAX_BYTES, new File(root, active + ".data").length());
        if (reserved + MAX_BYTES > 256L * 1024 * 1024) throw new IOException("The private message transfer quota is full. Finish or release existing transfers first.");
        if (!root.isDirectory() && !root.mkdirs()) throw new IOException("The private message directory is unavailable.");
        String token = UUID.randomUUID().toString();
        File file = new File(root, token + ".data");
        if (!file.createNewFile()) throw new IOException("Could not allocate a private message attachment.");
        name = PhoneJson.text(name, 512, false).trim();
        if (name.isEmpty()) name = "attachment";
        mime = mime(mime);
        long lifetime = "draft".equals(kind) ? 30 * 60_000L : "view".equals(kind) ? 15 * 60_000L : 48 * 60 * 60_000L;
        Lease lease = new Lease(token, kind, name, mime, file, System.currentTimeMillis() + lifetime);
        JSONObject row = PhoneJson.put(new JSONObject(), "kind", kind);
        PhoneJson.put(row, "name", name); PhoneJson.put(row, "mime", mime); PhoneJson.put(row, "expires", lease.expires);
        if (!metadata(context).edit().putString(token, row.toString()).commit()) { file.delete(); throw new IOException("Could not retain the private attachment lease."); }
        return lease;
    }

    static synchronized Lease resolve(Context context, String token, String kind) throws IOException {
        if (token == null || !token.matches("[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}")) throw new IOException("Invalid attachment identity.");
        try {
            JSONObject saved = new JSONObject(metadata(context).getString(token, "{}"));
            File file = new File(root(context), token + ".data");
            long expires = saved.optLong("expires");
            String purpose = saved.optString("kind");
            if (!file.isFile() || expires < System.currentTimeMillis() || kind != null && !kind.equals(purpose)
                    || !file.getCanonicalFile().getParentFile().equals(root(context).getCanonicalFile())) throw new IOException("The attachment lease expired or is unavailable.");
            return new Lease(token, purpose, saved.optString("name"), saved.optString("mime"), file, expires);
        } catch (org.json.JSONException invalid) { throw new IOException("Invalid private attachment metadata."); }
    }

    static Lease draft(Context context, String id) throws IOException {
        if (id == null || !id.startsWith("draft-attachment:")) throw new IOException("Invalid draft attachment.");
        return resolve(context, id.substring("draft-attachment:".length()), "draft");
    }

    static synchronized void remove(Context context, String token) {
        if (token == null || !token.matches("[0-9a-f-]{36}")) return;
        File file = new File(root(context), token + ".data");
        try { context.revokeUriPermission(FileProvider.getUriForFile(context, context.getPackageName() + ".message-files", file),
                android.content.Intent.FLAG_GRANT_READ_URI_PERMISSION | android.content.Intent.FLAG_GRANT_WRITE_URI_PERMISSION); }
        catch (RuntimeException unavailable) { }
        file.delete();
        metadata(context).edit().remove(token).commit();
    }

    static synchronized void clean(Context context) {
        for (Map.Entry<String, ?> row : metadata(context).getAll().entrySet()) {
            try {
                JSONObject value = new JSONObject(String.valueOf(row.getValue()));
                if (value.optLong("expires") < System.currentTimeMillis() || !new File(root(context), row.getKey() + ".data").isFile()) remove(context, row.getKey());
            } catch (org.json.JSONException invalid) { remove(context, row.getKey()); }
        }
    }

    static synchronized void discardDrafts(Context context) {
        for (Map.Entry<String, ?> row : metadata(context).getAll().entrySet()) {
            try { if ("draft".equals(new JSONObject(String.valueOf(row.getValue())).optString("kind"))) remove(context, row.getKey()); }
            catch (org.json.JSONException ignored) { }
        }
    }
}
