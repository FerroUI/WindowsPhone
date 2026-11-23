package org.ferroui.phone;

import android.content.Context;
import com.android.messaging.mmslib.pdu.*;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.util.UUID;
import org.json.JSONArray;
import org.json.JSONObject;

/** Publicly compiled AOSP codec, with bounded input and serialized access to its legacy static parser fields. */
final class PhoneMmsCodec {
    private PhoneMmsCodec() {}
    static synchronized GenericPdu parse(byte[] bytes) {
        if (bytes == null || bytes.length == 0 || bytes.length > PhoneMessageFiles.MAX_BYTES) return null;
        try { return new PduParser(bytes, true).parse(); }
        catch (RuntimeException malformed) { return null; }
    }
    static synchronized byte[] compose(Context context, GenericPdu pdu) {
        byte[] data = new PduComposer(context, pdu).make();
        if (data == null || data.length == 0 || data.length > PhoneMessageFiles.MAX_BYTES)
            throw new IllegalArgumentException("The multimedia message could not be encoded within the transfer limit.");
        return data;
    }

    static SendReq request(Context context, JSONObject message) throws Exception {
        JSONArray recipients = message.optJSONArray("recipients"), attachments = message.optJSONArray("attachments");
        if (!PhoneSmsSender.validRecipients(recipients) || attachments == null || attachments.length() > 8)
            throw new IllegalArgumentException("Invalid MMS recipients or attachments.");
        String subject = message.optString("subject"), text = message.optString("body");
        if (subject.getBytes(StandardCharsets.UTF_8).length > 512 || text.getBytes(StandardCharsets.UTF_8).length > 16000
                || subject.isEmpty() && text.isEmpty() && attachments.length() == 0)
            throw new IllegalArgumentException("Invalid multimedia message text.");
        SendReq request = new SendReq();
        request.setTransactionId(("phone-" + UUID.randomUUID()).getBytes(StandardCharsets.US_ASCII));
        request.setDate(System.currentTimeMillis() / 1000);
        request.setContentType("application/vnd.wap.multipart.mixed".getBytes(StandardCharsets.US_ASCII));
        request.setFrom(new EncodedStringValue(PduHeaders.FROM_INSERT_ADDRESS_TOKEN_STR));
        request.setMessageClass(PduHeaders.MESSAGE_CLASS_PERSONAL_STR.getBytes(StandardCharsets.US_ASCII));
        request.setDeliveryReport(PduHeaders.VALUE_YES);
        request.setReadReport(PduHeaders.VALUE_NO);
        EncodedStringValue[] to = new EncodedStringValue[recipients.length()];
        for (int i = 0; i < to.length; i++) to[i] = new EncodedStringValue(PhoneNumbers.messageAddress(recipients.optString(i)));
        request.setTo(to);
        if (!subject.isEmpty()) request.setSubject(new EncodedStringValue(subject));
        PduBody body = new PduBody();
        long size = 0;
        if (!text.isEmpty()) {
            PduPart part = new PduPart(); part.setContentType("text/plain".getBytes(StandardCharsets.US_ASCII));
            part.setCharset(CharacterSets.UTF_8); part.setContentLocation("message.txt".getBytes(StandardCharsets.US_ASCII));
            byte[] data = text.getBytes(StandardCharsets.UTF_8); part.setData(data); body.addPart(part); size += data.length;
        }
        java.util.HashSet<String> seen = new java.util.HashSet<>();
        for (int i = 0; i < attachments.length(); i++) {
            String id = attachments.optString(i);
            if (!seen.add(id)) throw new IllegalArgumentException("Duplicate MMS attachment.");
            PhoneMessageFiles.Lease lease = PhoneMessageFiles.draft(context, id);
            size += lease.file.length();
            if (size > PhoneMessageFiles.MAX_BYTES) throw new IllegalArgumentException("The multimedia message exceeds its transfer limit.");
            PduPart part = new PduPart();
            part.setContentType(lease.mime.getBytes(StandardCharsets.US_ASCII));
            part.setContentId(("attachment-" + i).getBytes(StandardCharsets.US_ASCII));
            // Header filenames are bounded, independent of the private filesystem identity.
            part.setName(lease.name.getBytes(StandardCharsets.UTF_8));
            part.setFilename(lease.name.getBytes(StandardCharsets.UTF_8));
            part.setContentLocation(("attachment-" + i).getBytes(StandardCharsets.US_ASCII));
            part.setData(Files.readAllBytes(lease.file.toPath()));
            body.addPart(part);
        }
        request.setBody(body); request.setMessageSize(size);
        return request;
    }

    static String iso(byte[] value) { return value == null ? null : new String(value, StandardCharsets.ISO_8859_1); }
    static String decoded(String value, int charset) {
        if (value == null) return "";
        return new EncodedStringValue(charset, value.getBytes(StandardCharsets.ISO_8859_1)).getString();
    }
    static String text(EncodedStringValue value) { return value == null ? "" : value.getString(); }
}
