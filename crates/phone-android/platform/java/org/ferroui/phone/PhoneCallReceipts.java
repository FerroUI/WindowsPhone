package org.ferroui.phone;

import java.util.ArrayList;
import java.util.Collections;
import java.util.Iterator;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.UUID;

/** Process-local, bounded authority for exactly the call rows a query observed. */
final class PhoneCallReceipts {
    static final int MAX_ROWS = 500, MAX_RECEIPTS = 4;
    static final long LIFETIME_MILLIS = 120000;
    static final class Row {
        final long id, timestamp;
        Row(long id, long timestamp) { this.id = id; this.timestamp = timestamp; }
    }
    private static final class Receipt {
        final long created;
        final List<Row> rows;
        Receipt(long created, List<Row> rows) { this.created = created; this.rows = Collections.unmodifiableList(new ArrayList<>(rows)); }
    }
    private final LinkedHashMap<String, Receipt> receipts = new LinkedHashMap<>();
    private long revision, generation;
    synchronized long revision() {
        if (revision == Long.MAX_VALUE) throw new IllegalStateException("Call observation identities exhausted.");
        return ++revision;
    }
    synchronized long lease() { return generation; }
    synchronized String issue(long lease, long now, List<Row> rows) {
        expire(now);
        if (lease != generation || rows.isEmpty()) return null;
        if (rows.size() > MAX_ROWS) throw new IllegalArgumentException("Too many call rows.");
        if (receipts.size() == MAX_RECEIPTS) receipts.remove(receipts.keySet().iterator().next());
        String token = UUID.randomUUID().toString();
        receipts.put(token, new Receipt(now, rows));
        return token;
    }
    synchronized List<Row> consume(String token, long now) {
        expire(now);
        Receipt receipt = receipts.remove(token);
        return receipt == null ? Collections.emptyList() : receipt.rows;
    }
    synchronized boolean current(long lease) { return lease == generation; }
    synchronized void clear() { receipts.clear(); generation++; }
    private void expire(long now) {
        for (Iterator<Receipt> it = receipts.values().iterator(); it.hasNext();) {
            long created = it.next().created;
            if (now < created || now - created >= LIFETIME_MILLIS) it.remove();
        }
    }
}
