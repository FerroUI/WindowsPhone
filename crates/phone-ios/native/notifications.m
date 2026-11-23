#import "bridge.h"
#import <UserNotifications/UserNotifications.h>

@interface WPIOSNotifications : NSObject <UNUserNotificationCenterDelegate>
@end
@implementation WPIOSNotifications
- (void)userNotificationCenter:(UNUserNotificationCenter *)center willPresentNotification:(UNNotification *)notification
    withCompletionHandler:(void (^)(UNNotificationPresentationOptions))complete {
    if (@available(iOS 14, *)) complete(UNNotificationPresentationOptionBanner | UNNotificationPresentationOptionList);
    else complete(UNNotificationPresentationOptionAlert);
}
@end

static UNUserNotificationCenter *notification_center(void) {
    if (!NSBundle.mainBundle.bundleIdentifier.length) return nil;
    UNUserNotificationCenter *center = UNUserNotificationCenter.currentNotificationCenter;
    static WPIOSNotifications *delegate;
    static dispatch_once_t once;
    dispatch_once(&once, ^{ delegate = [WPIOSNotifications new]; });
    if (!center.delegate) center.delegate = delegate;
    return center;
}

static void read_permission(UNUserNotificationCenter *center, WPIOSOperation *operation) {
    [center getNotificationSettingsWithCompletionHandler:^(UNNotificationSettings *settings) {
        int permission = 0;
        switch (settings.authorizationStatus) {
            case UNAuthorizationStatusNotDetermined: permission = 1; break;
            case UNAuthorizationStatusDenied: permission = 3; break;
            case UNAuthorizationStatusAuthorized:
            case UNAuthorizationStatusProvisional: permission = 2; break;
            default:
                if (@available(iOS 14, *)) {
                    if (settings.authorizationStatus == UNAuthorizationStatusEphemeral) permission = 2;
                }
                break;
        }
        dispatch_async(dispatch_get_main_queue(), ^{
            [operation finish:2 first:permission second:0 message:@"Notification permission refreshed."];
        });
    }];
}

int wp_ios_notification_permission(uint64_t identifier, WPIOSCompletion callback, int request) {
    WPIOSOperation *operation = wp_ios_operation(identifier, callback, 60);
    if (!operation) return 0;
    UNUserNotificationCenter *center = notification_center();
    if (!center) {
        [operation finish:0 first:0 second:0 message:@"The notification provider requires a packaged app."];
        return 1;
    }
    if (request) {
        if (UIApplication.sharedApplication.applicationState != UIApplicationStateActive) {
            [operation finish:0 first:0 second:0 message:@"Open the app before requesting notification permission."];
            return 1;
        }
        [center requestAuthorizationWithOptions:UNAuthorizationOptionAlert completionHandler:^(BOOL granted, NSError *error) {
            if (error) dispatch_async(dispatch_get_main_queue(), ^{
                [operation finish:0 first:0 second:0 message:error.localizedDescription];
            });
            else read_permission(center, operation);
        }];
    } else read_permission(center, operation);
    return 1;
}

int wp_ios_notify(uint64_t identifier, WPIOSCompletion callback, const char *title, const char *body) {
    WPIOSOperation *operation = wp_ios_operation(identifier, callback, 15);
    if (!operation) return 0;
    UNUserNotificationCenter *center = notification_center();
    if (!center) {
        [operation finish:0 first:0 second:0 message:@"The notification provider requires a packaged app."];
        return 1;
    }
    UNMutableNotificationContent *content = [UNMutableNotificationContent new];
    content.title = [NSString stringWithUTF8String:title];
    content.body = [NSString stringWithUTF8String:body];
    // Phone foreground alerts own sound; this is a silent OS banner.
    content.sound = nil;
    UNNotificationRequest *request = [UNNotificationRequest requestWithIdentifier:NSUUID.UUID.UUIDString content:content trigger:nil];
    [center addNotificationRequest:request withCompletionHandler:^(NSError *error) {
        dispatch_async(dispatch_get_main_queue(), ^{
            [operation finish:error ? 0 : 1 first:0 second:0
                message:error.localizedDescription ?: @"iOS accepted the notification request."];
        });
    }];
    return 1;
}
