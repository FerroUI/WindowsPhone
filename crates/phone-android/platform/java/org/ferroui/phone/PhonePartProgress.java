package org.ferroui.phone;

import java.util.Base64;
import java.util.BitSet;

/** Idempotent multipart receipts: accepted transport, sent and delivered are different states. */
final class PhonePartProgress {
    final int total;
    final BitSet sent, delivered, failed;
    PhonePartProgress(int total, String sent, String delivered, String failed) {
        if (total < 1 || total > 256) throw new IllegalArgumentException("Invalid SMS part count.");
        this.total = total;
        this.sent = decode(sent); this.delivered = decode(delivered); this.failed = decode(failed);
        if (this.sent.length() > total || this.delivered.length() > total || this.failed.length() > total)
            throw new IllegalArgumentException("Invalid SMS receipt bitmap.");
    }
    private static BitSet decode(String value) { return value == null || value.isEmpty() ? new BitSet() : BitSet.valueOf(Base64.getDecoder().decode(value)); }
    static String encode(BitSet bits) { return Base64.getEncoder().encodeToString(bits.toByteArray()); }
    void sent(int part, boolean success) {
        check(part);
        if (delivered.get(part) || sent.get(part) || failed.get(part)) return;
        if (success) sent.set(part); else failed.set(part);
    }
    void delivered(int part, int status) {
        check(part);
        if (delivered.get(part)) return;
        if (status == 0) { delivered.set(part); sent.set(part); failed.clear(part); }
        else if (status == 64) failed.set(part);
        // 32..63 are temporary status reports, not delivery or a terminal failure.
    }
    private void check(int part) { if (part < 0 || part >= total) throw new IllegalArgumentException("Invalid SMS part identity."); }
    String status() {
        if (!failed.isEmpty()) return "failed";
        if (delivered.cardinality() == total) return "delivered";
        if (sent.cardinality() == total) return "sent";
        return "sending";
    }
}
