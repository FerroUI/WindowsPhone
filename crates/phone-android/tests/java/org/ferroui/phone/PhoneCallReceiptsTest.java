package org.ferroui.phone;

/** Portable tests execute the production receipt and authority policies. */
public final class PhoneCallReceiptsTest {
    private static int assertions;
    private static void check(boolean value, String message) { assertions++; if (!value) throw new AssertionError(message); }
    static int run() {
        assertions = 0;
        PhoneCallReceipts receipts = new PhoneCallReceipts();
        long firstRevision = receipts.revision(), firstLease = receipts.lease();
        java.util.ArrayList<PhoneCallReceipts.Row> captured = new java.util.ArrayList<>();
        captured.add(new PhoneCallReceipts.Row(11, 100));
        String token = receipts.issue(firstLease, 10, captured);
        captured.add(new PhoneCallReceipts.Row(12, 101));
        java.util.List<PhoneCallReceipts.Row> acknowledged = receipts.consume(token, 11);
        check(acknowledged.size() == 1 && acknowledged.get(0).id == 11 && acknowledged.get(0).timestamp == 100,
                "A later call cannot enter the immutable acknowledged snapshot");
        check(receipts.consume(token, 12).isEmpty(), "Call receipts are one-use");
        token = receipts.issue(firstLease, 20, captured);
        check(receipts.consume(token, 20 + PhoneCallReceipts.LIFETIME_MILLIS).isEmpty(), "Expired call selections cannot write");
        token = receipts.issue(firstLease, 30, captured);
        check(receipts.consume(token, 29).isEmpty(), "A regressed monotonic clock fails closed");
        token = receipts.issue(firstLease, 40, captured);
        for (int i = 0; i < PhoneCallReceipts.MAX_RECEIPTS; i++) receipts.issue(firstLease, 40 + i, captured);
        check(receipts.consume(token, 45).isEmpty(), "Unconsumed call selections have a bounded retention budget");
        token = receipts.issue(firstLease, 50, captured);
        receipts.clear();
        check(receipts.consume(token, 51).isEmpty(), "Role/permission/lifecycle loss retires all call selections");
        check(receipts.issue(firstLease, 52, captured) == null, "A pre-revocation query cannot mint a new call receipt after regrant");
        check(receipts.revision() > firstRevision, "Call observation ordering survives invalidation");
        for (int i = captured.size(); i <= PhoneCallReceipts.MAX_ROWS; i++) captured.add(new PhoneCallReceipts.Row(i, i));
        try { receipts.issue(receipts.lease(), 60, captured); throw new AssertionError("Unbounded call receipt accepted"); }
        catch (IllegalArgumentException expected) { assertions++; }
        check(PhoneObservationEpoch.privateRow("missedCalls", PhoneObservationEpoch.CALL_WRITE), "Write-role loss also retires queued acknowledgement receipts");
        check(!PhoneObservationEpoch.privateRow("messages", PhoneObservationEpoch.CALLS), "Call-log authority does not purge unrelated messages");
        return assertions;
    }
    public static void main(String[] args) { System.out.println("Call receipt assertions passed: " + run()); }
}
