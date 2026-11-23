package org.ferroui.phone;

/** Activity ownership is separate from the permissions checked by every native provider. */
final class PhoneDeviceAccess {
    private PhoneDeviceAccess() {}

    static boolean requiresForeground(String kind) {
        if (kind == null) return true;
        switch (kind) {
            // Warm intents arrive before onResume. Explicit, cancellable reads use
            // application Context and do not open UI or request any permission.
            case "snapshot":
            case "contacts":
            case "callHistory":
            case "calls":
            case "messageThreads":
            case "messages":
            case "messageAttachments":
            case "notifications":
            case "shortcuts": return false;
            // New requests require foreground ownership unless reviewed as reads.
            default: return true;
        }
    }
}
