package org.ferroui.phone;

import android.content.ContentUris;
import android.content.ContentValues;
import android.content.Context;
import android.net.Uri;
import android.provider.Telephony;
import android.telephony.TelephonyManager;
import com.android.messaging.mmslib.pdu.*;
import java.io.IOException;
import java.io.OutputStream;
import java.nio.charset.StandardCharsets;
import java.util.HashSet;

/** Default-SMS provider persistence using public content APIs, without Android's hidden PduPersister. */
final class PhoneMmsStore {
    private PhoneMmsStore() {}

    static long notification(Context context, NotificationInd notification, int subscription) {
        String location = PhoneMmsCodec.iso(notification.getContentLocation());
        try (android.database.Cursor cursor = context.getContentResolver().query(Telephony.Mms.CONTENT_URI,
                new String[]{Telephony.Mms._ID}, "ct_l=? AND sub_id=?", new String[]{location, Integer.toString(subscription)}, null)) {
            if (cursor != null && cursor.moveToFirst()) return cursor.getLong(0);
        }
        ContentValues values = new ContentValues();
        values.put(Telephony.Mms.MESSAGE_TYPE, PduHeaders.MESSAGE_TYPE_NOTIFICATION_IND);
        values.put(Telephony.Mms.MMS_VERSION, notification.getMmsVersion());
        values.put(Telephony.Mms.DATE, System.currentTimeMillis() / 1000);
        values.put(Telephony.Mms.READ, 0); values.put(Telephony.Mms.SEEN, 0);
        values.put(Telephony.Mms.CONTENT_LOCATION, location);
        values.put(Telephony.Mms.TRANSACTION_ID, PhoneMmsCodec.iso(notification.getTransactionId()));
        values.put(Telephony.Mms.MESSAGE_SIZE, notification.getMessageSize());
        values.put(Telephony.Mms.EXPIRY, notification.getExpiry());
        values.put(Telephony.Mms.SUBSCRIPTION_ID, subscription);
        subject(values, notification.getSubject());
        HashSet<String> recipients = new HashSet<>();
        add(recipients, notification.getFrom());
        if (!recipients.isEmpty()) values.put(Telephony.Mms.THREAD_ID, Telephony.Threads.getOrCreateThreadId(context, recipients));
        Uri uri = context.getContentResolver().insert(Telephony.Mms.Inbox.CONTENT_URI, values);
        if (uri == null) throw new IllegalStateException("Android could not retain the incoming MMS notification.");
        long id = ContentUris.parseId(uri);
        address(context, id, notification.getFrom(), PduHeaders.FROM);
        return id;
    }

    static long outgoing(Context context, SendReq request, int subscription) throws IOException {
        validateContent(request);
        ContentValues values = headers(context, request, subscription, false);
        Uri uri = context.getContentResolver().insert(Telephony.Mms.Outbox.CONTENT_URI, values);
        if (uri == null) throw new IOException("Android could not create the MMS outbox record.");
        long id = ContentUris.parseId(uri);
        try { content(context, id, request); }
        catch (IOException | RuntimeException error) { context.getContentResolver().delete(uri, null, null); throw error; }
        return id;
    }

    static void incoming(Context context, long id, RetrieveConf message, int subscription, String location, String transaction) throws IOException {
        synchronized (PhoneMessageProvider.MUTATIONS) {
        if (!PhoneRoles.held(context, "sms") || !PhoneMmsTransport.notificationCurrent(context, id, subscription, location, transaction))
            throw new IOException("The MMS notification was removed or replaced before completion.");
        Uri uri = ContentUris.withAppendedId(Telephony.Mms.CONTENT_URI, id);
        // Preflight before any provider replacement; retain the notification identity on replay.
        validateContent(message);
        ContentValues values = headers(context, message, subscription, true);
        content(context, id, message);
        values.put(Telephony.Mms.MESSAGE_BOX, Telephony.Mms.MESSAGE_BOX_INBOX);
        values.put(Telephony.Mms.RETRIEVE_STATUS, PduHeaders.RETRIEVE_STATUS_OK);
        if (context.getContentResolver().update(uri, values, "m_type=? AND sub_id=? AND ct_l=? AND tr_id=?",
                new String[]{Integer.toString(PduHeaders.MESSAGE_TYPE_NOTIFICATION_IND), Integer.toString(subscription), location, transaction}) != 1) throw new IOException("The MMS notification was deleted before completion.");
        }
    }

    private static ContentValues headers(Context context, MultimediaMessagePdu message, int subscription, boolean incoming) {
        ContentValues values = new ContentValues();
        values.put(Telephony.Mms.MESSAGE_TYPE, message.getMessageType());
        values.put(Telephony.Mms.MMS_VERSION, message.getMmsVersion());
        values.put(Telephony.Mms.DATE, message.getDate() > 0 ? message.getDate() : System.currentTimeMillis() / 1000);
        values.put(Telephony.Mms.DATE_SENT, message.getDate());
        values.put(Telephony.Mms.READ, incoming ? 0 : 1); values.put(Telephony.Mms.SEEN, incoming ? 0 : 1);
        values.put(Telephony.Mms.SUBSCRIPTION_ID, subscription);
        subject(values, message.getSubject());
        HashSet<String> recipients = new HashSet<>();
        if (incoming) add(recipients, message.getFrom());
        if (message.getTo() != null) for (EncodedStringValue value : message.getTo()) add(recipients, value);
        if (message instanceof RetrieveConf) {
            RetrieveConf retrieved = (RetrieveConf) message;
            if (retrieved.getCc() != null) for (EncodedStringValue value : retrieved.getCc()) add(recipients, value);
            values.put(Telephony.Mms.MESSAGE_ID, PhoneMmsCodec.iso(retrieved.getMessageId()));
            values.put(Telephony.Mms.CONTENT_TYPE, PhoneMmsCodec.iso(retrieved.getContentType()));
        } else if (message instanceof SendReq) {
            SendReq sent = (SendReq) message;
            values.put(Telephony.Mms.TRANSACTION_ID, PhoneMmsCodec.iso(sent.getTransactionId()));
            values.put(Telephony.Mms.CONTENT_TYPE, PhoneMmsCodec.iso(sent.getContentType()));
            values.put(Telephony.Mms.MESSAGE_SIZE, sent.getMessageSize());
            values.put(Telephony.Mms.DELIVERY_REPORT, sent.getDeliveryReport());
        }
        if (incoming && recipients.size() > 1) {
            try {
                TelephonyManager manager = context.getSystemService(TelephonyManager.class);
                String own = manager == null ? null : manager.createForSubscriptionId(subscription).getLine1Number();
                String normalized = PhoneNumbers.messageAddress(own);
                if (normalized != null) recipients.removeIf(value -> normalized.equals(PhoneNumbers.messageAddress(value)));
            } catch (SecurityException unavailable) { }
        }
        if (!recipients.isEmpty()) values.put(Telephony.Mms.THREAD_ID, Telephony.Threads.getOrCreateThreadId(context, recipients));
        return values;
    }

    private static void add(HashSet<String> recipients, EncodedStringValue value) {
        String address = PhoneMmsCodec.text(value);
        if (!address.isEmpty() && !PduHeaders.FROM_INSERT_ADDRESS_TOKEN_STR.equals(address) && recipients.size() < 32)
            recipients.add(PhoneJson.text(address, 256, false));
    }

    private static void subject(ContentValues values, EncodedStringValue subject) {
        if (subject == null) return;
        values.put(Telephony.Mms.SUBJECT, PhoneMmsCodec.iso(subject.getTextString()));
        values.put(Telephony.Mms.SUBJECT_CHARSET, subject.getCharacterSet());
    }

    private static void validateContent(MultimediaMessagePdu message) throws IOException {
        // Validate all allocations and header bounds before replacing any provider part/address.
        PduBody checkedBody = message.getBody();
        if (checkedBody == null || checkedBody.getPartsNum() > 100) throw new IOException("Invalid MMS parts.");
        long checkedBytes = 0;
        for (int index = 0; index < checkedBody.getPartsNum(); index++) {
            PduPart part = checkedBody.getPart(index);
            if (part == null || part.getContentType() == null || part.getContentType().length > 128)
                throw new IOException("Invalid MMS content type.");
            byte[] bytes = part.getData(); checkedBytes += bytes == null ? 0 : bytes.length;
            if (checkedBytes > PhoneMessageFiles.MAX_BYTES) throw new IOException("MMS content exceeds its storage limit.");
            for (byte[] header : new byte[][]{part.getName(), part.getFilename(), part.getContentId(), part.getContentLocation()})
                if (header != null && header.length > 4096) throw new IOException("MMS part header exceeds its storage limit.");
        }
        if (message.getTo() != null && message.getTo().length > 32 || message instanceof RetrieveConf
                && ((RetrieveConf) message).getCc() != null && ((RetrieveConf) message).getCc().length > 32)
            throw new IOException("MMS address list exceeds its storage limit.");
        if (message.getSubject() != null && message.getSubject().getTextString().length > 4096)
            throw new IOException("MMS subject exceeds its storage limit.");
        validateAddress(message.getFrom());
        if (message.getTo() != null) for (EncodedStringValue value : message.getTo()) validateAddress(value);
        if (message instanceof RetrieveConf && ((RetrieveConf) message).getCc() != null)
            for (EncodedStringValue value : ((RetrieveConf) message).getCc()) validateAddress(value);
    }

    private static void validateAddress(EncodedStringValue value) throws IOException {
        if (value != null && value.getTextString().length > 512) throw new IOException("MMS address exceeds its storage limit.");
    }

    private static void content(Context context, long id, MultimediaMessagePdu message) throws IOException {
        validateContent(message);
        Uri messageUri = ContentUris.withAppendedId(Telephony.Mms.CONTENT_URI, id);
        Uri addresses = messageUri.buildUpon().appendPath("addr").build();
        Uri parts = messageUri.buildUpon().appendPath("part").build();
        context.getContentResolver().delete(addresses, null, null);
        context.getContentResolver().delete(parts, null, null);
        address(context, id, message.getFrom(), PduHeaders.FROM);
        if (message.getTo() != null) for (EncodedStringValue value : message.getTo()) address(context, id, value, PduHeaders.TO);
        if (message instanceof RetrieveConf && ((RetrieveConf) message).getCc() != null)
            for (EncodedStringValue value : ((RetrieveConf) message).getCc()) address(context, id, value, PduHeaders.CC);
        PduBody body = message.getBody();
        if (body == null || body.getPartsNum() > 100) throw new IOException("Invalid MMS parts.");
        long total = 0;
        for (int index = 0; index < body.getPartsNum(); index++) {
            PduPart part = body.getPart(index);
            byte[] data = part.getData();
            if (data == null) data = new byte[0];
            total += data.length;
            if (total > PhoneMessageFiles.MAX_BYTES) throw new IOException("MMS content exceeds its storage limit.");
            String mime = part.getContentType() == null ? "application/octet-stream" : new String(part.getContentType(), StandardCharsets.US_ASCII);
            if (!mime.matches("[a-zA-Z0-9!#$&^_.+-]+/[a-zA-Z0-9!#$&^_.+-]+") || mime.length() > 128) mime = "application/octet-stream";
            ContentValues values = new ContentValues();
            values.put(Telephony.Mms.Part.SEQ, index); values.put(Telephony.Mms.Part.CONTENT_TYPE, mime);
            values.put(Telephony.Mms.Part.CHARSET, part.getCharset());
            values.put(Telephony.Mms.Part.NAME, PhoneMmsCodec.iso(part.getName()));
            values.put(Telephony.Mms.Part.FILENAME, PhoneMmsCodec.iso(part.getFilename()));
            values.put(Telephony.Mms.Part.CONTENT_ID, PhoneMmsCodec.iso(part.getContentId()));
            values.put(Telephony.Mms.Part.CONTENT_LOCATION, PhoneMmsCodec.iso(part.getContentLocation()));
            boolean text = "text/plain".equals(mime) || "text/html".equals(mime) || "application/smil".equals(mime);
            if (text) {
                values.put(Telephony.Mms.Part.TEXT, new EncodedStringValue(part.getCharset(), data).getString());
                values.put(Telephony.Mms.Part.CHARSET, CharacterSets.UTF_8);
            }
            Uri stored = context.getContentResolver().insert(parts, values);
            if (stored == null) throw new IOException("Android could not store an MMS part.");
            if (!text) try (OutputStream output = context.getContentResolver().openOutputStream(stored, "w")) {
                if (output == null) throw new IOException("Android could not open an MMS part.");
                output.write(data);
            }
        }
    }

    private static void address(Context context, long id, EncodedStringValue address, int type) {
        if (address == null) return;
        ContentValues value = new ContentValues();
        value.put(Telephony.Mms.Addr.ADDRESS, PhoneMmsCodec.iso(address.getTextString()));
        value.put(Telephony.Mms.Addr.CHARSET, address.getCharacterSet()); value.put(Telephony.Mms.Addr.TYPE, type);
        context.getContentResolver().insert(ContentUris.withAppendedId(Telephony.Mms.CONTENT_URI, id).buildUpon().appendPath("addr").build(), value);
    }
}
