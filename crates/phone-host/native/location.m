#import <Foundation/Foundation.h>
#import <CoreLocation/CoreLocation.h>
#import <dispatch/dispatch.h>
#include <stdio.h>
#include <math.h>

@interface WPPositionRequest : NSObject <CLLocationManagerDelegate>
@property(nonatomic, strong) CLLocationManager *manager;
@property(nonatomic, strong) dispatch_semaphore_t ready;
@property(nonatomic, strong) CLLocation *position;
@property(nonatomic, copy) NSString *error;
@property(nonatomic) BOOL finished;
@end
@implementation WPPositionRequest
- (void)complete:(CLLocation *)position error:(NSString *)error {
    if (self.finished) return;
    self.finished = YES;
    self.position = position;
    self.error = error;
    [self.manager stopUpdatingLocation];
    self.manager.delegate = nil;
    dispatch_semaphore_signal(self.ready);
}
- (void)locationManager:(CLLocationManager *)manager didUpdateLocations:(NSArray<CLLocation *> *)locations {
    CLLocation *position = locations.lastObject;
    if (position && position.horizontalAccuracy >= 0 && fabs(position.timestamp.timeIntervalSinceNow) < 60)
        [self complete:position error:nil];
}
- (void)locationManager:(CLLocationManager *)manager didFailWithError:(NSError *)error {
    if (error.code == kCLErrorLocationUnknown) return;
    [self complete:nil error:error.localizedDescription];
}
- (void)locationManagerDidChangeAuthorization:(CLLocationManager *)manager {
    if (manager.authorizationStatus == kCLAuthorizationStatusDenied || manager.authorizationStatus == kCLAuthorizationStatusRestricted)
        [self complete:nil error:@"Location access was not allowed by macOS."];
}
@end

int wp_locate(double *latitude, double *longitude, double *accuracy, char *error, size_t length) {
    @autoreleasepool {
        if (length) error[0] = 0;
        if (!NSBundle.mainBundle.bundleIdentifier.length) {
            if (length) snprintf(error, length, "Run the packaged Windows Phone application to request location access.");
            return 0;
        }
        WPPositionRequest *request = [WPPositionRequest new];
        request.ready = dispatch_semaphore_create(0);
        // CLLocationManager and its delegate live on the normal app run loop.
        dispatch_async(dispatch_get_main_queue(), ^{
            request.manager = [CLLocationManager new];
            request.manager.delegate = request;
            request.manager.desiredAccuracy = kCLLocationAccuracyHundredMeters;
            [request.manager startUpdatingLocation];
        });
        long timed_out = dispatch_semaphore_wait(request.ready, dispatch_time(DISPATCH_TIME_NOW, 30 * NSEC_PER_SEC));
        if (timed_out) {
            dispatch_async(dispatch_get_main_queue(), ^{
                request.finished = YES;
                [request.manager stopUpdatingLocation];
                request.manager.delegate = nil;
            });
            if (length) snprintf(error, length, "Location acquisition timed out. Check your device permission and provider.");
            return 0;
        }
        if (!request.position) {
            if (length) snprintf(error, length, "%s", (request.error ?: @"The device did not return a position.").UTF8String);
            return 0;
        }
        *latitude = request.position.coordinate.latitude;
        *longitude = request.position.coordinate.longitude;
        *accuracy = request.position.horizontalAccuracy;
        return 1;
    }
}
