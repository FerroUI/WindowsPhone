package org.ferroui.phone;

/** One physical BACK press owns one callback; repeated/cancelled/stale releases own none. */
final class PhoneBackKey {
    private boolean tracking;
    private int device;
    private long downTime;

    void down(int deviceId, long time, int repeatCount) {
        if (repeatCount == 0) {
            tracking = true;
            device = deviceId;
            downTime = time;
        }
    }

    boolean up(int deviceId, long time, boolean cancelled) {
        if (!tracking || device != deviceId || downTime != time) return false;
        tracking = false;
        return !cancelled;
    }

    void cancel() { tracking = false; }
}
