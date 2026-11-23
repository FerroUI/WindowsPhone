package org.ferroui.phone;

import android.app.Notification;
import android.app.NotificationChannel;
import android.app.NotificationManager;
import android.app.PendingIntent;
import android.content.Context;
import android.content.Intent;
import android.media.AudioAttributes;
import android.media.MediaMetadata;
import android.media.session.MediaSession;
import android.media.session.PlaybackState;
import android.os.SystemClock;
import java.lang.ref.WeakReference;
import java.util.ArrayDeque;
import org.json.JSONArray;
import org.json.JSONObject;

/** Owns Android's session, never the decoder or a pointer into Rust/UI objects. */
public final class PhoneMediaSession {
    static final int NOTIFICATION = 2;
    static final String CHANNEL = "phone-media-playback";
    static WeakReference<PhoneMediaSession> current = new WeakReference<>(null);
    private final Context context;
    private final MediaSession session;
    private final ArrayDeque<JSONObject> commands = new ArrayDeque<>();
    private String generation = "0", title = "", artist = "", album = "", state = "stopped";
    private boolean active, next, previous, seekable, closed;
    private boolean serviceRequested;
    private long lastServiceAttempt;
    private long playRequestDeadline;
    private String notificationKey = "";
    private String serviceFailure;

    public static PhoneMediaSession create(Context context) {
        try { return new PhoneMediaSession(context); }
        catch (RuntimeException unavailable) { return null; }
    }

    private PhoneMediaSession(Context context) {
        this.context = context.getApplicationContext();
        session = new MediaSession(this.context, "Windows Phone");
        session.setFlags(MediaSession.FLAG_HANDLES_MEDIA_BUTTONS | MediaSession.FLAG_HANDLES_TRANSPORT_CONTROLS);
        session.setPlaybackToLocal(new AudioAttributes.Builder().setUsage(AudioAttributes.USAGE_MEDIA)
                .setContentType(AudioAttributes.CONTENT_TYPE_MUSIC).build());
        current = new WeakReference<>(this);
    }

    public synchronized String status() { return closed ? "closed" : "ready"; }

    private synchronized boolean enqueue(String expected, String action, long position) {
        if (closed || !active || !generation.equals(expected) || commands.size() >= 32) return false;
        if (!(action.equals("play") || action.equals("pause") || action.equals("stop")
                || action.equals("next") || action.equals("previous") || action.equals("seek"))) return false;
        if ((action.equals("next") && !next) || (action.equals("previous") && !previous)
                || (action.equals("seek") && (!seekable || position < 0 || position > 31536000000L))) return false;
        JSONObject command = PhoneJson.put(PhoneJson.put(new JSONObject(), "generation", expected), "action", action);
        if (action.equals("seek")) PhoneJson.put(command, "position_ms", position);
        commands.addLast(command);
        return true;
    }

    private MediaSession.Callback callbacks(final String expected) {
        return new MediaSession.Callback() {
            @Override public void onPlay() { requestPlay(expected); }
            @Override public void onPause() { enqueue(expected, "pause", 0); }
            @Override public void onStop() { enqueue(expected, "stop", 0); }
            @Override public void onSkipToNext() { enqueue(expected, "next", 0); }
            @Override public void onSkipToPrevious() { enqueue(expected, "previous", 0); }
            @Override public void onSeekTo(long position) { enqueue(expected, "seek", position); }
        };
    }

    private synchronized void requestPlay(String expected) {
        if (closed || !active || !generation.equals(expected)) return;
        try {
            context.startForegroundService(new Intent(context, PhoneMediaService.class)
                    .setAction("play").putExtra("generation", expected));
        } catch (RuntimeException denied) {
            serviceFailure = "Android declined background playback: " + PhoneJson.bounded(denied.getMessage(), 160);
        }
    }

    public synchronized String publish(String json) {
        if (closed) return "The Android media session is closed.";
        if (json == null || json.length() > 32768) return "Invalid Android media metadata size.";
        try {
            JSONObject value = new JSONObject(json);
            String incoming = value.getString("generation");
            if (!incoming.matches("[0-9]{1,20}")) return "Invalid media generation.";
            if (!generation.equals(incoming)) {
                generation = incoming; commands.clear();
                playRequestDeadline = 0;
                session.setCallback(callbacks(incoming), PhoneTasks.MAIN);
            }
            active = value.optBoolean("active") && !generation.equals("0");
            state = active ? value.optString("state", "stopped") : "stopped";
            next = active && value.optBoolean("can_next"); previous = active && value.optBoolean("can_previous");
            seekable = active && value.optBoolean("seekable");
            title = active ? PhoneJson.bounded(value.optString("title"), 1024) : "";
            artist = active ? PhoneJson.bounded(value.optString("artist"), 1024) : "";
            album = active ? PhoneJson.bounded(value.optString("album"), 1024) : "";
            long position = Math.max(0, Math.min(31536000000L, value.optLong("position_ms")));
            long duration = value.optLong("duration_ms", -1);
            if (duration >= 0) position = Math.min(position, duration);
            MediaMetadata.Builder metadata = new MediaMetadata.Builder().putString(MediaMetadata.METADATA_KEY_MEDIA_ID, generation)
                    .putString(MediaMetadata.METADATA_KEY_TITLE, title).putString(MediaMetadata.METADATA_KEY_ARTIST, artist)
                    .putString(MediaMetadata.METADATA_KEY_ALBUM, album);
            if (duration >= 0 && duration <= 31536000000L) metadata.putLong(MediaMetadata.METADATA_KEY_DURATION, duration);
            session.setMetadata(metadata.build());
            long actions = PlaybackState.ACTION_PLAY | PlaybackState.ACTION_PAUSE | PlaybackState.ACTION_PLAY_PAUSE | PlaybackState.ACTION_STOP;
            if (next) actions |= PlaybackState.ACTION_SKIP_TO_NEXT;
            if (previous) actions |= PlaybackState.ACTION_SKIP_TO_PREVIOUS;
            if (seekable) actions |= PlaybackState.ACTION_SEEK_TO;
            int playback = state.equals("playing") ? PlaybackState.STATE_PLAYING
                    : state.equals("paused") ? PlaybackState.STATE_PAUSED : PlaybackState.STATE_STOPPED;
            session.setPlaybackState(new PlaybackState.Builder().setActions(active ? actions : 0)
                    .setState(playback, position, playback == PlaybackState.STATE_PLAYING ? 1f : 0f,
                            SystemClock.elapsedRealtime()).build());
            session.setActive(active && playback != PlaybackState.STATE_STOPPED);
            updateNotification();
            if (serviceFailure != null) { String error = serviceFailure; serviceFailure = null; return error; }
            return "";
        } catch (Exception error) { return "Android could not publish media controls: " + PhoneJson.bounded(error.getMessage(), 160); }
    }

    public synchronized String takeCommands() {
        JSONArray values = new JSONArray();
        while (!commands.isEmpty()) values.put(commands.removeFirst());
        return values.toString();
    }

    private PendingIntent command(String action, int request) {
        Intent intent = new Intent(context, PhoneMediaService.class).setAction(action).putExtra("generation", generation);
        int flags = PendingIntent.FLAG_CANCEL_CURRENT | PendingIntent.FLAG_IMMUTABLE;
        // A visible Play action promotes its service before the asynchronous decoder
        // requests audio focus. This also satisfies Android 15's focus prerequisite.
        return action.equals("play") ? PendingIntent.getForegroundService(context, request, intent, flags)
                : PendingIntent.getService(context, request, intent, flags);
    }

    private static void createChannel(Context context) {
        NotificationManager manager = context.getSystemService(NotificationManager.class);
        NotificationChannel channel = new NotificationChannel(CHANNEL, "Media playback", NotificationManager.IMPORTANCE_LOW);
        channel.setSound(null, null); channel.enableVibration(false); manager.createNotificationChannel(channel);
    }

    static Notification endedNotification(Context context) {
        createChannel(context);
        return new Notification.Builder(context, CHANNEL).setSmallIcon(android.R.drawable.ic_media_play)
                .setContentTitle("Windows Phone").setContentText("Playback has ended.").setOnlyAlertOnce(true).build();
    }

    synchronized Notification notification() {
        createChannel(context);
        Intent open = PhoneActivity.openIntent(context);
        Notification.Builder builder = new Notification.Builder(context, CHANNEL)
                .setSmallIcon(android.R.drawable.ic_media_play).setContentTitle(title).setContentText(artist)
                .setSubText(album).setVisibility(Notification.VISIBILITY_PUBLIC).setOnlyAlertOnce(true)
                .setOngoing(state.equals("playing"))
                .setContentIntent(open == null ? null : PendingIntent.getActivity(context, 3, open,
                        PendingIntent.FLAG_UPDATE_CURRENT | PendingIntent.FLAG_IMMUTABLE));
        int index = 0;
        if (previous) { builder.addAction(new Notification.Action.Builder(android.R.drawable.ic_media_previous, "Previous", command("previous", 10)).build()); index++; }
        boolean playing = state.equals("playing");
        builder.addAction(new Notification.Action.Builder(playing ? android.R.drawable.ic_media_pause : android.R.drawable.ic_media_play,
                playing ? "Pause" : "Play", command(playing ? "pause" : "play", 11)).build());
        if (next) builder.addAction(new Notification.Action.Builder(android.R.drawable.ic_media_next, "Next", command("next", 12)).build());
        builder.setStyle(new Notification.MediaStyle().setMediaSession(session.getSessionToken()).setShowActionsInCompactView(index));
        return builder.build();
    }

    private void updateNotification() {
        NotificationManager manager = context.getSystemService(NotificationManager.class);
        String key = generation + "|" + state + "|" + title + "|" + artist + "|" + album + "|" + next + "|" + previous;
        if (!active || (state.equals("stopped") && SystemClock.elapsedRealtime() >= playRequestDeadline)) {
            if (serviceRequested) context.stopService(new Intent(context, PhoneMediaService.class));
            serviceRequested = false; notificationKey = ""; manager.cancel(NOTIFICATION); return;
        }
        if (state.equals("playing") && !serviceRequested) {
            if (SystemClock.elapsedRealtime() - lastServiceAttempt < 5000 && lastServiceAttempt != 0) return;
            lastServiceAttempt = SystemClock.elapsedRealtime();
            context.startForegroundService(new Intent(context, PhoneMediaService.class));
            serviceRequested = true;
        } else if (!state.equals("playing") && serviceRequested && SystemClock.elapsedRealtime() >= playRequestDeadline) {
            context.stopService(new Intent(context, PhoneMediaService.class)); serviceRequested = false;
        }
        if (!key.equals(notificationKey)) { manager.notify(NOTIFICATION, notification()); notificationKey = key; }
    }

    synchronized boolean serviceCommand(Intent intent) {
        boolean accepted = intent != null && intent.getAction() != null
                && enqueue(intent.getStringExtra("generation"), intent.getAction(), 0);
        if (accepted && intent.getAction().equals("play")) {
            playRequestDeadline = SystemClock.elapsedRealtime() + 5000;
            serviceRequested = true;
        }
        return accepted;
    }
    synchronized boolean playing() { return !closed && active && state.equals("playing"); }
    synchronized void taskRemoved() { enqueue(generation, "stop", 0); }
    synchronized void serviceStopped() { serviceRequested = false; playRequestDeadline = 0; }

    public synchronized void close() {
        if (closed) return;
        closed = true; commands.clear();
        try { session.setActive(false); session.setCallback(null); session.release(); } catch (RuntimeException ignored) {}
        try { context.stopService(new Intent(context, PhoneMediaService.class)); } catch (RuntimeException ignored) {}
        try { context.getSystemService(NotificationManager.class).cancel(NOTIFICATION); } catch (RuntimeException ignored) {}
        if (current.get() == this) current.clear();
    }
}
