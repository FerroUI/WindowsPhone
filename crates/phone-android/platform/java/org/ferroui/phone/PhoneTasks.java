package org.ferroui.phone;

import android.os.Handler;
import android.os.Looper;
import java.util.HashMap;
import java.util.concurrent.ArrayBlockingQueue;
import java.util.concurrent.Callable;
import java.util.concurrent.Future;
import java.util.concurrent.RejectedExecutionException;
import java.util.concurrent.ThreadPoolExecutor;
import java.util.concurrent.TimeUnit;
import org.json.JSONObject;

/** Bounded native operations. Dropped Rust futures cancel delivery and provider resources. */
final class PhoneTasks {
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
    }

    static synchronized long create() {
        if (tasks.size() >= 8 || next == Long.MAX_VALUE) return 0;
        long id = next++;
        Task task = new Task();
        tasks.put(id, task);
        task.timeout = () -> timeout(id);
        task.expiry = () -> cancel(id);
        MAIN.postDelayed(task.timeout, 30000);
        // A suspended UI may never consume its result. Bound its retention too.
        MAIN.postDelayed(task.expiry, 60000);
        return id;
    }

    static long work(Callable<JSONObject> operation) {
        long id = create();
        if (id == 0) return 0;
        try {
            Future<?> future = worker.submit(() -> {
                try { complete(id, operation.call()); }
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

    static void complete(long id, JSONObject result) {
        Runnable cancel;
        synchronized (PhoneTasks.class) {
            Task task = tasks.get(id);
            if (task == null || task.result != null) return;
            task.result = result.toString();
            MAIN.removeCallbacks(task.timeout);
            cancel = task.cancel;
            task.cancel = null;
        }
        if (cancel != null) MAIN.post(cancel);
    }

    static synchronized String poll(long id) {
        Task task = tasks.get(id);
        if (task == null) return PhoneJson.error("The Android request expired or was cancelled.").toString();
        if (task.result == null) return PhoneJson.value("pending").toString();
        tasks.remove(id);
        MAIN.removeCallbacks(task.timeout);
        MAIN.removeCallbacks(task.expiry);
        return task.result;
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
