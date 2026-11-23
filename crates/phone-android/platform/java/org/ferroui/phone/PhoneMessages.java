package org.ferroui.phone;

import android.content.ContentUris;
import android.content.ContentValues;
import android.content.Context;
import android.content.Intent;
import android.database.Cursor;
import android.net.Uri;
import android.provider.Telephony;
import android.telephony.SmsMessage;
import android.telephony.SubscriptionManager;
import android.util.Base64;
import com.android.messaging.mmslib.pdu.*;
import org.json.JSONArray;
import org.json.JSONObject;

/** Durable native processing; no portable backup or activity owns carrier/provider state. */
final class PhoneMessages {
    private PhoneMessages() {}
    static int subscription(Intent intent) {
        int result = intent.getIntExtra(SubscriptionManager.EXTRA_SUBSCRIPTION_INDEX, SubscriptionManager.INVALID_SUBSCRIPTION_ID);
        // Telephony's protected delivery broadcasts also carry this longstanding documented SMS extra.
        if (!SubscriptionManager.isValidSubscriptionId(result)) result = intent.getIntExtra("subscription", SubscriptionManager.INVALID_SUBSCRIPTION_ID);
        return result;
    }
    static void process(Context context, String kind, JSONObject value) {
        if (value == null) return;
        switch (kind) {
            case "smsResult": PhoneSmsSender.receivedResult(context, value); break;
            case "mmsResult": PhoneMmsTransport.result(context, value); break;
            case "incomingSms": incomingSms(context, value); break;
            case "incomingMms": incomingMms(context, value); break;
            case "replySms": {
                JSONObject result = PhoneSmsSender.send(context, value);
                if ("error".equals(result.optString("kind"))) PhoneMessageNotifications.show(context, "Message could not be sent", result.optString("message"), null);
                break;
            }
            default: break;
        }
    }
    private static void incomingSms(Context context, JSONObject value) {
        String identity = value.optString("identity");
        PhoneMessageJournal journal = PhoneMessageJournal.get(context);
        if (identity.isEmpty() || journal.received(identity)) return;
        JSONArray parts = value.optJSONArray("parts");
        if (parts == null || parts.length() == 0 || parts.length() > 256) return;
        StringBuilder body = new StringBuilder(); SmsMessage first = null;
        for (int index = 0; index < parts.length(); index++) {
            SmsMessage sms;
            try { sms = SmsMessage.createFromPdu(Base64.decode(parts.optString(index), Base64.DEFAULT), value.optString("format", "3gpp")); }
            catch (RuntimeException invalid) { return; }
            if (sms == null) return;
            if (first == null) first = sms;
            if (!java.util.Objects.equals(first.getOriginatingAddress(), sms.getOriginatingAddress())) return;
            if (sms.getMessageBody() != null) body.append(sms.getMessageBody());
            if (body.length() > 65536) return;
        }
        if (first == null) return;
        String address = PhoneJson.text(first.getOriginatingAddress(), 256, false);
        if (address.isEmpty()) return;
        long sent = Math.max(0, first.getTimestampMillis());
        int subscription = value.optInt("subscription", SubscriptionManager.INVALID_SUBSCRIPTION_ID);
        // Cross-provider transaction recovery: a crash after insertion must not duplicate an already stored PDU.
        String selection = "address=? AND body=? AND date_sent=? AND sub_id=? AND type=?";
        String[] args = {address, body.toString(), Long.toString(sent), Integer.toString(subscription), Integer.toString(Telephony.Sms.MESSAGE_TYPE_INBOX)};
        try (Cursor existing = context.getContentResolver().query(Telephony.Sms.CONTENT_URI, new String[]{Telephony.Sms._ID}, selection, args, null)) {
            if (existing != null && existing.moveToFirst()) { journal.recordReceived(identity); return; }
        }
        long thread = Telephony.Threads.getOrCreateThreadId(context, address);
        ContentValues row = new ContentValues();
        row.put(Telephony.Sms.ADDRESS, address); row.put(Telephony.Sms.BODY, body.toString());
        row.put(Telephony.Sms.DATE, value.optLong("received_at", System.currentTimeMillis())); row.put(Telephony.Sms.DATE_SENT, sent);
        row.put(Telephony.Sms.READ, 0); row.put(Telephony.Sms.SEEN, 0); row.put(Telephony.Sms.SUBSCRIPTION_ID, subscription);
        row.put(Telephony.Sms.THREAD_ID, thread); row.put(Telephony.Sms.PROTOCOL, first.getProtocolIdentifier());
        row.put(Telephony.Sms.REPLY_PATH_PRESENT, first.isReplyPathPresent() ? 1 : 0); row.put(Telephony.Sms.SERVICE_CENTER, first.getServiceCenterAddress());
        Uri stored = context.getContentResolver().insert(Telephony.Sms.Inbox.CONTENT_URI, row);
        if (stored == null) throw new IllegalStateException("Android could not retain the incoming text message.");
        journal.recordReceived(identity);
        PhoneDevice.messagesChanged();
        PhoneMessageNotifications.show(context, address, PhoneJson.text(body, 4096, true), "thread:" + thread);
    }
    private static void incomingMms(Context context, JSONObject value) {
        String identity = value.optString("identity");
        PhoneMessageJournal journal = PhoneMessageJournal.get(context);
        if (identity.isEmpty() || journal.received(identity)) return;
        GenericPdu pdu;
        try { pdu = PhoneMmsCodec.parse(Base64.decode(value.optString("pdu"), Base64.DEFAULT)); }
        catch (RuntimeException invalid) { return; }
        if (pdu instanceof NotificationInd) PhoneMmsTransport.notification(context, (NotificationInd) pdu, value.optInt("subscription", -1));
        else if (pdu instanceof DeliveryInd) PhoneMmsTransport.delivery(context, (DeliveryInd) pdu, value.optInt("subscription", -1));
        // Unsupported read reports are deliberately not exposed as delivery or read acknowledgement.
        journal.recordReceived(identity);
    }
}
