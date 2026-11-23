#import "device.h"
#import <CoreLocation/CoreLocation.h>
#include <math.h>

@interface WPIOSLocation : NSObject <CLLocationManagerDelegate>
@property(nonatomic, strong) CLLocationManager *manager;
@property(nonatomic, weak) WPIOSOperation *operation;
@property(nonatomic) BOOL requested;
@property(nonatomic) BOOL permissionOnly;
- (void)authorized;
@end
@implementation WPIOSLocation
- (void)authorized {
    CLAuthorizationStatus status;
    if (@available(iOS 14, *)) status = self.manager.authorizationStatus;
    else status = CLLocationManager.authorizationStatus;
    if (self.permissionOnly && status != kCLAuthorizationStatusNotDetermined) {
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
- (void)locationManager:(CLLocationManager *)manager didChangeAuthorizationStatus:(CLAuthorizationStatus)status { [self authorized]; }
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
    if (!CLLocationManager.locationServicesEnabled) {
        [operation finish:0 first:0 second:0 message:@"Location services are disabled on this device."];
        return;
    }
    WPIOSLocation *request = [WPIOSLocation new];
    request.operation = operation;
    request.permissionOnly = permissionOnly;
    request.manager = [CLLocationManager new];
    request.manager.delegate = request;
    request.manager.desiredAccuracy = kCLLocationAccuracyHundredMeters;
    // The operation retains the delegate through cancellation/completion only.
    operation.cancelled = ^{
        [request.manager stopUpdatingLocation];
        request.manager.delegate = nil;
    };
    CLAuthorizationStatus status;
    if (@available(iOS 14, *)) status = request.manager.authorizationStatus;
    else status = CLLocationManager.authorizationStatus;
    if (status == kCLAuthorizationStatusNotDetermined) [request.manager requestWhenInUseAuthorization];
    else [request authorized];
}

void wp_ios_location_permission(WPIOSOperation *operation) { beginLocation(operation, YES); }

int wp_ios_locate(uint64_t identifier, WPIOSCompletion callback) {
    WPIOSOperation *operation = wp_ios_operation(identifier, callback, 30);
    if (!operation) return 0;
    beginLocation(operation, NO);
    return 1;
}
