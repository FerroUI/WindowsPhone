package org.ferroui.phone;

/** Normalize public SmsMessage.getStatus without conflating acceptance/forwarding with receipt. */
final class PhoneDeliveryStatus {
    private PhoneDeliveryStatus() {}
    // 0=confirmed received, 32=still pending, 64=terminal failure, -1=no delivery evidence.
    static int normalize(String format, int raw, boolean report) {
        if (!report || raw < 0) return -1;
        if ("3gpp".equals(format)) {
            if (raw == 0) return 0;
            if (raw >= 32 && raw < 64) return 32;
            if (raw >= 64 && raw < 128) return 64;
            return -1; // Forwarded/unconfirmed, replaced, reserved and carrier-specific completions.
        }
        if (!"3gpp2".equals(format) || (raw & 0xfc00ffff) != 0) return -1;
        int error = raw >>> 24 & 3, status = raw >>> 16 & 255;
        if (error == 0 && status == 2) return 0;
        if (error == 2) return 32;
        if (error == 3 || error == 0 && status == 3) return 64;
        return -1; // CDMA accepted/deposited and reserved error class do not prove receipt.
    }
}
