package org.ferroui.phone;

/** Runs on the ordinary JVM: receipt ordering and numeric-address safety have no Android mocks. */
public final class PhoneContractTest {
    private static int assertions;
    private static void check(boolean value, String message) { assertions++; if (!value) throw new AssertionError(message); }
    public static void main(String[] args) {
        PhoneObservationEpoch epoch = new PhoneObservationEpoch();
        check(epoch.reconcile(0) == PhoneObservationEpoch.ALL, "First authority observation establishes a fresh private-read epoch");
        long unavailable = epoch.lease();
        check(epoch.reconcile(0) == 0 && epoch.current(unavailable), "An unchanged snapshot does not restart observations");
        check(epoch.reconcile(PhoneObservationEpoch.MESSAGES) == PhoneObservationEpoch.MESSAGES, "Message grant identifies only the changed authority");
        long granted = epoch.lease();
        check(!epoch.current(unavailable), "An old denied epoch cannot publish after grant");
        check(epoch.reconcile(0) == PhoneObservationEpoch.MESSAGES && !epoch.current(granted), "Revocation rejects a still-running private query");
        epoch.reconcile(PhoneObservationEpoch.MESSAGES);
        check(!epoch.current(granted), "Regrant never revives a query from before revocation");
        long regranted = epoch.lease(); epoch.invalidate();
        check(!epoch.current(regranted), "An own-app authority callback invalidates reads even if a revoke/regrant occurred between snapshots");
        check(epoch.reconcile(PhoneObservationEpoch.MESSAGES | PhoneObservationEpoch.NOTIFICATIONS) == PhoneObservationEpoch.NOTIFICATIONS, "Listener connection has its own authority bit");
        long connected = epoch.lease();
        check(epoch.reconcile(PhoneObservationEpoch.MESSAGES) == PhoneObservationEpoch.NOTIFICATIONS && !epoch.current(connected), "Listener disconnection invalidates the prior read epoch");
        for (String kind : new String[]{"messageThreads", "messages", "notifications", "missedCalls", "callHistory"})
            check(PhoneObservationEpoch.privateRow(kind, PhoneObservationEpoch.ALL), "Queued private rows are purged: " + kind);
        for (String kind : new String[]{"messageSent", "sendFailed", "messageDownloaded", "composeDraft", "calls", "snapshot"})
            check(!PhoneObservationEpoch.privateRow(kind, PhoneObservationEpoch.ALL), "Authority invalidation preserves accepted side effects: " + kind);
        check(!PhoneObservationEpoch.privateRow("notifications", PhoneObservationEpoch.MESSAGES), "SMS authority does not purge independent notification rows");
        check(!PhoneObservationEpoch.privateRow("messages", PhoneObservationEpoch.NOTIFICATIONS), "Listener authority does not purge independent message rows");
        long beforeBinder = epoch.lease();
        check(!epoch.usable(beforeBinder, () -> { epoch.invalidate(); return true; }), "A revoke/regrant during the authority IPC rejects a previously queued response");
        check(!epoch.usable(beforeBinder, () -> { throw new AssertionError("Stale lease queried authority"); }), "Stale reads do not query authority again");
        check(!epoch.usable(epoch.lease(), () -> false), "Current epoch does not override an actual denied operation");
        check(epoch.usable(epoch.lease(), () -> true), "A fresh authorized read remains usable");
        assertions += PhoneCallReceiptsTest.run();
        PhoneBackKey back = new PhoneBackKey();
        check(!back.up(1, 10, false), "An orphan Back release cannot navigate");
        back.down(1, 10, 0);
        check(back.up(1, 10, false), "One physical Back pair dispatches the shared callback");
        check(!back.up(1, 10, false), "A duplicate release cannot dispatch Back twice");
        back.down(1, 20, 1);
        check(!back.up(1, 20, false), "An orphan repeat cannot create a Back press");
        back.down(1, 30, 0); back.down(1, 30, 1); back.down(1, 30, 2);
        check(back.up(1, 30, false), "Repeats retain exactly one release callback");
        check(!back.up(1, 30, false), "Repeated press still has only one callback");
        back.down(1, 40, 0);
        check(!back.up(1, 40, true), "Canceled Back does not navigate");
        check(!back.up(1, 40, false), "Cancellation releases the owned key pair");
        back.down(1, 50, 0); back.cancel();
        check(!back.up(1, 50, false), "Activity suspension cancels an unfinished pair");
        back.down(1, 60, 0);
        check(!back.up(2, 60, false), "A different input device cannot release the pair");
        check(!back.up(1, 61, false), "A different down-time cannot release the pair");
        check(back.up(1, 60, false), "The actual matching release still owns its callback");
        back.down(1, 70, 0); back.down(1, 71, 0);
        check(!back.up(1, 70, false), "An old release cannot finish a newer Back press");
        check(back.up(1, 71, false), "The replacement press has exactly one callback");
        back.down(1, 72, 0);
        check(back.up(1, 72, false), "A distinct next press reaches App exit or HOME handling normally");
        for (String read : new String[]{"snapshot", "contacts", "callHistory", "missedCalls", "calls", "messageThreads", "messages", "messageAttachments", "notifications", "shortcuts"})
            check(!PhoneDeviceAccess.requiresForeground(read), "Warm activation can start the permission-gated read: " + read);
        for (String command : new String[]{"acknowledgeMissedCalls", "setControl", "openControl", "requestRole", "requestPermission", "saveContact", "deleteContact", "dial", "callAction", "composeMessage", "composeMultimediaMessage", "pickMessageAttachments", "discardMessageAttachments", "openMessageAttachment", "downloadMessage", "sendMessage", "sendMultimediaMessage", "markMessagesRead", "deleteMessages", "notificationAction", "dismissNotification", "dismissNotifications", "launchShortcut", "openApplicationSettings", "requestUninstallApplication", "openMap"})
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
