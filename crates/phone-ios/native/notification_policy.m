#import "device.h"
#import <UserNotifications/UserNotifications.h>

static NSString *policyKey = @"FerroUIPhoneCarrierNotificationPolicy";

uint32_t wp_ios_notification_policy(void) {
    // No stored host preference means no autonomous notification. The shared
    // runtime supplies the real phone settings at first attach and on changes.
    return [NSUserDefaults.standardUserDefaults objectForKey:policyKey]
        ? ((uint32_t)[NSUserDefaults.standardUserDefaults integerForKey:policyKey] & 63u) : 0;
}

static BOOL carrierContent(UNNotificationContent *content) {
    id thread = content.userInfo[@"ferrouiCarrierThread"];
    return [thread isKindOfClass:NSString.class] && [thread hasPrefix:@"ios:"];
}

void wp_ios_set_notification_policy(uint32_t policy) {
    NSCAssert(NSThread.isMainThread, @"Phone notification policy belongs to the UIKit thread");
    policy &= 63u;
    uint32_t previous = wp_ios_notification_policy();
    if (previous == policy) return;
    [NSUserDefaults.standardUserDefaults setInteger:policy forKey:policyKey];
    BOOL disabled = !(policy & 1u) || !(policy & 2u);
    BOOL hidePreview = (previous & 32u) && !(policy & 32u);
    if (!disabled && !hidePreview) return;
    UNUserNotificationCenter *center = UNUserNotificationCenter.currentNotificationCenter;
    [center getDeliveredNotificationsWithCompletionHandler:^(NSArray<UNNotification *> *values) {
        NSMutableArray *identifiers = [NSMutableArray new];
        for (UNNotification *value in values)
            if (carrierContent(value.request.content)) [identifiers addObject:value.request.identifier];
        [center removeDeliveredNotificationsWithIdentifiers:identifiers];
    }];
    [center getPendingNotificationRequestsWithCompletionHandler:^(NSArray<UNNotificationRequest *> *values) {
        NSMutableArray *identifiers = [NSMutableArray new];
        for (UNNotificationRequest *value in values)
            if (carrierContent(value.content)) [identifiers addObject:value.identifier];
        [center removePendingNotificationRequestsWithIdentifiers:identifiers];
    }];
}
