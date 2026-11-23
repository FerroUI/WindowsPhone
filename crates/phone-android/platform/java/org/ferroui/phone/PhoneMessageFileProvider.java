package org.ferroui.phone;

import android.database.Cursor;
import android.database.MatrixCursor;
import android.net.Uri;
import android.os.ParcelFileDescriptor;
import android.provider.OpenableColumns;
import androidx.core.content.FileProvider;
import java.io.FileNotFoundException;
import java.io.IOException;
import java.util.List;

/** Android's URI grant check is retained; this additionally checks identity, expiry and access direction. */
public final class PhoneMessageFileProvider extends FileProvider {
    private PhoneMessageFiles.Lease lease(Uri uri) throws FileNotFoundException {
        List<String> path = uri.getPathSegments();
        if (path.size() != 2 || !"transfers".equals(path.get(0)) || !path.get(1).endsWith(".data")) throw new FileNotFoundException("Invalid message transfer URI.");
        String token = path.get(1).substring(0, path.get(1).length() - 5);
        try { return PhoneMessageFiles.resolve(getContext(), token, null); }
        catch (IOException expired) { throw new FileNotFoundException(expired.getMessage()); }
    }
    @Override public ParcelFileDescriptor openFile(Uri uri, String mode) throws FileNotFoundException {
        PhoneMessageFiles.Lease lease = lease(uri);
        boolean write = mode.indexOf('w') >= 0 || mode.indexOf('+') >= 0;
        if (!"download".equals(lease.kind) && write) throw new FileNotFoundException("This message URI permits reading only.");
        if ("download".equals(lease.kind) && !("w".equals(mode) || "wt".equals(mode) || "rw".equals(mode) || "rwt".equals(mode) || "r".equals(mode)))
            throw new FileNotFoundException("Invalid download access mode.");
        return super.openFile(uri, mode);
    }
    @Override public Uri insert(Uri uri, android.content.ContentValues values) { throw new UnsupportedOperationException("Message leases cannot insert provider rows."); }
    @Override public int update(Uri uri, android.content.ContentValues values, String selection, String[] arguments) { throw new UnsupportedOperationException("Message leases cannot update provider rows."); }
    @Override public int delete(Uri uri, String selection, String[] arguments) { throw new UnsupportedOperationException("Only the owning message transport can release a lease."); }

    @Override public String getType(Uri uri) {
        try { return lease(uri).mime; } catch (FileNotFoundException unavailable) { return null; }
    }
    @Override public Cursor query(Uri uri, String[] projection, String selection, String[] args, String sort) {
        try {
            PhoneMessageFiles.Lease lease = lease(uri);
            MatrixCursor result = new MatrixCursor(new String[]{OpenableColumns.DISPLAY_NAME, OpenableColumns.SIZE});
            result.addRow(new Object[]{lease.name, lease.file.length()});
            return result;
        } catch (FileNotFoundException expired) { return new MatrixCursor(new String[]{OpenableColumns.DISPLAY_NAME, OpenableColumns.SIZE}); }
    }
}
