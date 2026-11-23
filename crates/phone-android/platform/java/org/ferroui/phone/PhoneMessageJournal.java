package org.ferroui.phone;

import android.content.ContentValues;
import android.content.Context;
import android.database.Cursor;
import android.database.sqlite.SQLiteDatabase;
import android.database.sqlite.SQLiteOpenHelper;
import java.nio.charset.StandardCharsets;
import java.security.MessageDigest;
import java.util.UUID;
import org.json.JSONObject;

/** Private durable receipts/outbox identity; carrier callbacks survive shell and process teardown. */
final class PhoneMessageJournal extends SQLiteOpenHelper {
    private static PhoneMessageJournal instance;
    static synchronized PhoneMessageJournal get(Context context) {
        if (instance == null) instance = new PhoneMessageJournal(context.getApplicationContext());
        return instance;
    }
    private PhoneMessageJournal(Context context) { super(context, "phone-message-journal.db", null, 1); setWriteAheadLoggingEnabled(true); }
    @Override public void onCreate(SQLiteDatabase db) {
        db.execSQL("CREATE TABLE jobs (id INTEGER PRIMARY KEY AUTOINCREMENT, token TEXT NOT NULL UNIQUE, kind TEXT NOT NULL, payload TEXT NOT NULL)");
        db.execSQL("CREATE TABLE batches (client TEXT PRIMARY KEY, digest TEXT NOT NULL, result TEXT NOT NULL, status TEXT NOT NULL, created INTEGER NOT NULL)");
        db.execSQL("CREATE TABLE sends (token TEXT PRIMARY KEY, client TEXT NOT NULL, provider_id INTEGER NOT NULL, transport TEXT NOT NULL, provider_key TEXT NOT NULL, parts INTEGER NOT NULL, sent TEXT NOT NULL DEFAULT '', delivered TEXT NOT NULL DEFAULT '', failed TEXT NOT NULL DEFAULT '')");
        db.execSQL("CREATE TABLE mms_delivery (provider_id INTEGER NOT NULL, address TEXT NOT NULL, status INTEGER NOT NULL DEFAULT 0, PRIMARY KEY (provider_id,address))");
        db.execSQL("CREATE TABLE received (token TEXT PRIMARY KEY, created INTEGER NOT NULL)");
        db.execSQL("CREATE TABLE downloads (token TEXT PRIMARY KEY, provider_id INTEGER NOT NULL, subscription INTEGER NOT NULL, location TEXT NOT NULL, transaction_id TEXT NOT NULL)");
    }
    @Override public void onUpgrade(SQLiteDatabase db, int oldVersion, int newVersion) { throw new IllegalStateException("Unsupported message journal migration."); }

    synchronized void enqueue(String token, String kind, JSONObject payload) {
        String data = payload.toString();
        if (data.getBytes(StandardCharsets.UTF_8).length > 256000) throw new IllegalArgumentException("Message work exceeds its private journal limit.");
        SQLiteDatabase db = getWritableDatabase();
        try (Cursor count = db.rawQuery("SELECT COUNT(*) FROM jobs", null)) {
            if (count.moveToFirst() && count.getInt(0) >= 512) throw new IllegalStateException("Android message processing queue is full.");
        }
        ContentValues row = new ContentValues();
        row.put("token", token); row.put("kind", kind); row.put("payload", data);
        db.insertWithOnConflict("jobs", null, row, SQLiteDatabase.CONFLICT_IGNORE);
    }

    synchronized JSONObject next() {
        try (Cursor cursor = getReadableDatabase().query("jobs", new String[]{"id", "kind", "payload"}, null, null, null, null, "id ASC", "1")) {
            if (!cursor.moveToFirst()) return null;
            JSONObject result = PhoneJson.put(new JSONObject(), "id", cursor.getLong(0));
            PhoneJson.put(result, "kind", cursor.getString(1));
            try { return PhoneJson.put(result, "payload", new JSONObject(cursor.getString(2))); }
            catch (org.json.JSONException damaged) { getWritableDatabase().delete("jobs", "id=?", new String[]{Long.toString(cursor.getLong(0))}); return null; }
        }
    }
    synchronized void complete(long id) { getWritableDatabase().delete("jobs", "id=?", new String[]{Long.toString(id)}); }
    synchronized boolean pending() {
        try (Cursor cursor = getReadableDatabase().rawQuery("SELECT 1 FROM jobs LIMIT 1", null)) { return cursor.moveToFirst(); }
    }

    synchronized boolean received(String token) {
        try (Cursor cursor = getReadableDatabase().rawQuery("SELECT 1 FROM received WHERE token=?", new String[]{token})) { return cursor.moveToFirst(); }
    }
    synchronized void recordReceived(String token) {
        ContentValues row = new ContentValues(); row.put("token", token); row.put("created", System.currentTimeMillis());
        SQLiteDatabase db = getWritableDatabase();
        db.insertWithOnConflict("received", null, row, SQLiteDatabase.CONFLICT_IGNORE);
        db.execSQL("DELETE FROM received WHERE token IN (SELECT token FROM received ORDER BY created DESC LIMIT -1 OFFSET 8192)");
    }

    synchronized void deleted(Context context, String transport, long id) {
        SQLiteDatabase db = getWritableDatabase();
        try (Cursor sends = db.query("sends", new String[]{"token", "parts"}, "provider_id=? AND transport=?", new String[]{Long.toString(id), transport}, null, null, null)) {
            while (sends.moveToNext()) {
                String token = sends.getString(0);
                if ("sms".equals(transport)) for (int part = 0; part < sends.getInt(1); part++) {
                    PhoneSmsSender.receipt(context, token, part, "sent").cancel();
                    PhoneSmsSender.receipt(context, token, part, "delivered").cancel();
                } else { PhoneMmsTransport.receipt(context, token, "sent").cancel(); PhoneMessageFiles.remove(context, token); }
            }
        }
        db.delete("sends", "provider_id=? AND transport=?", new String[]{Long.toString(id), transport});
        if ("mms".equals(transport)) {
            try (Cursor pending = db.query("downloads", new String[]{"token"}, "provider_id=?", new String[]{Long.toString(id)}, null, null, null)) {
                while (pending.moveToNext()) { PhoneMmsTransport.receipt(context, pending.getString(0), "download").cancel(); PhoneMessageFiles.remove(context, pending.getString(0)); }
            }
            db.delete("downloads", "provider_id=?", new String[]{Long.toString(id)});
            db.delete("mms_delivery", "provider_id=?", new String[]{Long.toString(id)});
        }
        // Batches retain only a hash/result/status tombstone, never the deleted body or addresses.
    }

    synchronized void roleRevoked(Context context) {
        SQLiteDatabase db = getWritableDatabase();
        java.util.ArrayList<Long> ids = new java.util.ArrayList<>(); java.util.ArrayList<String> transports = new java.util.ArrayList<>();
        try (Cursor sends = db.query("sends", new String[]{"provider_id", "transport"}, null, null, null, null, null)) {
            while (sends.moveToNext()) { ids.add(sends.getLong(0)); transports.add(sends.getString(1)); }
        }
        for (int index = 0; index < ids.size(); index++) deleted(context, transports.get(index), ids.get(index));
        try (Cursor transfers = db.query("downloads", new String[]{"token"}, null, null, null, null, null)) {
            while (transfers.moveToNext()) { PhoneMmsTransport.receipt(context, transfers.getString(0), "download").cancel(); PhoneMessageFiles.remove(context, transfers.getString(0)); }
        }
        db.delete("downloads", null, null); db.delete("mms_delivery", null, null); db.delete("jobs", null, null);
        // A role revocation removes private message contents; opaque send tombstones remain bounded.
    }

    synchronized void prune(Context context) {
        SQLiteDatabase db = getWritableDatabase();
        long cutoff = System.currentTimeMillis() - 30L * 24 * 60 * 60 * 1000;
        java.util.ArrayList<String> clients = new java.util.ArrayList<>();
        try (Cursor old = db.query("batches", new String[]{"client"}, "created<?", new String[]{Long.toString(cutoff)}, null, null, "created ASC", "128")) {
            while (old.moveToNext()) clients.add(old.getString(0));
        }
        for (String client : clients) {
            try (Cursor sends = db.query("sends", new String[]{"provider_id", "transport"}, "client=?", new String[]{client}, null, null, null)) {
                java.util.ArrayList<Long> ids = new java.util.ArrayList<>(); java.util.ArrayList<String> transports = new java.util.ArrayList<>();
                while (sends.moveToNext()) { ids.add(sends.getLong(0)); transports.add(sends.getString(1)); }
                for (int index = 0; index < ids.size(); index++) deleted(context, transports.get(index), ids.get(index));
            }
            db.delete("batches", "client=?", new String[]{client});
        }
    }

    static String digest(String value) { return digest(value.getBytes(StandardCharsets.UTF_8)); }
    static String digest(byte[] value) {
        try {
            byte[] bytes = MessageDigest.getInstance("SHA-256").digest(value);
            StringBuilder result = new StringBuilder();
            for (byte b : bytes) result.append(String.format(java.util.Locale.ROOT, "%02x", b & 255));
            return result.toString();
        } catch (java.security.NoSuchAlgorithmException impossible) { throw new IllegalStateException(impossible); }
    }
}
