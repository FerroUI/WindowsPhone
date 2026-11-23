package org.ferroui.phone;

import android.Manifest;
import android.content.Context;
import android.database.Cursor;
import android.net.Uri;
import android.os.CancellationSignal;
import android.provider.ContactsContract;
import android.provider.ContactsContract.CommonDataKinds;
import java.util.ArrayList;
import java.util.Comparator;
import java.util.HashMap;
import java.util.LinkedHashMap;
import org.json.JSONArray;
import org.json.JSONObject;

/** Bounded, cancellation-aware reads from the current profile's Contacts provider. */
final class PhoneContacts {
    private PhoneContacts() {}
    private static final class Contact {
        long id; String lookup, name; boolean favorite;
        ArrayList<String> phones = new ArrayList<>(), emails = new ArrayList<>();
    }

    static JSONObject read(Context context, String query, int limit, CancellationSignal signal) {
        if (!PhonePermissions.granted(context, Manifest.permission.READ_CONTACTS))
            return PhoneJson.error("Grant Android contacts permission before reading device contacts.");
        if (limit < 1 || limit > 1000 || query.getBytes(java.nio.charset.StandardCharsets.UTF_8).length > 512)
            return PhoneJson.error("Invalid device-contact query.");
        LinkedHashMap<Long, Contact> contacts = new LinkedHashMap<>();
        Uri base = query.isEmpty() ? ContactsContract.Contacts.CONTENT_URI
                : Uri.withAppendedPath(ContactsContract.Contacts.CONTENT_FILTER_URI, query);
        names(context, base, limit, contacts, signal, false);
        if (!query.isEmpty() && contacts.size() < limit) {
            names(context, Uri.withAppendedPath(CommonDataKinds.Phone.CONTENT_FILTER_URI, query), limit, contacts, signal, true);
            names(context, Uri.withAppendedPath(CommonDataKinds.Email.CONTENT_FILTER_URI, query), limit, contacts, signal, true);
        }
        ArrayList<Long> ids = new ArrayList<>(contacts.keySet());
        for (int start = 0; start < ids.size(); start += 100) {
            signal.throwIfCanceled();
            int count = Math.min(100, ids.size() - start);
            String[] arguments = new String[count + 2];
            StringBuilder selection = new StringBuilder(ContactsContract.Data.CONTACT_ID + " IN (");
            for (int i = 0; i < count; i++) {
                if (i != 0) selection.append(',');
                selection.append('?');
                arguments[i] = Long.toString(ids.get(start + i));
            }
            selection.append(") AND ").append(ContactsContract.Data.MIMETYPE).append(" IN (?,?)");
            arguments[count] = CommonDataKinds.Phone.CONTENT_ITEM_TYPE;
            arguments[count + 1] = CommonDataKinds.Email.CONTENT_ITEM_TYPE;
            String[] columns = {ContactsContract.Data.CONTACT_ID, ContactsContract.Data.MIMETYPE, ContactsContract.Data.DATA1};
            try (Cursor cursor = context.getContentResolver().query(ContactsContract.Data.CONTENT_URI, columns,
                    selection.toString(), arguments, ContactsContract.Data.IS_PRIMARY + " DESC", signal)) {
                if (cursor == null) continue;
                while (cursor.moveToNext()) {
                    signal.throwIfCanceled();
                    Contact contact = contacts.get(cursor.getLong(0));
                    if (contact == null) continue;
                    boolean phone = CommonDataKinds.Phone.CONTENT_ITEM_TYPE.equals(cursor.getString(1));
                    ArrayList<String> values = phone ? contact.phones : contact.emails;
                    String value = PhoneJson.text(cursor.getString(2), phone ? 256 : 512, false).trim();
                    if (!value.isEmpty() && values.size() < 32 && !values.contains(value)) values.add(value);
                }
            }
        }
        ArrayList<Contact> sorted = new ArrayList<>(contacts.values());
        sorted.sort(Comparator.comparing(value -> value.name, String.CASE_INSENSITIVE_ORDER));
        JSONArray rows = new JSONArray();
        int bytes = 0;
        for (Contact contact : sorted) {
            JSONObject row = PhoneJson.put(new JSONObject(), "id", PhoneContactEditor.identity(contact.id, contact.lookup));
            PhoneJson.put(row, "name", contact.name);
            PhoneJson.put(row, "phones", new JSONArray(contact.phones));
            PhoneJson.put(row, "emails", new JSONArray(contact.emails));
            PhoneJson.put(row, "favorite", contact.favorite);
            int size = PhoneJson.bytes(row) + 1;
            if (bytes + size > 850000) break;
            bytes += size;
            rows.put(row);
        }
        return PhoneJson.device("contacts", rows);
    }

    private static void names(Context context, Uri base, int limit, LinkedHashMap<Long, Contact> rows,
            CancellationSignal signal, boolean data) {
        Uri uri = base.buildUpon().appendQueryParameter(ContactsContract.LIMIT_PARAM_KEY, Integer.toString(limit)).build();
        String[] columns = {data ? ContactsContract.Data.CONTACT_ID : ContactsContract.Contacts._ID,
                ContactsContract.Contacts.DISPLAY_NAME_PRIMARY, ContactsContract.Contacts.STARRED, ContactsContract.Contacts.LOOKUP_KEY};
        try (Cursor cursor = context.getContentResolver().query(uri, columns, null, null,
                ContactsContract.Contacts.SORT_KEY_PRIMARY + " ASC", signal)) {
            if (cursor == null) return;
            while (rows.size() < limit && cursor.moveToNext()) {
                signal.throwIfCanceled();
                long id = cursor.getLong(0);
                if (id <= 0 || rows.containsKey(id)) continue;
                Contact row = new Contact();
                row.id = id;
                row.name = PhoneJson.text(cursor.getString(1), 512, false);
                row.favorite = cursor.getInt(2) != 0;
                row.lookup = cursor.getString(3);
                if (row.lookup == null || PhoneContactEditor.identity(id, row.lookup).length() > 1024) continue;
                rows.put(id, row);
            }
        }
    }
}
