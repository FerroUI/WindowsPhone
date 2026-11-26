package org.ferroui.phone;

/** Private-read lifetime only. PhoneDevice holds its queue monitor around every access. */
final class PhoneObservationEpoch {
    static final int MESSAGES = 1, NOTIFICATIONS = 2, CALLS = 4, CALL_WRITE = 8,
            ALL = MESSAGES | NOTIFICATIONS | CALLS | CALL_WRITE;
    private volatile long generation;
    private int authority;
    private boolean known;

    int reconcile(int next) {
        int changed = known ? authority ^ next : ALL;
        authority = next; known = true;
        if (changed != 0) invalidate();
        return changed;
    }
    void invalidate() { generation++; }
    long lease() { return generation; }
    boolean current(long lease) { return generation == lease; }
    boolean usable(long lease, java.util.function.BooleanSupplier authority) {
        // Authority checks can block in Binder while a revoke/regrant callback advances the epoch.
        return current(lease) && authority.getAsBoolean() && current(lease);
    }

    static boolean privateRow(String kind, int changed) {
        return (changed & MESSAGES) != 0 && ("messageThreads".equals(kind) || "messages".equals(kind))
                || (changed & NOTIFICATIONS) != 0 && "notifications".equals(kind)
                || (changed & (CALLS | CALL_WRITE)) != 0 && ("missedCalls".equals(kind) || "callHistory".equals(kind));
    }
}
