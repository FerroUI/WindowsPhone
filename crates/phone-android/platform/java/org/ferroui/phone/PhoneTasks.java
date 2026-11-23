package org.ferroui.phone;

import android.os.Handler;
import android.os.Looper;
import android.os.CancellationSignal;
import java.util.HashMap;
import java.util.concurrent.ArrayBlockingQueue;
import java.util.concurrent.Callable;
import java.util.concurrent.Future;
import java.util.concurrent.RejectedExecutionException;
import java.util.concurrent.ThreadPoolExecutor;
import java.util.concurrent.TimeUnit;
import java.util.function.BooleanSupplier;
import org.json.JSONObject;

/** Bounded native operations. Dropped Rust futures cancel delivery and provider resources. */
final class PhoneTasks {
    interface Query { JSONObject run(CancellationSignal cancellation) throws Exception; }
    static final Handler MAIN = new Handler(Looper.getMainLooper());
    private static final HashMap<Long, Task> tasks = new HashMap<>();
    private static final ThreadPoolExecutor worker = new ThreadPoolExecutor(1, 1, 0L,
            TimeUnit.MILLISECONDS, new ArrayBlockingQueue<>(8), runnable -> {
                Thread thread = new Thread(runnable, "phone-applications");
                thread.setDaemon(true);
                return thread;
            });
    private static long next = 1;

    private static final class Task {
        String result;
        Future<?> future;
        Runnable cancel;
        Runnable timeout;
        Runnable expiry;
        BooleanSupplier readLease;
    }

    static JSONObject onMain(Callable<JSONObject> operation) throws Exception {
        if (Looper.myLooper() == Looper.getMainLooper()) return operation.call();
        java.util.concurrent.FutureTask<JSONObject> task = new java.util.concurrent.FutureTask<>(operation);
        MAIN.post(task);
        try { return task.get(5, TimeUnit.SECONDS); }
        finally { if (!task.isDone()) { task.cancel(false); MAIN.removeCallbacks(task); } }
    }

    static synchronized long create() {
        return create(30000);
    }

    static synchronized long create(long timeoutMillis) {
        if (tasks.size() >= 8 || next == Long.MAX_VALUE) return 0;
        long id = next++;
        Task task = new Task();
        tasks.put(id, task);
        task.timeout = () -> timeout(id);
        task.expiry = () -> cancel(id);
        MAIN.postDelayed(task.timeout, Math.min(180000, Math.max(1000, timeoutMillis)));
        // A suspended UI may never consume its result. Bound its retention too.
        MAIN.postDelayed(task.expiry, Math.min(180000, Math.max(1000, timeoutMillis)) + 30000);
        return id;
    }

    static synchronized long createRead(BooleanSupplier lease) {
        long id = create();
        if (id != 0) tasks.get(id).readLease = lease;
        return id;
    }

    static long work(Callable<JSONObject> operation) {
        return work(operation, null);
    }

    private static long work(Callable<JSONObject> operation, BooleanSupplier lease) {
        long id = lease == null ? create() : createRead(lease);
        if (id == 0) return 0;
        try {
            Future<?> future = worker.submit(() -> {
                try {
                    synchronized (PhoneTasks.class) { Task current = tasks.get(id); if (current == null || current.result != null) return; }
                    complete(id, operation.call());
                }
                catch (Exception error) { complete(id, PhoneJson.error("Android could not complete the operation.")); }
            });
            synchronized (PhoneTasks.class) {
                Task task = tasks.get(id);
                if (task != null && task.result == null) task.future = future; else future.cancel(true);
            }
        } catch (RejectedExecutionException busy) {
            complete(id, PhoneJson.error("Android is already processing other application requests."));
        }
        return id;
    }

    static long query(Query operation) {
        return query(operation, null);
    }

    static long query(Query operation, BooleanSupplier lease) {
        CancellationSignal signal = new CancellationSignal();
        long id = work(() -> operation.run(signal), lease);
        if (id != 0) cancellation(id, signal::cancel);
        return id;
    }

    static void cancellation(long id, Runnable cancel) {
        synchronized (PhoneTasks.class) {
            Task task = tasks.get(id);
            if (task != null && task.result == null) { task.cancel = cancel; return; }
        }
        MAIN.post(cancel);
    }

    private static void timeout(long id) {
        Future<?> future;
        synchronized (PhoneTasks.class) {
            Task task = tasks.get(id);
            if (task == null || task.result != null) return;
            future = task.future;
        }
        complete(id, PhoneJson.error("The Android operation timed out."));
        if (future != null) future.cancel(true);
    }

    static boolean complete(long id, JSONObject result) {
        Runnable cancel;
        synchronized (PhoneTasks.class) {
            Task task = tasks.get(id);
            if (task == null || task.result != null) return false;
            task.result = (valid(task) ? result : readExpired()).toString();
            MAIN.removeCallbacks(task.timeout);
            cancel = task.cancel;
            task.cancel = null;
        }
        if (cancel != null) MAIN.post(cancel);
        return true;
    }

    static synchronized String poll(long id) {
        Task task = tasks.get(id);
        if (task == null) return PhoneJson.error("The Android request expired or was cancelled.").toString();
        if (!valid(task)) {
            cancel(id);
            return readExpired().toString();
        }
        if (task.result == null) return PhoneJson.value("pending").toString();
        tasks.remove(id);
        MAIN.removeCallbacks(task.timeout);
        MAIN.removeCallbacks(task.expiry);
        return task.result;
    }

    private static boolean valid(Task task) {
        if (task.readLease == null) return true;
        try { return task.readLease.getAsBoolean(); }
        catch (RuntimeException revokedOrUnavailable) { return false; }
    }

    private static JSONObject readExpired() { return PhoneJson.error("Android private-read access changed. Refresh the authorized data."); }

    static void invalidateReads() {
        java.util.ArrayList<Task> stale = new java.util.ArrayList<>();
        synchronized (PhoneTasks.class) {
            for (Task task : tasks.values()) {
                if (task.readLease != null && !valid(task)) {
                    // Replace even a completed result: it may not have been polled by Rust yet.
                    task.result = readExpired().toString();
                    MAIN.removeCallbacks(task.timeout);
                    stale.add(task);
                }
            }
        }
        for (Task task : stale) {
            if (task.future != null) task.future.cancel(true);
            Runnable cancel;
            synchronized (PhoneTasks.class) { cancel = task.cancel; task.cancel = null; }
            if (cancel != null) MAIN.post(cancel);
        }
    }

    static void cancel(long id) {
        Task task;
        synchronized (PhoneTasks.class) { task = tasks.remove(id); }
        if (task == null) return;
        MAIN.removeCallbacks(task.timeout);
        MAIN.removeCallbacks(task.expiry);
        if (task.future != null) task.future.cancel(true);
        if (task.cancel != null) MAIN.post(task.cancel);
    }

    static void close() {
        Long[] pending;
        synchronized (PhoneTasks.class) { pending = tasks.keySet().toArray(new Long[0]); }
        for (long id : pending) cancel(id);
        worker.purge();
    }
}
