#import "device.h"
#import <CoreLocation/CoreLocation.h>
#include <math.h>

// Device snapshots must never synchronously query locationd during UIKit
// startup. A manager reports its initial authorization asynchronously, then
// reports changes made in Settings through the same delegate. Observing this
// state does not request authorization or start location updates.
@interface WPIOSLocationAuthorization : NSObject <CLLocationManagerDelegate>
@property(nonatomic, strong) CLLocationManager *manager;
@end

static WPIOSLocationAuthorization *authorizationObserver;
static NSString *observedAuthorization;
static BOOL observingAuthorization;
static NSUInteger authorizationGeneration;

static void observeAuthorization(CLAuthorizationStatus status) {
    if (!observingAuthorization) return;
    NSString *value;
    switch (status) {
        case kCLAuthorizationStatusNotDetermined: value = @"prompt"; break;
        case kCLAuthorizationStatusAuthorizedAlways:
        case kCLAuthorizationStatusAuthorizedWhenInUse: value = @"granted"; break;
        case kCLAuthorizationStatusRestricted: value = @"restricted"; break;
        case kCLAuthorizationStatusDenied: value = @"denied"; break;
        default: value = @"unknown"; break;
    }
    if (![observedAuthorization isEqualToString:value]) {
        observedAuthorization = value;
        wp_ios_device_dirty();
    }
}

@implementation WPIOSLocationAuthorization
- (void)locationManagerDidChangeAuthorization:(CLLocationManager *)manager {
    if (!observingAuthorization || self != authorizationObserver || manager != self.manager) return;
    observeAuthorization(manager.authorizationStatus);
}
@end

NSString *wp_ios_location_authorization(void) {
    NSCAssert(NSThread.isMainThread, @"Location observations require the UIKit thread");
    return observedAuthorization ?: @"unknown";
}

void wp_ios_location_monitor(int enabled) {
    NSCAssert(NSThread.isMainThread, @"Location observation lifetime requires the UIKit thread");
    if (!enabled) {
        observingAuthorization = NO;
        ++authorizationGeneration;
        authorizationObserver.manager.delegate = nil;
        authorizationObserver = nil;
        observedAuthorization = nil;
        return;
    }
    if (observingAuthorization) return;
    observingAuthorization = YES;
    NSUInteger generation = ++authorizationGeneration;
    // Let framework initialization finish before constructing an SDK manager.
    // A suspend/close before this block runs invalidates the pending creation.
    dispatch_async(dispatch_get_main_queue(), ^{
        if (!observingAuthorization || generation != authorizationGeneration) return;
        WPIOSLocationAuthorization *observer = [WPIOSLocationAuthorization new];
        observer.manager = [CLLocationManager new];
        authorizationObserver = observer;
        observer.manager.delegate = observer;
    });
}

@interface WPIOSLocation : NSObject <CLLocationManagerDelegate>
@property(nonatomic, strong) CLLocationManager *manager;
@property(nonatomic, weak) WPIOSOperation *operation;
@property(nonatomic) BOOL authorizationRequested;
@property(nonatomic) BOOL requested;
@property(nonatomic) BOOL permissionOnly;
- (void)authorized;
@end
@implementation WPIOSLocation
- (void)authorized {
    if (!self.operation || self.operation.finished) return;
    CLAuthorizationStatus status = self.manager.authorizationStatus;
    observeAuthorization(status);
    if (status == kCLAuthorizationStatusNotDetermined) {
        if (!self.authorizationRequested) {
            self.authorizationRequested = YES;
            [self.manager requestWhenInUseAuthorization];
        }
        return;
    }
    if (self.permissionOnly) {
        wp_ios_device_finish(self.operation, @"snapshot", wp_ios_device_snapshot()); return;
    }
    if (status == kCLAuthorizationStatusDenied || status == kCLAuthorizationStatusRestricted) {
        [self.operation finish:0 first:0 second:0 message:@"Location access is blocked in iOS settings."];
    } else if ((status == kCLAuthorizationStatusAuthorizedWhenInUse || status == kCLAuthorizationStatusAuthorizedAlways) && !self.requested) {
        self.requested = YES;
        [self.manager requestLocation];
    }
}
- (void)locationManagerDidChangeAuthorization:(CLLocationManager *)manager { [self authorized]; }
- (void)locationManager:(CLLocationManager *)manager didFailWithError:(NSError *)error {
    [self.operation finish:0 first:0 second:0 message:error.localizedDescription];
}
- (void)locationManager:(CLLocationManager *)manager didUpdateLocations:(NSArray<CLLocation *> *)locations {
    CLLocation *position = locations.lastObject;
    CLLocationCoordinate2D coordinate = position.coordinate;
    if (!position || !CLLocationCoordinate2DIsValid(coordinate) ||
        !isfinite(coordinate.latitude) || !isfinite(coordinate.longitude) ||
        position.horizontalAccuracy < 0 || fabs(position.timestamp.timeIntervalSinceNow) > 60) {
        [self.operation finish:0 first:0 second:0 message:@"iOS did not return a recent valid location."];
        return;
    }
    [self.operation finish:3 first:coordinate.latitude second:coordinate.longitude message:@"Location acquired."];
}
@end

static void beginLocation(WPIOSOperation *operation, BOOL permissionOnly) {
    if (UIApplication.sharedApplication.applicationState != UIApplicationStateActive) {
        [operation finish:0 first:0 second:0 message:@"Open the app before requesting location access."];
        return;
    }
    if (![NSBundle.mainBundle objectForInfoDictionaryKey:@"NSLocationWhenInUseUsageDescription"]) {
        [operation finish:0 first:0 second:0 message:@"The app package does not declare location usage."];
        return;
    }
    WPIOSLocation *request = [WPIOSLocation new];
    request.operation = operation;
    request.permissionOnly = permissionOnly;
    request.manager = [CLLocationManager new];
    request.manager.desiredAccuracy = kCLLocationAccuracyHundredMeters;
    // The operation retains the delegate through cancellation/completion only.
    operation.cancelled = ^{
        [request.manager stopUpdatingLocation];
        request.manager.delegate = nil;
    };
    // The initial authorization callback drives the explicit request. Querying
    // the deprecated class status or locationServicesEnabled here can block the
    // UI thread waiting for locationd; disabled services report denied through
    // the delegate. The owning operation retains its existing timeout/cancel.
    request.manager.delegate = request;
}

void wp_ios_location_permission(WPIOSOperation *operation) { beginLocation(operation, YES); }

int wp_ios_locate(uint64_t identifier, WPIOSCompletion callback) {
    WPIOSOperation *operation = wp_ios_operation(identifier, callback, 30);
    if (!operation) return 0;
    beginLocation(operation, NO);
    return 1;
}
