#import "bridge.h"
#include <stdio.h>
#include <string.h>

static NSMutableDictionary<NSNumber *, WPIOSOperation *> *operations;

@implementation WPIOSOperation
- (void)finish:(int)code first:(double)first second:(double)second message:(NSString *)message {
    NSAssert(NSThread.isMainThread, @"Operation delivery requires the UIKit thread");
    if (self.finished) return;
    self.finished = YES;
    [self.timer invalidate];
    self.timer = nil;
    void (^cancel)(void) = self.cancelled;
    self.cancelled = nil;
    if (cancel) cancel();
    [operations removeObjectForKey:@(self.identifier)];
    // The Rust callback copies this short-lived UTF-8 string synchronously.
    self.completion(self.identifier, code, first, second, (message ?: @"").UTF8String);
}
@end

WPIOSOperation *wp_ios_operation(uint64_t identifier, WPIOSCompletion callback, NSTimeInterval timeout) {
    if (!NSThread.isMainThread || !identifier || !callback) return nil;
    if (!operations) operations = [NSMutableDictionary new];
    if (operations.count >= 16 || operations[@(identifier)]) return nil;
    WPIOSOperation *operation = [WPIOSOperation new];
    operation.identifier = identifier;
    operation.completion = callback;
    operations[@(identifier)] = operation;
    __weak WPIOSOperation *weak = operation;
    operation.timer = [NSTimer scheduledTimerWithTimeInterval:timeout repeats:NO block:^(NSTimer *timer) {
        [weak finish:0 first:0 second:0 message:@"The iOS operation timed out."];
    }];
    return operation;
}

void wp_ios_cancel(uint64_t identifier) {
    void (^cancel)(void) = ^{
        WPIOSOperation *operation = operations[@(identifier)];
        [operation finish:0 first:0 second:0 message:@"The iOS operation was cancelled."];
    };
    if (NSThread.isMainThread) cancel();
    else dispatch_async(dispatch_get_main_queue(), cancel);
}

void wp_ios_operation_complete(uint64_t identifier, int code, const char *message) {
    NSCAssert(NSThread.isMainThread, @"Operation delivery requires the UIKit thread");
    WPIOSOperation *operation = operations[@(identifier)];
    if (!operation || operation.finished) return;
    NSString *text = message ? [NSString stringWithUTF8String:message] : @"";
    [operation finish:code first:0 second:0 message:text];
}

WPIOSOperation *wp_ios_operation_lookup(uint64_t identifier) {
    return NSThread.isMainThread ? operations[@(identifier)] : nil;
}

UIViewController *wp_ios_presenter(void) {
    if (!NSThread.isMainThread) return nil;
    for (UIScene *scene in UIApplication.sharedApplication.connectedScenes) {
        if (scene.activationState != UISceneActivationStateForegroundActive ||
            ![scene isKindOfClass:UIWindowScene.class]) continue;
        for (UIWindow *window in ((UIWindowScene *)scene).windows) {
            if (!window.isKeyWindow) continue;
            UIViewController *controller = window.rootViewController;
            while (controller.presentedViewController && !controller.presentedViewController.isBeingDismissed)
                controller = controller.presentedViewController;
            return controller;
        }
    }
    return nil;
}

int wp_ios_settings(uint64_t identifier, WPIOSCompletion callback, int page) {
    WPIOSOperation *operation = wp_ios_operation(identifier, callback, 15);
    if (!operation) return 0;
    NSString *address = UIApplicationOpenSettingsURLString;
    if (page == 1) {
        [operation finish:0 first:0 second:0 message:@"iOS has no public API for opening network settings directly."];
        return 1;
    }
    if (page == 2) {
        if (@available(iOS 15.4, *)) address = UIApplicationOpenNotificationSettingsURLString;
    }
    if (page == 3) {
#if __IPHONE_OS_VERSION_MAX_ALLOWED >= 180300
        if (@available(iOS 18.3, *)) address = UIApplicationOpenDefaultApplicationsSettingsURLString;
        else {
            [operation finish:0 first:0 second:0 message:@"Default-app settings require iOS 18.3 or later."];
            return 1;
        }
#else
        [operation finish:0 first:0 second:0 message:@"This build does not expose the default-app settings API."];
        return 1;
#endif
    }
    if (!address.length) {
        [operation finish:0 first:0 second:0 message:@"This iOS version does not expose that settings page."];
        return 1;
    }
    [UIApplication.sharedApplication openURL:[NSURL URLWithString:address] options:@{}
        completionHandler:^(BOOL success) {
            [operation finish:success ? 1 : 0 first:0 second:0
                message:success ? @"Device settings opened." : @"iOS did not open device settings."];
        }];
    return 1;
}

int wp_ios_haptic(int kind) {
    if (!NSThread.isMainThread || UIApplication.sharedApplication.applicationState != UIApplicationStateActive) return 0;
    if (kind == 0) {
        UISelectionFeedbackGenerator *generator = [UISelectionFeedbackGenerator new];
        [generator prepare];
        [generator selectionChanged];
    } else {
        UIImpactFeedbackStyle style = kind == 3 ? UIImpactFeedbackStyleHeavy :
            kind == 2 ? UIImpactFeedbackStyleMedium : UIImpactFeedbackStyleLight;
        UIImpactFeedbackGenerator *generator = [[UIImpactFeedbackGenerator alloc] initWithStyle:style];
        [generator prepare];
        [generator impactOccurred];
    }
    return 1;
}

void wp_ios_observe(double *battery, int *charging, int *power_saver) {
    if (!NSThread.isMainThread) return;
    UIDevice *device = UIDevice.currentDevice;
    *battery = device.batteryLevel;
    *charging = device.batteryState == UIDeviceBatteryStateUnknown ? -1 :
        (device.batteryState == UIDeviceBatteryStateCharging || device.batteryState == UIDeviceBatteryStateFull);
    *power_saver = NSProcessInfo.processInfo.lowPowerModeEnabled;
}

void wp_ios_monitor_power(int enabled) {
    if (NSThread.isMainThread) UIDevice.currentDevice.batteryMonitoringEnabled = enabled != 0;
}

int wp_ios_data_path(char *buffer, size_t length) {
    if (!buffer || !length) return 0;
    buffer[0] = 0;
    NSError *error;
    NSURL *base = [NSFileManager.defaultManager URLForDirectory:NSApplicationSupportDirectory
        inDomain:NSUserDomainMask appropriateForURL:nil create:YES error:&error];
    if (!base) return 0;
    NSURL *folder = [base URLByAppendingPathComponent:@"FerroUIWindowsPhone" isDirectory:YES];
    if (![NSFileManager.defaultManager createDirectoryAtURL:folder withIntermediateDirectories:YES attributes:nil error:&error]) return 0;
    const char *path = folder.path.UTF8String;
    if (!path || strlen(path) >= length) return 0;
    snprintf(buffer, length, "%s", path);
    return 1;
}
