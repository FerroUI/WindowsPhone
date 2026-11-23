#import <AppKit/AppKit.h>
#import <MediaPlayer/MediaPlayer.h>
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

void *wp_media_create(char *error, size_t length) {
    @autoreleasepool {
        if (length) error[0] = 0;
        if (!NSThread.isMainThread || NSBundle.mainBundle.bundleIdentifier.length == 0) {
            if (length) snprintf(error, length, "%s", "Run the packaged Windows Phone application to use macOS media controls.");
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
            if (length) snprintf(error, length, "%s", "macOS did not register all media command handlers.");
            return NULL;
        }
        return (__bridge_retained void *)session;
    }
}

void wp_media_publish(void *handle, const WPMediaSnapshot *snapshot) {
    @autoreleasepool {
        WPPhoneMediaSession *session = (__bridge WPPhoneMediaSession *)handle;
        if (!session || !snapshot) return;
        @synchronized (session) {
            if (session.closed) return;
            if (session.generation != snapshot->generation || !snapshot->active) [session.events removeAllObjects];
            session.active = snapshot->active != 0;
            session.generation = snapshot->generation;
            session.seekable = snapshot->seekable != 0;
            session.canNext = snapshot->can_next != 0;
            session.canPrevious = snapshot->can_previous != 0;
            session.duration = snapshot->duration;
        }
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
            info.playbackState = MPNowPlayingPlaybackStateStopped;
            return;
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
        info.nowPlayingInfo = metadata;
        info.playbackState = snapshot->state == 2 ? MPNowPlayingPlaybackStatePlaying :
            snapshot->state == 1 ? MPNowPlayingPlaybackStatePaused : MPNowPlayingPlaybackStateStopped;
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
        MPNowPlayingInfoCenter.defaultCenter.nowPlayingInfo = nil;
        MPNowPlayingInfoCenter.defaultCenter.playbackState = MPNowPlayingPlaybackStateStopped;
    }
}
