#import <AppKit/AppKit.h>
#import <UserNotifications/UserNotifications.h>
#import <dispatch/dispatch.h>
#include <stdio.h>

// UserNotifications requires an application bundle with an identifier. A plain
// cargo-run executable reports unsupported instead of entering an ObjC exception.
static UNUserNotificationCenter *center(void) {
    if (NSBundle.mainBundle.bundleIdentifier.length == 0) return nil;
    return UNUserNotificationCenter.currentNotificationCenter;
}
static int failure(char *output, size_t length, NSString *message) {
    if (length) snprintf(output, length, "%s", (message ?: @"Notification request failed").UTF8String);
    return -1;
}
@interface WPNotificationDelegate : NSObject <UNUserNotificationCenterDelegate>
@end
@implementation WPNotificationDelegate
- (void)userNotificationCenter:(UNUserNotificationCenter *)center
      willPresentNotification:(UNNotification *)notification
        withCompletionHandler:(void (^)(UNNotificationPresentationOptions))complete {
    complete(UNNotificationPresentationOptionBanner | UNNotificationPresentationOptionList);
}
- (void)userNotificationCenter:(UNUserNotificationCenter *)center
 didReceiveNotificationResponse:(UNNotificationResponse *)response
        withCompletionHandler:(void (^)(void))complete {
    dispatch_async(dispatch_get_main_queue(), ^{ [NSApp activateIgnoringOtherApps:YES]; });
    complete();
}
@end
static void install_delegate(UNUserNotificationCenter *notificationCenter) {
    static WPNotificationDelegate *delegate;
    static dispatch_once_t once;
    dispatch_once(&once, ^{ delegate = [WPNotificationDelegate new]; });
    notificationCenter.delegate = delegate;
}

int wp_notification_permission(int request, char *error, size_t length) {
    @autoreleasepool {
        if (length) error[0] = 0;
        UNUserNotificationCenter *notifications = center();
        if (!notifications) return 0;
        install_delegate(notifications);
        dispatch_semaphore_t ready = dispatch_semaphore_create(0);
        __block NSInteger status = -1;
        __block NSString *message;
        if (request) {
            [notifications requestAuthorizationWithOptions:UNAuthorizationOptionAlert
                completionHandler:^(BOOL granted, NSError *error) {
                    status = granted ? 2 : 3;
                    message = error.localizedDescription;
                    dispatch_semaphore_signal(ready);
                }];
        } else {
            [notifications getNotificationSettingsWithCompletionHandler:^(UNNotificationSettings *settings) {
                switch (settings.authorizationStatus) {
                    case UNAuthorizationStatusNotDetermined: status = 1; break;
                    case UNAuthorizationStatusDenied: status = 3; break;
                    case UNAuthorizationStatusAuthorized:
                    case UNAuthorizationStatusProvisional: status = 2; break;
                    default: status = -1; break;
                }
                dispatch_semaphore_signal(ready);
            }];
        }
        // Blocks retain their own semaphore/state beyond a timeout; no Rust
        // stack address is captured by an asynchronous completion.
        if (dispatch_semaphore_wait(ready, dispatch_time(DISPATCH_TIME_NOW, 30 * NSEC_PER_SEC)))
            return failure(error, length, @"Notification permission is still pending. Refresh device information after responding.");
        if (message) return failure(error, length, message);
        return (int)status;
    }
}

int wp_notification_show(const char *title, const char *body, char *error, size_t length) {
    @autoreleasepool {
        if (length) error[0] = 0;
        UNUserNotificationCenter *notifications = center();
        if (!notifications) return failure(error, length, @"Run the packaged Windows Phone application to use device notifications.");
        install_delegate(notifications);
        UNMutableNotificationContent *content = [UNMutableNotificationContent new];
        content.title = [NSString stringWithUTF8String:title];
        content.body = [NSString stringWithUTF8String:body];
        // Foreground alarm playback owns sound; never double-play a system tone.
        content.sound = nil;
        UNNotificationRequest *request = [UNNotificationRequest requestWithIdentifier:NSUUID.UUID.UUIDString content:content trigger:nil];
        dispatch_semaphore_t ready = dispatch_semaphore_create(0);
        __block NSString *message;
        [notifications addNotificationRequest:request withCompletionHandler:^(NSError *error) {
            message = error.localizedDescription;
            dispatch_semaphore_signal(ready);
        }];
        if (dispatch_semaphore_wait(ready, dispatch_time(DISPATCH_TIME_NOW, 5 * NSEC_PER_SEC)))
            return failure(error, length, @"The system notification request timed out.");
        return message ? failure(error, length, message) : 1;
    }
}
