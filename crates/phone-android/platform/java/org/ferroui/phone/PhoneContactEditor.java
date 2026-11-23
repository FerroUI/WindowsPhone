package org.ferroui.phone;

import android.Manifest;
import android.app.Activity;
import android.content.ContentValues;
import android.content.Context;
import android.content.Intent;
import android.net.Uri;
import android.provider.ContactsContract;
import android.provider.ContactsContract.CommonDataKinds;
import android.util.Base64;
import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import org.json.JSONArray;
import org.json.JSONObject;

/** Native contact editing preserves Android account/raw-contact aggregation and user confirmation. */
final class PhoneContactEditor {
    private PhoneContactEditor() {}
    static String identity(long id, String key) { return "contact:" + id + ":" + PhoneIntents.encode(key == null ? "" : key); }
    private static Uri lookup(Context context, String id) {
        if (id == null || id.length() > 1024 || !id.startsWith("contact:")) throw new IllegalArgumentException("Invalid device contact.");
        String[] fields = id.split(":", 3);
        if (fields.length != 3 || !fields[1].matches("[1-9][0-9]{0,18}") || !fields[2].matches("[A-Za-z0-9_-]+")) throw new IllegalArgumentException("Invalid contact lookup identity.");
        String key = new String(Base64.decode(fields[2], Base64.URL_SAFE | Base64.NO_WRAP | Base64.NO_PADDING), StandardCharsets.UTF_8);
        Uri lookup = ContactsContract.Contacts.getLookupUri(Long.parseLong(fields[1]), key);
        if (lookup == null || ContactsContract.Contacts.lookupContact(context.getContentResolver(), lookup) == null)
            throw new IllegalArgumentException("That contact was removed or is no longer accessible.");
        return lookup;
    }
    static boolean available(Context context) {
        return PhoneSettings.resolves(context, new Intent(Intent.ACTION_INSERT).setDataAndType(ContactsContract.Contacts.CONTENT_URI, ContactsContract.Contacts.CONTENT_TYPE));
    }
    static JSONObject edit(Activity activity, JSONObject contact) {
        if (contact == null) return PhoneJson.error("A contact is required.");
        String id = contact.optString("id");
        Intent intent = id.isEmpty() ? new Intent(Intent.ACTION_INSERT).setDataAndType(ContactsContract.Contacts.CONTENT_URI, ContactsContract.Contacts.CONTENT_TYPE)
                : new Intent(Intent.ACTION_EDIT).setDataAndType(lookup(activity, id), ContactsContract.Contacts.CONTENT_ITEM_TYPE);
        intent.putExtra(ContactsContract.Intents.Insert.NAME, contact.optString("name"));
        ArrayList<ContentValues> data = new ArrayList<>();
        add(data, contact.optJSONArray("phones"), CommonDataKinds.Phone.CONTENT_ITEM_TYPE);
        add(data, contact.optJSONArray("emails"), CommonDataKinds.Email.CONTENT_ITEM_TYPE);
        intent.putParcelableArrayListExtra(ContactsContract.Intents.Insert.DATA, data);
        intent.putExtra("finishActivityOnSaveCompleted", true);
        if (!PhoneSettings.resolves(activity, intent)) return PhoneJson.error("No Android contact editor is available.");
        activity.startActivity(intent);
        return PhoneJson.deviceAccepted("Android opened the contact editor. Review the account, merged contact and fields there; changes are observed after saving.");
    }
    private static void add(ArrayList<ContentValues> values, JSONArray fields, String mime) {
        if (fields == null) return;
        for (int index = 0; index < fields.length() && index < 32; index++) {
            ContentValues row = new ContentValues(); row.put(ContactsContract.Data.MIMETYPE, mime);
            row.put(ContactsContract.Data.DATA1, fields.optString(index)); values.add(row);
        }
    }
    static boolean canDelete(Activity activity) {
        if (PhonePermissions.granted(activity, Manifest.permission.WRITE_CONTACTS)) return true;
        activity.requestPermissions(new String[]{Manifest.permission.WRITE_CONTACTS}, PhonePermissions.REQUEST);
        return false;
    }
    static JSONObject delete(Context context, String id) {
        if (!PhonePermissions.granted(context, Manifest.permission.WRITE_CONTACTS)) return PhoneJson.error("Android contacts write permission is required to delete this contact.");
        int count = context.getContentResolver().delete(lookup(context, id), null, null);
        return count > 0 ? PhoneJson.deviceAccepted("Android deleted the selected device contact.") : PhoneJson.error("The contact is no longer available or its account is read only.");
    }
}
