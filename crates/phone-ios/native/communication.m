#import "device.h"
#import "attachments.h"
#import <MessageUI/MessageUI.h>
#import <MapKit/MapKit.h>
#import <UniformTypeIdentifiers/UniformTypeIdentifiers.h>
#include <math.h>

@interface WPIOSMessageComposer : NSObject <MFMessageComposeViewControllerDelegate, UIAdaptivePresentationControllerDelegate>
@property(nonatomic, weak) WPIOSOperation *operation;
@end
@implementation WPIOSMessageComposer
- (void)messageComposeViewController:(MFMessageComposeViewController *)controller didFinishWithResult:(MessageComposeResult)result {
    NSString *outcome = result == MessageComposeResultSent ? @"sent" :
        result == MessageComposeResultCancelled ? @"cancelled" : @"failed";
    // The shared draft still owns its leases after system composition. UIKit
    // owns the copied attachment bytes; only explicit discard/expiry/close
    // releases the caller's draft copies.
    wp_ios_device_finish(self.operation, @"messageComposed", @{@"outcome":outcome});
}
- (void)presentationControllerDidDismiss:(UIPresentationController *)controller {
    wp_ios_device_finish(self.operation, @"messageComposed", @{@"outcome":@"cancelled"});
}
@end

void wp_ios_compose_message(WPIOSOperation *operation, NSArray *recipients, NSString *body) {
    wp_ios_compose_multimedia(operation, @{@"recipients":recipients, @"body":body, @"subject":@"", @"attachments":@[]});
}

void wp_ios_compose_multimedia(WPIOSOperation *operation, NSDictionary *request) {
    if (![MFMessageComposeViewController canSendText]) {
        [operation finish:0 first:0 second:0 message:@"This device cannot present a text message composer. A configured messaging service is required."]; return;
    }
    UIViewController *presenter = wp_ios_presenter();
    if (!presenter || presenter.isBeingPresented || presenter.isBeingDismissed) {
        [operation finish:0 first:0 second:0 message:@"The active app cannot present a message composer right now."]; return;
    }
    MFMessageComposeViewController *controller = [MFMessageComposeViewController new];
    WPIOSMessageComposer *delegate = [WPIOSMessageComposer new]; delegate.operation = operation;
    NSArray *identifiers = request[@"attachments"] ?: @[];
    NSString *subject = request[@"subject"] ?: @"";
    if ((identifiers.count && ![MFMessageComposeViewController canSendAttachments]) ||
        (subject.length && ![MFMessageComposeViewController canSendSubject])) {
        [operation finish:0 first:0 second:0 message:@"The configured message composer cannot send these attachments or a subject."]; return;
    }
    NSUInteger total = 0;
    for (NSString *identifier in identifiers) {
        WPIOSAttachment *lease = wp_ios_attachment(identifier);
        UTType *type = [UTType typeWithMIMEType:lease.mime ?: @"application/octet-stream"];
        NSData *bytes = lease ? [NSData dataWithContentsOfURL:lease.url] : nil;
        total += bytes.length;
        if (!lease || !bytes || bytes.length != lease.bytes || total > 24 * 1024 * 1024 ||
            ![controller addAttachmentData:bytes typeIdentifier:type.identifier ?: UTTypeData.identifier filename:lease.name]) {
            [operation finish:0 first:0 second:0 message:@"A draft attachment expired or is not supported by the system message composer. Select the file again."]; return;
        }
    }
    controller.recipients = request[@"recipients"]; controller.body = request[@"body"];
    if (subject.length) controller.subject = subject;
    controller.messageComposeDelegate = delegate;
    operation.cancelled = ^{
        delegate.operation = nil; controller.messageComposeDelegate = nil;
        controller.presentationController.delegate = nil;
        if (controller.presentingViewController) [controller dismissViewControllerAnimated:YES completion:nil];
    };
    [presenter presentViewController:controller animated:YES completion:nil];
    controller.presentationController.delegate = delegate;
}

void wp_ios_dial(WPIOSOperation *operation, NSString *number, NSString *account) {
    if (account && account != (id)NSNull.null && account.length) {
        [operation finish:0 first:0 second:0 message:@"A system calling URL cannot select a SIM. Use the entitled iOS 26 dialer provider for explicit cellular account selection."]; return;
    }
    NSMutableCharacterSet *allowed = [NSCharacterSet characterSetWithCharactersInString:@"+0123456789*#(),;"];
    NSString *compact = [[number componentsSeparatedByCharactersInSet:[NSCharacterSet characterSetWithCharactersInString:@" -."]] componentsJoinedByString:@""];
    NSString *encoded = [compact stringByAddingPercentEncodingWithAllowedCharacters:allowed];
    // '#' must be escaped; otherwise NSURL treats it as a fragment. Never allow
    // caller-controlled schemes, queries, or a selected default-app loop.
    encoded = [encoded stringByReplacingOccurrencesOfString:@"#" withString:@"%23"];
    NSURL *url = [NSURL URLWithString:[@"tel:" stringByAppendingString:encoded ?: @""]];
    if (![UIApplication.sharedApplication canOpenURL:url]) {
        [operation finish:0 first:0 second:0 message:@"No configured calling service is available on this device."]; return;
    }
    wp_ios_device_open_url(operation, url, @"The system accepted the calling handoff. Call connection is managed by the selected calling app.");
}

static NSString *directionsMode(NSString *mode) {
    if ([mode isEqualToString:@"walking"]) return MKLaunchOptionsDirectionsModeWalking;
    if ([mode isEqualToString:@"transit"]) return MKLaunchOptionsDirectionsModeTransit;
#if __IPHONE_OS_VERSION_MAX_ALLOWED >= 140000
    if ([mode isEqualToString:@"cycling"]) return MKLaunchOptionsDirectionsModeCycling;
#endif
    return MKLaunchOptionsDirectionsModeDriving;
}

static void openItem(WPIOSOperation *operation, MKMapItem *item, NSDictionary *request) {
    if (operation.finished) return;
    if (UIApplication.sharedApplication.applicationState != UIApplicationStateActive) {
        [operation finish:0 first:0 second:0 message:@"The app was suspended before the Maps handoff."]; return;
    }
    BOOL destination = [request[@"destination"] boolValue];
    NSDictionary *options = destination ? @{MKLaunchOptionsDirectionsModeKey:directionsMode(request[@"mode"])} : @{};
    BOOL success = [item openInMapsWithLaunchOptions:options];
    if (success) wp_ios_device_accept(operation, @"The native Maps app accepted the place or navigation request.");
    else [operation finish:0 first:0 second:0 message:@"The native Maps app could not open this location."];
}

void wp_ios_open_map(WPIOSOperation *operation, NSDictionary *request) {
    if (UIApplication.sharedApplication.applicationState != UIApplicationStateActive) {
        [operation finish:0 first:0 second:0 message:@"Open the app before starting Maps."]; return;
    }
    id latitude = request[@"latitude"], longitude = request[@"longitude"];
    if ([latitude isKindOfClass:NSNumber.class] && [longitude isKindOfClass:NSNumber.class]) {
        CLLocationCoordinate2D coordinate = CLLocationCoordinate2DMake([latitude doubleValue], [longitude doubleValue]);
        if (!CLLocationCoordinate2DIsValid(coordinate) || !isfinite(coordinate.latitude) || !isfinite(coordinate.longitude)) {
            [operation finish:0 first:0 second:0 message:@"Map coordinates are invalid."]; return;
        }
        MKMapItem *item = [[MKMapItem alloc] initWithPlacemark:[[MKPlacemark alloc] initWithCoordinate:coordinate]];
        NSString *name = request[@"query"]; if (name.length) item.name = name;
        openItem(operation, item, request); return;
    }
    MKLocalSearchRequest *searchRequest = [MKLocalSearchRequest new]; searchRequest.naturalLanguageQuery = request[@"query"];
    MKLocalSearch *search = [[MKLocalSearch alloc] initWithRequest:searchRequest];
    operation.cancelled = ^{ [search cancel]; };
    __weak WPIOSOperation *weak = operation;
    [search startWithCompletionHandler:^(MKLocalSearchResponse *response, NSError *error) {
        WPIOSOperation *current = weak;
        if (!current || current.finished) return;
        if (UIApplication.sharedApplication.applicationState != UIApplicationStateActive) {
            [current finish:0 first:0 second:0 message:@"The app was suspended before the Maps handoff."]; return;
        }
        if (error || !response.mapItems.count) {
            [current finish:0 first:0 second:0 message:error.localizedDescription ?: @"Maps found no matching place."]; return;
        }
        // Preserve the system's search order. The native Maps UI remains the
        // confirmation surface before the person starts route guidance.
        if ([request[@"destination"] boolValue]) openItem(current, response.mapItems.firstObject, request);
        else {
            BOOL success = [MKMapItem openMapsWithItems:response.mapItems launchOptions:@{}];
            if (success) wp_ios_device_accept(current, @"The native Maps app accepted the search results.");
            else [current finish:0 first:0 second:0 message:@"The native Maps app could not open the search results."];
        }
    }];
}
