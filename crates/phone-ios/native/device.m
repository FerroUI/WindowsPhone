#import "device.h"
#import "attachments.h"
#import <AVFoundation/AVFoundation.h>
#import <Contacts/Contacts.h>
#import <MessageUI/MessageUI.h>
#include <string.h>

static const NSUInteger maximumPayload = 8 * 1024 * 1024;
static NSMutableArray<NSData *> *events;
static NSMutableArray *observers;
static BOOL dirty;
static NSData *lastSnapshot;
static NSData *queuedSnapshot;

NSString *wp_ios_text(NSString *text, NSUInteger maximumBytes) {
    if (![text isKindOfClass:NSString.class]) return @"";
    NSString *clean = [[text componentsSeparatedByCharactersInSet:NSCharacterSet.controlCharacterSet]
        componentsJoinedByString:@" "];
    if ([clean lengthOfBytesUsingEncoding:NSUTF8StringEncoding] <= maximumBytes) return clean;
    NSMutableString *bounded = [NSMutableString new];
    __block NSUInteger length = 0;
    [clean enumerateSubstringsInRange:NSMakeRange(0, clean.length)
        options:NSStringEnumerationByComposedCharacterSequences
        usingBlock:^(NSString *substring, NSRange range, NSRange enclosing, BOOL *stop) {
            NSUInteger size = [substring lengthOfBytesUsingEncoding:NSUTF8StringEncoding];
            if (length + size > maximumBytes) { *stop = YES; return; }
            [bounded appendString:substring]; length += size;
        }];
    return bounded;
}

static NSData *encoded(NSString *kind, id value) {
    NSDictionary *response = @{@"kind":kind, @"value":value ?: NSNull.null};
    if (![NSJSONSerialization isValidJSONObject:response]) return nil;
    NSData *data = [NSJSONSerialization dataWithJSONObject:response options:NSJSONWritingSortedKeys error:nil];
    return data.length <= maximumPayload ? data : nil;
}

void wp_ios_device_finish(WPIOSOperation *operation, NSString *kind, id value) {
    if (!operation || operation.finished) return;
    NSData *data = encoded(kind, value);
    if (!data) {
        [operation finish:0 first:0 second:0 message:@"The device result exceeds the bounded native data contract."];
        return;
    }
    if ([kind isEqualToString:@"snapshot"]) wp_ios_device_emit(kind, value);
    [operation finish:4 first:0 second:0 message:[[NSString alloc] initWithData:data encoding:NSUTF8StringEncoding]];
}

void wp_ios_device_accept(WPIOSOperation *operation, NSString *message) {
    wp_ios_device_finish(operation, @"accepted", @{@"message":message});
}

void wp_ios_device_dirty(void) {
    if (NSThread.isMainThread) dirty = YES;
    else dispatch_async(dispatch_get_main_queue(), ^{ dirty = YES; });
}

void wp_ios_device_emit(NSString *kind, id value) {
    NSCAssert(NSThread.isMainThread, @"Device observations require the UIKit thread");
    NSData *data = encoded(kind, value);
    if (!data) return;
    if ([kind isEqualToString:@"snapshot"]) {
        if ([lastSnapshot isEqualToData:data]) return;
        lastSnapshot = data;
        queuedSnapshot = data;
        return;
    }
    if (!events) events = [NSMutableArray new];
    if (events.count >= 32) [events removeObjectAtIndex:0];
    [events addObject:data];
}

static NSString *cameraPermission(void) {
    switch ([AVCaptureDevice authorizationStatusForMediaType:AVMediaTypeVideo]) {
        case AVAuthorizationStatusNotDetermined: return @"prompt";
        case AVAuthorizationStatusAuthorized: return @"granted";
        case AVAuthorizationStatusRestricted: return @"restricted";
        default: return @"denied";
    }
}

NSDictionary *wp_ios_device_snapshot(void) {
    wp_ios_smoke_phase("snapshot.begin");
    NSMutableArray *roles = [NSMutableArray new];
    NSMutableArray *accounts = [NSMutableArray new];
    NSMutableArray *subscriptions = [NSMutableArray new];
    NSMutableArray *supported = [NSMutableArray new];
    wp_ios_smoke_phase("snapshot.contacts");
    NSString *contactAccess = wp_ios_contacts_permission();
    if ([contactAccess isEqualToString:@"granted"] || [contactAccess isEqualToString:@"limited"])
        [supported addObjectsFromArray:@[@"readContacts", @"writeContacts"]];
    wp_ios_smoke_phase("snapshot.tel");
    if ([UIApplication.sharedApplication canOpenURL:[NSURL URLWithString:@"tel:0"]]) [supported addObject:@"dial"];
    wp_ios_smoke_phase("snapshot.compose_text");
    if ([MFMessageComposeViewController canSendText]) [supported addObject:@"composeMessage"];
    wp_ios_smoke_phase("snapshot.compose_attachments");
    if ([MFMessageComposeViewController canSendAttachments]) [supported addObject:@"pickMessageAttachments"];
    wp_ios_smoke_phase("snapshot.maps");
    if ([UIApplication.sharedApplication canOpenURL:[NSURL URLWithString:@"maps://"]]) [supported addObject:@"maps"];
    wp_ios_smoke_phase("snapshot.permissions");
    wp_ios_smoke_phase("snapshot.permissions.contacts.begin");
    NSString *contactsPermission = wp_ios_contacts_permission();
    wp_ios_smoke_phase("snapshot.permissions.contacts.end");
    wp_ios_smoke_phase("snapshot.permissions.camera.begin");
    NSString *cameraAccess = cameraPermission();
    wp_ios_smoke_phase("snapshot.permissions.camera.end");
    wp_ios_smoke_phase("snapshot.permissions.location.begin");
    NSString *locationAccess = wp_ios_location_authorization();
    wp_ios_smoke_phase("snapshot.permissions.location.end");
    wp_ios_smoke_phase("snapshot.permissions.notifications.begin");
    NSString *notificationAccess = wp_ios_notifications_permission();
    wp_ios_smoke_phase("snapshot.permissions.notifications.end");
    NSMutableArray *permissions = [NSMutableArray arrayWithArray:@[
        @{@"permission":@"contacts", @"status":contactsPermission},
        @{@"permission":@"camera", @"status":cameraAccess},
        @{@"permission":@"location", @"status":locationAccess},
        @{@"permission":@"notifications", @"status":notificationAccess}]];
#if WP_IOS_TELEPHONY
    wp_ios_smoke_phase("snapshot.carrier");
    char *payload = wp_ios_telephony_snapshot();
    if (payload) {
        NSData *data = [NSData dataWithBytes:payload length:strlen(payload)];
        NSDictionary *telephony = [NSJSONSerialization JSONObjectWithData:data options:0 error:nil];
        free(payload);
        if ([telephony isKindOfClass:NSDictionary.class]) {
            if ([telephony[@"roles"] isKindOfClass:NSArray.class]) [roles addObjectsFromArray:telephony[@"roles"]];
            if ([telephony[@"phone_accounts"] isKindOfClass:NSArray.class]) [accounts addObjectsFromArray:telephony[@"phone_accounts"]];
            if ([telephony[@"sms_subscriptions"] isKindOfClass:NSArray.class]) [subscriptions addObjectsFromArray:telephony[@"sms_subscriptions"]];
            if ([telephony[@"permissions"] isKindOfClass:NSArray.class]) [permissions addObjectsFromArray:telephony[@"permissions"]];
            for (NSString *operation in telephony[@"operations"])
                if (![supported containsObject:operation]) [supported addObject:operation];
        }
    }
#else
    [roles addObjectsFromArray:@[
        @{@"role":@"dialer", @"status":@"unsupported", @"detail":@"This build uses the system calling handoff. The optional iOS 26 provider requires an approved Default Dialer entitlement."},
        @{@"role":@"sms", @"status":@"unsupported", @"detail":@"Message composition is supported. The optional carrier provider requires an approved entitlement, eligible region and selection as default."}]];
#endif
    NSMutableArray *settings = [NSMutableArray arrayWithArray:@[@"application", @"permissions", @"privacy", @"notifications", @"sound", @"audioOutput", @"cast"]];
#if __IPHONE_OS_VERSION_MAX_ALLOWED >= 180300
    if (@available(iOS 18.3, *)) [settings addObject:@"defaultApps"];
#endif
    wp_ios_smoke_phase("snapshot.hardware");
    NSDictionary *snapshot = @{@"controls":wp_ios_hardware_controls(), @"roles":roles, @"settings":settings,
        @"permissions":permissions, @"phone_accounts":accounts, @"sms_subscriptions":subscriptions, @"operations":supported,
        @"detail":@"Public UIKit device integrations. Contacts respect the selected access scope; message composition reports the actual system composer outcome. Device controls reflect observed hardware and permissions."};
    wp_ios_smoke_phase("snapshot.end");
    return snapshot;
}

void wp_ios_device_open_url(WPIOSOperation *operation, NSURL *url, NSString *success) {
    if (!url || UIApplication.sharedApplication.applicationState != UIApplicationStateActive) {
        [operation finish:0 first:0 second:0 message:@"Open the app before starting this device handoff."]; return;
    }
    __weak WPIOSOperation *weak = operation;
    [UIApplication.sharedApplication openURL:url options:@{} completionHandler:^(BOOL accepted) {
        WPIOSOperation *current = weak;
        if (!current || current.finished) return;
        if (accepted) wp_ios_device_accept(current, success);
        else [current finish:0 first:0 second:0 message:@"iOS could not open this device service."];
    }];
}

static void requestPermission(WPIOSOperation *operation, NSString *permission) {
    if (UIApplication.sharedApplication.applicationState != UIApplicationStateActive) {
        [operation finish:0 first:0 second:0 message:@"Open the app before requesting device permission."]; return;
    }
    if ([permission isEqualToString:@"contacts"]) { wp_ios_contacts_authorize(operation); return; }
    if ([permission isEqualToString:@"location"]) { wp_ios_location_permission(operation); return; }
    if ([permission isEqualToString:@"camera"]) {
        if (![NSBundle.mainBundle objectForInfoDictionaryKey:@"NSCameraUsageDescription"]) {
            [operation finish:0 first:0 second:0 message:@"The app package does not declare camera usage."]; return;
        }
        if ([AVCaptureDevice authorizationStatusForMediaType:AVMediaTypeVideo] != AVAuthorizationStatusNotDetermined) {
            wp_ios_device_finish(operation, @"snapshot", wp_ios_device_snapshot()); return;
        }
        __weak WPIOSOperation *weak = operation;
        [AVCaptureDevice requestAccessForMediaType:AVMediaTypeVideo completionHandler:^(BOOL granted) {
            dispatch_async(dispatch_get_main_queue(), ^{
                WPIOSOperation *current = weak;
                if (current && !current.finished) wp_ios_device_finish(current, @"snapshot", wp_ios_device_snapshot());
            });
        }];
        return;
    }
    [operation finish:0 first:0 second:0 message:@"This permission is controlled by the native service or its default-app role, not a general iOS permission prompt."];
}

int wp_ios_device(uint64_t identifier, WPIOSCompletion callback, const char *json) {
    if (!json || strlen(json) > 1024 * 1024) return 0;
    NSData *data = [NSData dataWithBytes:json length:strlen(json)];
    NSDictionary *request = [NSJSONSerialization JSONObjectWithData:data options:0 error:nil];
    if (![request isKindOfClass:NSDictionary.class]) return 0;
    NSString *kind = request[@"kind"];
    NSTimeInterval timeout = ([kind isEqualToString:@"composeMessage"] || [kind isEqualToString:@"composeMultimediaMessage"] ||
        [kind isEqualToString:@"pickMessageAttachments"] || [kind isEqualToString:@"openMessageAttachment"] || [kind isEqualToString:@"openControl"]) ? 300 :
        [kind isEqualToString:@"requestPermission"] ? 60 : 30;
    WPIOSOperation *operation = wp_ios_operation(identifier, callback, timeout);
    if (!operation) return 0;
    if ([kind isEqualToString:@"snapshot"]) wp_ios_device_finish(operation, @"snapshot", wp_ios_device_snapshot());
    else if ([kind isEqualToString:@"contacts"]) wp_ios_contacts_read(operation, request[@"query"], [request[@"limit"] unsignedIntegerValue]);
    else if ([kind isEqualToString:@"saveContact"]) wp_ios_contacts_save(operation, request[@"contact"]);
    else if ([kind isEqualToString:@"deleteContact"]) wp_ios_contacts_delete(operation, request[@"id"]);
    else if ([kind isEqualToString:@"requestPermission"]) requestPermission(operation, request[@"permission"]);
    else if ([kind isEqualToString:@"setControl"]) wp_ios_hardware_set(operation, request);
    else if ([kind isEqualToString:@"openControl"]) wp_ios_hardware_open(operation, request[@"control"]);
    else if ([kind isEqualToString:@"composeMessage"]) wp_ios_compose_message(operation, request[@"recipients"], request[@"body"]);
    else if ([kind isEqualToString:@"composeMultimediaMessage"]) wp_ios_compose_multimedia(operation, request);
    else if ([kind isEqualToString:@"pickMessageAttachments"]) wp_ios_pick_attachments(operation, request[@"client_id"]);
    else if ([kind isEqualToString:@"discardMessageAttachments"]) {
        wp_ios_discard_attachments(request[@"ids"]);
        wp_ios_device_accept(operation, @"The selected draft attachments were discarded.");
    }
    else if ([kind isEqualToString:@"openMap"]) wp_ios_open_map(operation, request);
    else {
#if WP_IOS_TELEPHONY
        // Swift checks the real signed entitlement and actual OS role before
        // touching the corresponding service. A declined command falls back.
        if (wp_ios_telephony_command(identifier, json)) {
            operation.cancelled = ^{ wp_ios_telephony_cancel(identifier); };
            return 1;
        }
#endif
        if ([kind isEqualToString:@"dial"]) wp_ios_dial(operation, request[@"number"], request[@"account"]);
        else [operation finish:0 first:0 second:0 message:@"This operation requires an available native provider and any applicable user-selected default-app role. This iOS build cannot perform it."];
    }
    return 1;
}

void wp_ios_device_monitor(int enabled) {
    if (!NSThread.isMainThread) return;
    wp_ios_location_monitor(enabled);
    if (enabled > 0) wp_ios_notifications_install();
#if WP_IOS_TELEPHONY
    wp_ios_telephony_monitor(enabled);
#endif
    if (!enabled) {
        for (id observer in observers) [NSNotificationCenter.defaultCenter removeObserver:observer];
        observers = nil;
        [events removeAllObjects];
        lastSnapshot = nil;
        queuedSnapshot = nil;
        dirty = NO;
        wp_ios_hardware_suspend();
        return;
    }
    dirty = YES;
    if (observers) return;
    observers = [NSMutableArray new];
    for (NSString *name in @[UIScreenBrightnessDidChangeNotification, NSProcessInfoPowerStateDidChangeNotification,
                            UIDeviceBatteryLevelDidChangeNotification, UIDeviceBatteryStateDidChangeNotification,
                            AVAudioSessionRouteChangeNotification, CNContactStoreDidChangeNotification]) {
        id observer = [NSNotificationCenter.defaultCenter addObserverForName:name object:nil queue:NSOperationQueue.mainQueue
            usingBlock:^(NSNotification *note) {
                dirty = YES;
                if ([note.name isEqualToString:CNContactStoreDidChangeNotification]) {
                    wp_ios_contacts_changed();
                    wp_ios_device_emit(@"contacts", @[]);
                }
            }];
        [observers addObject:observer];
    }
}

size_t wp_ios_device_poll(char *buffer, size_t capacity) {
    if (!NSThread.isMainThread) return 0;
    wp_ios_smoke_phase("device_poll.begin");
    wp_ios_attachments_prune();
    wp_ios_smoke_phase("device_poll.pruned");
    if (dirty) { dirty = NO; wp_ios_device_emit(@"snapshot", wp_ios_device_snapshot()); }
    wp_ios_smoke_phase("device_poll.encoded");
    NSData *data = queuedSnapshot ?: events.firstObject;
    if (!data) return 0;
    if (!buffer || capacity <= data.length) return data.length;
    memcpy(buffer, data.bytes, data.length); buffer[data.length] = 0;
    NSUInteger length = data.length;
    if (data == queuedSnapshot) queuedSnapshot = nil;
    else [events removeObjectAtIndex:0];
    return length;
}

#if WP_IOS_TELEPHONY
// Swift async tasks return only an identifier and owned JSON/text. Lookup occurs
// on the UIKit thread, so cancelled or closed operations cannot be resurrected.
void wp_ios_telephony_complete(uint64_t identifier, int success, const char *json) {
    wp_ios_operation_complete(identifier, success ? 4 : 0, json);
}

void wp_ios_telephony_event(const char *json) {
    if (!NSThread.isMainThread || !json || strlen(json) > maximumPayload) return;
    NSData *data = [NSData dataWithBytes:json length:strlen(json)];
    NSDictionary *value = [NSJSONSerialization JSONObjectWithData:data options:0 error:nil];
    if ([value isKindOfClass:NSDictionary.class] && [value[@"kind"] isKindOfClass:NSString.class])
        wp_ios_device_emit(value[@"kind"], value[@"value"]);
}
#endif

void wp_ios_device_shutdown(void) {
    wp_ios_device_monitor(0);
#if WP_IOS_TELEPHONY
    wp_ios_telephony_monitor(-1);
#endif
    wp_ios_attachments_close();
}
