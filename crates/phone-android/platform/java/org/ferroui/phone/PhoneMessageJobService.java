package org.ferroui.phone;

import android.app.job.JobInfo;
import android.app.job.JobParameters;
import android.app.job.JobScheduler;
import android.app.job.JobService;
import android.app.job.JobWorkItem;
import android.content.ComponentName;
import android.content.Context;
import android.content.Intent;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;
import org.json.JSONObject;

/** System-scheduled message work uses no Activity, Rust pointers or foreground shell timers. */
public final class PhoneMessageJobService extends JobService {
    private static final int JOB = 0x46504d;
    private static final ExecutorService WORKER = Executors.newSingleThreadExecutor(runnable -> {
        Thread thread = new Thread(runnable, "phone-carrier-messages"); thread.setDaemon(true); return thread;
    });
    private final java.util.concurrent.atomic.AtomicLong generation = new java.util.concurrent.atomic.AtomicLong();

    static void wake(Context context) {
        if (!PhoneRoles.held(context, "sms") || !PhoneMessageJournal.get(context).pending()) return;
        JobScheduler scheduler = context.getSystemService(JobScheduler.class);
        if (scheduler == null) return;
        JobInfo job = new JobInfo.Builder(JOB, new ComponentName(context, PhoneMessageJobService.class))
                .setOverrideDeadline(0).build();
        // Enqueue extends an existing running job instead of cancelling it when another callback arrives.
        scheduler.enqueue(job, new JobWorkItem(new Intent("org.ferroui.phone.PROCESS_MESSAGE_JOURNAL")));
    }

    @Override public boolean onStartJob(JobParameters parameters) {
        final long lease = generation.incrementAndGet();
        WORKER.execute(() -> {
            boolean retry = false;
            try {
                JobWorkItem item;
                while (generation.get() == lease && (item = parameters.dequeueWork()) != null) {
                    JSONObject work;
                    while (generation.get() == lease && PhoneRoles.held(this, "sms") && (work = PhoneMessageJournal.get(this).next()) != null) {
                        PhoneMessages.process(this, work.optString("kind"), work.optJSONObject("payload"));
                        PhoneMessageJournal.get(this).complete(work.optLong("id"));
                    }
                    if (generation.get() == lease) parameters.completeWork(item);
                }
            } catch (RuntimeException providerUnavailable) { retry = PhoneRoles.held(this, "sms"); }
            boolean reschedule = retry;
            PhoneTasks.MAIN.post(() -> { if (generation.get() == lease) jobFinished(parameters, reschedule); });
        });
        return true;
    }

    @Override public boolean onStopJob(JobParameters parameters) { generation.incrementAndGet(); return true; }
}
