package org.ferroui.phone;

/** Runs on the ordinary JVM: receipt ordering and numeric-address safety have no Android mocks. */
public final class PhoneContractTest {
    private static int assertions;
    private static void check(boolean value, String message) { assertions++; if (!value) throw new AssertionError(message); }
    public static void main(String[] args) {
        for (String read : new String[]{"snapshot", "contacts", "callHistory", "calls", "messageThreads", "messages", "messageAttachments", "notifications", "shortcuts"})
            check(!PhoneDeviceAccess.requiresForeground(read), "Warm activation can start the permission-gated read: " + read);
        for (String command : new String[]{"setControl", "openControl", "requestRole", "requestPermission", "saveContact", "deleteContact", "dial", "callAction", "composeMessage", "composeMultimediaMessage", "pickMessageAttachments", "discardMessageAttachments", "openMessageAttachment", "downloadMessage", "sendMessage", "sendMultimediaMessage", "markMessagesRead", "deleteMessages", "notificationAction", "dismissNotification", "dismissNotifications", "launchShortcut", "openApplicationSettings", "requestUninstallApplication", "openMap"})
            check(PhoneDeviceAccess.requiresForeground(command), "Consent, UI and mutations retain foreground ownership: " + command);
        check(PhoneDeviceAccess.requiresForeground("futureUnknownCommand"), "New commands default to requiring foreground ownership");
        check(PhoneDeviceAccess.requiresForeground(null), "Missing commands cannot bypass foreground ownership");
        check(PhoneDeliveryStatus.normalize("3gpp2", 2 << 16, true) == 0, "CDMA delivery bits are 23..16");
        check(PhoneDeliveryStatus.normalize("3gpp2", 2 << 24 | 4 << 16, true) == 32, "CDMA temporary error class is bits25..24");
        check(PhoneDeliveryStatus.normalize("3gpp2", 3 << 24 | 4 << 16, true) == 64, "CDMA permanent error class");
        check(PhoneDeliveryStatus.normalize("3gpp2", 0, true) == -1, "CDMA accepted is not delivered");
        check(PhoneDeliveryStatus.normalize("3gpp2", 1 << 16, true) == -1, "CDMA deposited is not delivered");
        check(PhoneDeliveryStatus.normalize("3gpp2", 1 << 24, true) == -1, "Reserved CDMA error class stays unknown");
        check(PhoneDeliveryStatus.normalize("3gpp", 0, false) == -1, "A non-report SMS cannot confirm delivery");
        check(PhoneDeliveryStatus.normalize("3gpp", -1, true) == -1, "Negative status stays unknown");
        check(PhoneDeliveryStatus.normalize("3gpp", 0, true) == 0, "GSM received confirms delivery");
        check(PhoneDeliveryStatus.normalize("3gpp", 1, true) == -1, "GSM forwarding without confirmation is not delivery");
        check(PhoneDeliveryStatus.normalize("3gpp", 2, true) == -1, "GSM replacement is not delivery");
        check(PhoneDeliveryStatus.normalize("3gpp", 17, true) == -1, "Carrier-specific completion stays unknown");
        check(PhoneDeliveryStatus.normalize("3gpp", 128, true) == -1, "Reserved GSM status stays unknown");
        PhonePartProgress progress = new PhonePartProgress(3, "", "", "");
        check("sending".equals(progress.status()), "Creation is not a transport result");
        progress.sent(2, true); progress.sent(0, true);
        check("sending".equals(progress.status()), "Every SMS segment must be sent");
        progress.delivered(1, 32);
        check("sending".equals(progress.status()), "Temporary modem status is not success");
        progress.sent(1, true);
        check("sent".equals(progress.status()), "Sending is distinct from delivery");
        progress.delivered(2, 0); progress.delivered(0, 0);
        check("sent".equals(progress.status()), "One delivered segment cannot deliver a multipart message");
        progress.delivered(1, 64);
        check("failed".equals(progress.status()), "Terminal delivery failure is retained");
        progress.sent(1, true);
        check("failed".equals(progress.status()), "Late sent success cannot erase terminal failure");
        progress.delivered(1, 0); progress.delivered(1, 64); progress.sent(1, false);
        check("delivered".equals(progress.status()), "Actual delivery wins over stale failures");
        PhonePartProgress recovered = new PhonePartProgress(3, PhonePartProgress.encode(progress.sent), PhonePartProgress.encode(progress.delivered), PhonePartProgress.encode(progress.failed));
        check("delivered".equals(recovered.status()), "Durable receipt bits survive reconstruction");
        try { recovered.sent(3, true); throw new AssertionError("Out-of-range callback admitted"); } catch (IllegalArgumentException expected) { assertions++; }
        try { new PhonePartProgress(257, "", "", ""); throw new AssertionError("Unbounded multipart state admitted"); } catch (IllegalArgumentException expected) { assertions++; }
        try { new PhonePartProgress(1, "Ag==", "", ""); throw new AssertionError("Corrupt persisted segment admitted"); } catch (IllegalArgumentException expected) { assertions++; }
        check(PhoneNumbers.valid("+1 (555) 0101,123"), "Telecom supports pause sequences");
        check(PhoneNumbers.messageAddress("+1 (555) 0101,123") == null, "SMS must not turn a pause into another recipient number");
        check(PhoneNumbers.messageAddress("5550101;123") == null, "SMS must not silently drop an extension");
        check(PhoneNumbers.messageAddress("*#06#") == null, "Dialer service codes are not SMS addresses");
        check("+15550101".equals(PhoneNumbers.messageAddress("+1 (555) 0101")), "Display formatting normalizes safely");
        check(!PhoneNumbers.valid("tel:5550101?call=yes"), "Bridge numbers never inject URI parameters");
        check(!PhoneNumbers.valid("5550101\n"), "Control characters are rejected");
        check("😀".equals(PhoneJson.text("😀x", 4, false)), "UTF-8 byte bounds do not split a surrogate pair");
        check("".equals(PhoneJson.text("😀", 3, false)), "Four-byte code points respect a three-byte bound");
        check("ab".equals(PhoneJson.text("a\u0000\nb\ud800", 32, false)), "Line observations remove controls and invalid surrogates");
        check("a\nb".equals(PhoneJson.text("a\nb", 32, true)), "Message bodies retain allowed line breaks");
        System.out.println("Android pure contract assertions: " + assertions);
    }
}
