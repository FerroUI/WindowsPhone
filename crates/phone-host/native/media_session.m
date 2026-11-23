#import <Foundation/Foundation.h>
#import <MediaPlayer/MediaPlayer.h>
#import <TargetConditionals.h>
#if TARGET_OS_IPHONE
#import <AVFoundation/AVFoundation.h>
#endif
#include <stdint.h>
#include <stdio.h>
#include <math.h>

typedef struct {
    uint64_t generation;
    int active, state, seekable, can_next, can_previous;
    double position, duration;
    const char *title, *artist, *album;
} WPMediaSnapshot;
typedef struct { uint64_t generation; int kind; double value; } WPMediaCommand;

// This object owns its callback state and target tokens. Blocks only retain a
// weak Objective-C reference; neither a Rust object nor stack pointer is captured.
@interface WPPhoneMediaSession : NSObject
@property BOOL active;
@property BOOL closed;
@property BOOL seekable;
@property BOOL canNext;
@property BOOL canPrevious;
@property uint64_t generation;
@property double duration;
@property(nonatomic, strong) NSMutableArray<NSArray *> *events;
@property(nonatomic, strong) NSMutableArray<NSArray *> *registrations;
#if TARGET_OS_IPHONE
@property BOOL audioActive;
@property BOOL interrupted;
@property(nonatomic, strong) NSMutableArray *observers;
#endif
@end
@implementation WPPhoneMediaSession
@end

static MPRemoteCommandHandlerStatus enqueue(WPPhoneMediaSession *session, int kind, double value) {
    if (!session) return MPRemoteCommandHandlerStatusCommandFailed;
    @synchronized (session) {
        if (!session.active || session.closed || session.events.count >= 32)
            return MPRemoteCommandHandlerStatusCommandFailed;
        if (kind == 5 && !session.canNext) return MPRemoteCommandHandlerStatusNoSuchContent;
        if (kind == 6 && !session.canPrevious) return MPRemoteCommandHandlerStatusNoSuchContent;
        if (kind == 7 && (!session.seekable || !isfinite(value) || value < 0 || value > 31536000 ||
                         (session.duration >= 0 && value > session.duration)))
            return MPRemoteCommandHandlerStatusCommandFailed;
        [session.events addObject:@[@(session.generation), @(kind), @(value)]];
        return MPRemoteCommandHandlerStatusSuccess;
    }
}

static void register_command(WPPhoneMediaSession *session, MPRemoteCommand *command, int kind) {
    __weak WPPhoneMediaSession *weak = session;
    id token = [command addTargetWithHandler:^MPRemoteCommandHandlerStatus(MPRemoteCommandEvent *event) {
        double position = 0;
        if (kind == 7) {
            if (![event isKindOfClass:MPChangePlaybackPositionCommandEvent.class])
                return MPRemoteCommandHandlerStatusCommandFailed;
            position = ((MPChangePlaybackPositionCommandEvent *)event).positionTime;
        }
        return enqueue(weak, kind, position);
    }];
    if (token) [session.registrations addObject:@[command, token]];
    command.enabled = NO;
}

#if TARGET_OS_IPHONE
static void system_pause(WPPhoneMediaSession *session) {
    if (!session) return;
    @synchronized (session) {
        if (!session.active || session.closed) return;
        // A pending remote Play must not undo an interruption or expose audio
        // on the loudspeaker immediately after headphones are disconnected.
        [session.events removeAllObjects];
        [session.events addObject:@[@(session.generation), @2, @0]];
    }
}

static void observe_audio_session(WPPhoneMediaSession *session) {
    session.observers = [NSMutableArray new];
    __weak WPPhoneMediaSession *weak = session;
    NSNotificationCenter *center = NSNotificationCenter.defaultCenter;
    id interruption = [center addObserverForName:AVAudioSessionInterruptionNotification
        object:AVAudioSession.sharedInstance queue:NSOperationQueue.mainQueue
        usingBlock:^(NSNotification *notification) {
            WPPhoneMediaSession *owner = weak;
            if (!owner || owner.closed) return;
            BOOL began = [notification.userInfo[AVAudioSessionInterruptionTypeKey] unsignedIntegerValue]
                == AVAudioSessionInterruptionTypeBegan;
            owner.interrupted = began;
            owner.audioActive = NO;
            if (began) system_pause(owner);
            // Ending an interruption never starts playback without user intent.
        }];
    id route = [center addObserverForName:AVAudioSessionRouteChangeNotification
        object:AVAudioSession.sharedInstance queue:NSOperationQueue.mainQueue
        usingBlock:^(NSNotification *notification) {
            if ([notification.userInfo[AVAudioSessionRouteChangeReasonKey] unsignedIntegerValue]
                == AVAudioSessionRouteChangeReasonOldDeviceUnavailable) system_pause(weak);
        }];
    id reset = [center addObserverForName:AVAudioSessionMediaServicesWereResetNotification
        object:AVAudioSession.sharedInstance queue:NSOperationQueue.mainQueue
        usingBlock:^(NSNotification *notification) {
            WPPhoneMediaSession *owner = weak;
            if (!owner || owner.closed) return;
            owner.audioActive = NO;
            owner.interrupted = NO;
            system_pause(owner);
        }];
    for (id token in @[interruption, route, reset]) [session.observers addObject:token];
}

static BOOL update_audio_session(WPPhoneMediaSession *session, BOOL playing, char *error, size_t length) {
    if (session.interrupted) return YES;
    if (playing == session.audioActive) return YES;
    AVAudioSession *audio = AVAudioSession.sharedInstance;
    NSError *failure = nil;
    BOOL succeeded;
    if (playing) {
        // Defer activation until playback starts so merely opening the app
        // does not interrupt another app's audio. No background mode is added.
        succeeded = [audio setCategory:AVAudioSessionCategoryPlayback mode:AVAudioSessionModeDefault
            options:0 error:&failure] && [audio setActive:YES error:&failure];
    } else {
        succeeded = [audio setActive:NO withOptions:AVAudioSessionSetActiveOptionNotifyOthersOnDeactivation
            error:&failure];
    }
    if (succeeded) session.audioActive = playing;
    else if (length) snprintf(error, length, "%s", (failure.localizedDescription ?: @"iOS could not update the playback audio session.").UTF8String);
    return succeeded;
}
#endif

void *wp_media_create(char *error, size_t length) {
    @autoreleasepool {
        if (length) error[0] = 0;
        if (!NSThread.isMainThread || NSBundle.mainBundle.bundleIdentifier.length == 0) {
            if (length) snprintf(error, length, "%s", "Run the packaged Windows Phone application on its UI thread to use Apple media controls.");
            return NULL;
        }
        WPPhoneMediaSession *session = [WPPhoneMediaSession new];
        session.events = [NSMutableArray new];
        session.registrations = [NSMutableArray new];
        MPRemoteCommandCenter *center = MPRemoteCommandCenter.sharedCommandCenter;
        register_command(session, center.playCommand, 1);
        register_command(session, center.pauseCommand, 2);
        register_command(session, center.togglePlayPauseCommand, 3);
        register_command(session, center.stopCommand, 4);
        register_command(session, center.nextTrackCommand, 5);
        register_command(session, center.previousTrackCommand, 6);
        register_command(session, center.changePlaybackPositionCommand, 7);
        if (session.registrations.count != 7) {
            for (NSArray *entry in session.registrations) [(MPRemoteCommand *)entry[0] removeTarget:entry[1]];
            if (length) snprintf(error, length, "%s", "Apple MediaPlayer did not register all media command handlers.");
            return NULL;
        }
#if TARGET_OS_IPHONE
        observe_audio_session(session);
#endif
        return (__bridge_retained void *)session;
    }
}

int wp_media_publish(void *handle, const WPMediaSnapshot *snapshot, char *error, size_t length) {
    @autoreleasepool {
        if (length) error[0] = 0;
        WPPhoneMediaSession *session = (__bridge WPPhoneMediaSession *)handle;
        if (!session || !snapshot || !NSThread.isMainThread) {
            if (length) snprintf(error, length, "%s", "The media session is unavailable or not on the UI thread.");
            return 0;
        }
        @synchronized (session) {
            if (session.closed) {
                if (length) snprintf(error, length, "%s", "The media session is closed.");
                return 0;
            }
            if (session.generation != snapshot->generation || !snapshot->active) [session.events removeAllObjects];
            session.active = snapshot->active != 0;
            session.generation = snapshot->generation;
            session.seekable = snapshot->seekable != 0;
            session.canNext = snapshot->can_next != 0;
            session.canPrevious = snapshot->can_previous != 0;
            session.duration = snapshot->duration;
        }
#if TARGET_OS_IPHONE
        if (!update_audio_session(session, snapshot->active && snapshot->state == 2, error, length)) return 0;
#endif
        MPRemoteCommandCenter *center = MPRemoteCommandCenter.sharedCommandCenter;
        center.playCommand.enabled = session.active;
        center.pauseCommand.enabled = session.active;
        center.togglePlayPauseCommand.enabled = session.active;
        center.stopCommand.enabled = session.active;
        center.nextTrackCommand.enabled = session.active && session.canNext;
        center.previousTrackCommand.enabled = session.active && session.canPrevious;
        center.changePlaybackPositionCommand.enabled = session.active && session.seekable;
        MPNowPlayingInfoCenter *info = MPNowPlayingInfoCenter.defaultCenter;
        if (!snapshot->active) {
            info.nowPlayingInfo = nil;
#if !TARGET_OS_IPHONE
            info.playbackState = MPNowPlayingPlaybackStateStopped;
#endif
            return 1;
        }
        NSMutableDictionary *metadata = [NSMutableDictionary dictionaryWithDictionary:@{
            MPMediaItemPropertyTitle: [NSString stringWithUTF8String:snapshot->title],
            MPMediaItemPropertyArtist: [NSString stringWithUTF8String:snapshot->artist],
            MPMediaItemPropertyAlbumTitle: [NSString stringWithUTF8String:snapshot->album],
            MPNowPlayingInfoPropertyElapsedPlaybackTime: @(snapshot->position),
            MPNowPlayingInfoPropertyDefaultPlaybackRate: @1.0,
            MPNowPlayingInfoPropertyPlaybackRate: @(snapshot->state == 2 ? 1.0 : 0.0)
        }];
        if (snapshot->duration >= 0) metadata[MPMediaItemPropertyPlaybackDuration] = @(snapshot->duration);
#if TARGET_OS_IPHONE
        if (session.interrupted) metadata[MPNowPlayingInfoPropertyPlaybackRate] = @0.0;
#endif
        info.nowPlayingInfo = metadata;
#if !TARGET_OS_IPHONE
        info.playbackState = snapshot->state == 2 ? MPNowPlayingPlaybackStatePlaying :
            snapshot->state == 1 ? MPNowPlayingPlaybackStatePaused : MPNowPlayingPlaybackStateStopped;
#endif
        return 1;
    }
}

int wp_media_pop(void *handle, WPMediaCommand *command) {
    WPPhoneMediaSession *session = (__bridge WPPhoneMediaSession *)handle;
    if (!session || !command) return 0;
    @synchronized (session) {
        if (session.closed || session.events.count == 0) return 0;
        NSArray *event = session.events.firstObject;
        command->generation = [event[0] unsignedLongLongValue];
        command->kind = [event[1] intValue];
        command->value = [event[2] doubleValue];
        [session.events removeObjectAtIndex:0];
        return 1;
    }
}

void wp_media_close(void *handle) {
    @autoreleasepool {
        WPPhoneMediaSession *session = (__bridge_transfer WPPhoneMediaSession *)handle;
        if (!session) return;
        @synchronized (session) {
            session.closed = YES;
            session.active = NO;
            [session.events removeAllObjects];
        }
        for (NSArray *entry in session.registrations) {
            MPRemoteCommand *command = entry[0];
            [command removeTarget:entry[1]];
            command.enabled = NO;
        }
        [session.registrations removeAllObjects];
#if TARGET_OS_IPHONE
        for (id token in session.observers) [NSNotificationCenter.defaultCenter removeObserver:token];
        [session.observers removeAllObjects];
        if (session.audioActive) [AVAudioSession.sharedInstance setActive:NO
            withOptions:AVAudioSessionSetActiveOptionNotifyOthersOnDeactivation error:nil];
#endif
        MPNowPlayingInfoCenter.defaultCenter.nowPlayingInfo = nil;
#if !TARGET_OS_IPHONE
        MPNowPlayingInfoCenter.defaultCenter.playbackState = MPNowPlayingPlaybackStateStopped;
#endif
    }
}
