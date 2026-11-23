package org.ferroui.phone;

import android.app.Activity;
import android.content.ClipData;
import android.content.ContentValues;
import android.content.Context;
import android.content.Intent;
import android.database.Cursor;
import android.database.MatrixCursor;
import android.net.Uri;
import android.os.ParcelFileDescriptor;
import android.os.Process;
import android.os.SystemClock;
import android.provider.OpenableColumns;
import android.system.Os;
import android.system.OsConstants;
import android.system.StructStat;
import android.webkit.MimeTypeMap;
import androidx.core.content.FileProvider;
import java.io.File;
import java.io.FileDescriptor;
import java.io.FileNotFoundException;
import java.nio.file.Files;
import java.util.ArrayList;
import java.util.HashMap;
import java.util.Locale;
import java.util.UUID;
import org.json.JSONObject;

/** Read-only, expiring handoffs of individual app-owned media files. */
public final class PhoneAssetProvider extends FileProvider {
    private static final long LIFETIME_MS = 10 * 60 * 1000;
    private static final HashMap<String, Lease> LEASES = new HashMap<>();

    private static final class Lease {
        final Uri uri;
        final File file;
        final String mime;
        final StructStat stat;
        final long expires = SystemClock.elapsedRealtime() + LIFETIME_MS;
        final Runnable cleanup;
        Lease(Context context, Uri uri, File file, String mime, StructStat stat) {
            this.uri = uri; this.file = file; this.mime = mime; this.stat = stat;
            cleanup = () -> release(context, uri);
        }
    }

    private static void release(Context context, Uri uri) {
        Lease lease;
        synchronized (LEASES) { lease = LEASES.remove(uri.toString()); }
        if (lease != null) PhoneTasks.MAIN.removeCallbacks(lease.cleanup);
        try { context.revokeUriPermission(uri, Intent.FLAG_GRANT_READ_URI_PERMISSION); }
        catch (RuntimeException ignored) { /* The receiving activity may already have ended. */ }
    }

    static void close(Context context) {
        ArrayList<Uri> active = new ArrayList<>();
        synchronized (LEASES) { for (Lease lease : LEASES.values()) active.add(lease.uri); }
        for (Uri uri : active) release(context, uri);
    }

    private static Lease lease(Uri uri) {
        synchronized (LEASES) {
            Lease lease = LEASES.get(uri.toString());
            if (lease == null || SystemClock.elapsedRealtime() >= lease.expires)
                throw new SecurityException("The media handoff has expired. Open it again from Windows Phone.");
            return lease;
        }
    }

    private static String contentType(File file, String requested) {
        String mime = requested.split(";", 2)[0].trim().toLowerCase(Locale.ROOT);
        if (mime.matches("(image|audio|video)/[a-z0-9.+_-]{1,96}")) return mime;
        String name = file.getName();
        String extension = name.substring(name.lastIndexOf('.') + 1).toLowerCase(Locale.ROOT);
        mime = MimeTypeMap.getSingleton().getMimeTypeFromExtension(extension);
        if (mime == null || !mime.matches("(image|audio|video)/[a-z0-9.+_-]{1,96}"))
            throw new IllegalArgumentException("This media file has no supported content type.");
        return mime;
    }

    static JSONObject open(Activity activity, String path, String requestedType) throws Exception {
        if (path == null || path.length() > 4096 || requestedType == null || requestedType.length() > 128)
            return PhoneJson.error("Invalid media handoff identity.");
        Context context = activity.getApplicationContext();
        File files = context.getFilesDir().getCanonicalFile();
        File owner = new File(files, "windows-phone");
        File root = new File(owner, "assets");
        File supplied = new File(path);
        File file = supplied.getCanonicalFile();
        if (Files.isSymbolicLink(owner.toPath()) || Files.isSymbolicLink(root.toPath())
                || Files.isSymbolicLink(supplied.toPath()) || !root.equals(file.getParentFile())
                || !file.getName().matches("[A-Za-z0-9][A-Za-z0-9_-]{0,127}\\.(jpg|jpeg|png|gif|webp|mp3|wav|flac|ogg|opus|m4a|aac|wma|mp4|m4v|mov|mkv|webm|avi|wmv|mpg|mpeg)"))
            return PhoneJson.error("Only an app-owned media asset can be handed to another application.");
        StructStat stat = Os.lstat(file.getPath());
        if (!OsConstants.S_ISREG(stat.st_mode) || stat.st_uid != Process.myUid()
                || stat.st_size <= 0 || stat.st_size > 128L * 1024 * 1024)
            return PhoneJson.error("The media asset is not a valid owned regular file.");
        String mime = contentType(file, requestedType);
        Uri uri = getUriForFile(context, BuildConfig.APPLICATION_ID + ".assets", file).buildUpon()
                .appendQueryParameter("lease", UUID.randomUUID().toString()).build();
        ArrayList<Uri> expired = new ArrayList<>();
        synchronized (LEASES) {
            for (Lease old : LEASES.values()) if (old.expires <= SystemClock.elapsedRealtime()) expired.add(old.uri);
        }
        for (Uri old : expired) release(context, old);
        Lease lease = new Lease(context, uri, file, mime, stat);
        synchronized (LEASES) {
            if (LEASES.size() >= 32) return PhoneJson.error("Too many media handoffs are still active.");
            LEASES.put(uri.toString(), lease);
        }
        PhoneTasks.MAIN.postDelayed(lease.cleanup, LIFETIME_MS);
        Intent view = new Intent(Intent.ACTION_VIEW).setDataAndType(uri, mime)
                .addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION);
        view.setClipData(ClipData.newRawUri("Media", uri));
        try { activity.startActivity(view); }
        catch (RuntimeException error) { release(context, uri); throw error; }
        return PhoneJson.accepted("Android accepted the media handoff. Its read permission expires after ten minutes.");
    }

    @Override public ParcelFileDescriptor openFile(Uri uri, String mode) throws FileNotFoundException {
        if (!"r".equals(mode)) throw new SecurityException("Media handoffs are read-only.");
        Lease lease = lease(uri);
        FileDescriptor descriptor = null;
        try {
            descriptor = Os.open(lease.file.getPath(), OsConstants.O_RDONLY | OsConstants.O_NOFOLLOW | OsConstants.O_CLOEXEC, 0);
            StructStat stat = Os.fstat(descriptor);
            if (!OsConstants.S_ISREG(stat.st_mode) || stat.st_uid != Process.myUid()
                    || stat.st_ino != lease.stat.st_ino || stat.st_dev != lease.stat.st_dev || stat.st_size != lease.stat.st_size)
                throw new SecurityException("The media asset changed after this handoff.");
            return ParcelFileDescriptor.dup(descriptor);
        } catch (SecurityException error) { throw error; }
        catch (Exception error) { throw new FileNotFoundException("The media asset is no longer available."); }
        finally { if (descriptor != null) try { Os.close(descriptor); } catch (Exception ignored) {} }
    }

    @Override public String getType(Uri uri) { return lease(uri).mime; }

    @Override public Cursor query(Uri uri, String[] projection, String selection, String[] arguments, String order) {
        Lease lease = lease(uri);
        ArrayList<String> columns = new ArrayList<>();
        ArrayList<Object> values = new ArrayList<>();
        if (projection == null) projection = new String[]{OpenableColumns.DISPLAY_NAME, OpenableColumns.SIZE};
        if (projection.length > 32) throw new IllegalArgumentException("Too many media columns.");
        for (String column : projection) {
            if (OpenableColumns.DISPLAY_NAME.equals(column)) { columns.add(column); values.add(lease.file.getName()); }
            if (OpenableColumns.SIZE.equals(column)) { columns.add(column); values.add(lease.stat.st_size); }
        }
        MatrixCursor cursor = new MatrixCursor(columns.toArray(new String[0]), 1);
        cursor.addRow(values);
        return cursor;
    }

    @Override public int delete(Uri uri, String where, String[] args) { throw new SecurityException("Media handoffs are read-only."); }
    @Override public int update(Uri uri, ContentValues values, String where, String[] args) { throw new SecurityException("Media handoffs are read-only."); }
    @Override public Uri insert(Uri uri, ContentValues values) { throw new SecurityException("Media handoffs are read-only."); }
}
