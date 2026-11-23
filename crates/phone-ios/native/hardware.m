#import "device.h"
#import <AVFoundation/AVFoundation.h>
#import <AVKit/AVKit.h>
#import <MediaPlayer/MediaPlayer.h>
#import <TargetConditionals.h>
#include <math.h>

static BOOL ownsTorch;

static AVCaptureDevice *torchDevice(void) {
    return [AVCaptureDevice defaultDeviceWithDeviceType:AVCaptureDeviceTypeBuiltInWideAngleCamera
        mediaType:AVMediaTypeVideo position:AVCaptureDevicePositionBack];
}

static UIScreen *activeScreen(void) {
    UIViewController *presenter = wp_ios_presenter();
    return presenter.view.window.screen ?: UIScreen.mainScreen;
}

static BOOL brightnessWritable(void) {
    // The simulator has no physical backlight. UIKit also restricts this
    // property to the main display, not an external scene or iOS-on-Mac host.
    return !TARGET_OS_SIMULATOR && !NSProcessInfo.processInfo.isiOSAppOnMac &&
        activeScreen() == UIScreen.mainScreen;
}

static NSDictionary *control(NSString *name, NSString *access, NSString *kind, id value, NSString *detail) {
    NSMutableDictionary *observed = [NSMutableDictionary dictionaryWithObject:kind forKey:@"kind"];
    if (value) observed[@"value"] = value;
    return @{@"control":name, @"access":access, @"value":observed, @"detail":detail};
}

NSArray *wp_ios_hardware_controls(void) {
    AVCaptureDevice *torch = torchDevice();
    BOOL permitted = [AVCaptureDevice authorizationStatusForMediaType:AVMediaTypeVideo] == AVAuthorizationStatusAuthorized;
    BOOL supported = torch && torch.hasTorch;
    NSString *torchAccess = !supported ? @"unavailable" : !permitted ? @"permission" :
        torch.isTorchAvailable ? @"writable" : @"readOnly";
    NSMutableArray *names = [NSMutableArray new];
    for (AVAudioSessionPortDescription *port in AVAudioSession.sharedInstance.currentRoute.outputs)
        [names addObject:port.portName];
    NSString *route = wp_ios_text([names componentsJoinedByString:@", "], 512);
    return @[
        control(@"brightness", brightnessWritable() ? @"writable" : @"readOnly", @"level", @(activeScreen().brightness),
            TARGET_OS_SIMULATOR ? @"The simulator reports a brightness value but has no writable physical backlight." :
            brightnessWritable() ? @"Main-screen brightness is controlled by UIKit and remains effective until the device locks." :
            @"The active scene does not expose a supported main-screen brightness control."),
        control(@"flashlight", torchAccess, supported ? @"toggle" : @"unknown", supported ? @(torch.isTorchActive) : nil,
            !supported ? @"This device has no rear torch." : !permitted ? @"Camera permission is required to use the torch." :
            @"The torch is released when the app suspends. Camera use or thermal conditions can make it unavailable."),
        control(@"mediaVolume", @"panel", @"level", @(AVAudioSession.sharedInstance.outputVolume),
            @"Use the native volume panel. iOS does not provide a general writable system volume property."),
        control(@"audioOutput", @"panel", route.length ? @"choice" : @"unknown", route.length ? route : nil,
            @"Choose an available output in the system route picker."),
        control(@"cast", @"panel", route.length ? @"choice" : @"unknown", route.length ? route : nil,
            @"The native route picker chooses compatible audio/video destinations for the app; it does not enable system screen mirroring."),
        control(@"batterySaver", @"readOnly", @"toggle", @(NSProcessInfo.processInfo.lowPowerModeEnabled),
            @"Low Power Mode is observed from iOS. Change it in the system Settings app.")
    ];
}

void wp_ios_hardware_set(WPIOSOperation *operation, NSDictionary *request) {
    if (UIApplication.sharedApplication.applicationState != UIApplicationStateActive) {
        [operation finish:0 first:0 second:0 message:@"Open the app before changing a device control."]; return;
    }
    NSString *name = request[@"control"];
    NSDictionary *value = request[@"value"];
    if ([name isEqualToString:@"brightness"] && [value[@"kind"] isEqualToString:@"level"]) {
        if (!brightnessWritable()) {
            [operation finish:0 first:0 second:0 message:TARGET_OS_SIMULATOR ?
                @"Screen brightness is read-only in iOS Simulator." :
                @"This scene does not have a writable physical main-screen brightness control."];
            return;
        }
        double level = [value[@"value"] doubleValue];
        if (!isfinite(level) || level < 0 || level > 1) {
            [operation finish:0 first:0 second:0 message:@"Brightness must be between zero and one."]; return;
        }
        activeScreen().brightness = level;
        wp_ios_device_finish(operation, @"snapshot", wp_ios_device_snapshot()); return;
    }
    if ([name isEqualToString:@"flashlight"] && [value[@"kind"] isEqualToString:@"toggle"]) {
        AVCaptureDevice *device = torchDevice();
        if (!device.hasTorch || !device.isTorchAvailable) {
            [operation finish:0 first:0 second:0 message:@"The torch is unavailable on this device right now."]; return;
        }
        if ([AVCaptureDevice authorizationStatusForMediaType:AVMediaTypeVideo] != AVAuthorizationStatusAuthorized) {
            [operation finish:0 first:0 second:0 message:@"Allow camera access before changing the torch."]; return;
        }
        NSError *error;
        if (![device lockForConfiguration:&error]) {
            [operation finish:0 first:0 second:0 message:error.localizedDescription]; return;
        }
        BOOL enabled = [value[@"value"] boolValue];
        BOOL success = YES;
        if (enabled) success = [device setTorchModeOnWithLevel:AVCaptureMaxAvailableTorchLevel error:&error];
        else device.torchMode = AVCaptureTorchModeOff;
        [device unlockForConfiguration];
        if (!success) [operation finish:0 first:0 second:0 message:error.localizedDescription];
        else { ownsTorch = enabled; wp_ios_device_finish(operation, @"snapshot", wp_ios_device_snapshot()); }
        return;
    }
    [operation finish:0 first:0 second:0 message:@"iOS does not allow the app to write this system control. Use its supported native panel or system Settings."];
}

@interface WPIOSAudioPanel : UIViewController <UIAdaptivePresentationControllerDelegate>
@property(nonatomic, weak) WPIOSOperation *operation;
@property(nonatomic) BOOL video;
@end
@implementation WPIOSAudioPanel
- (void)viewDidLoad {
    [super viewDidLoad];
    self.title = @"Audio output and volume";
    self.view.backgroundColor = UIColor.systemBackgroundColor;
    self.navigationItem.rightBarButtonItem = [[UIBarButtonItem alloc] initWithBarButtonSystemItem:UIBarButtonSystemItemDone
        target:self action:@selector(done)];
    UIStackView *stack = [UIStackView new]; stack.axis = UILayoutConstraintAxisVertical;
    stack.spacing = 24; stack.alignment = UIStackViewAlignmentFill; stack.translatesAutoresizingMaskIntoConstraints = NO;
    UILabel *label = [UILabel new]; label.text = @"Adjust playback volume or choose an available AirPlay output.";
    label.numberOfLines = 0; label.font = [UIFont preferredFontForTextStyle:UIFontTextStyleBody];
    MPVolumeView *volume = [MPVolumeView new]; volume.showsRouteButton = NO;
    AVRoutePickerView *route = [AVRoutePickerView new]; route.prioritizesVideoDevices = self.video;
    route.accessibilityLabel = @"Choose playback output";
    [stack addArrangedSubview:label]; [stack addArrangedSubview:volume]; [stack addArrangedSubview:route];
    [self.view addSubview:stack];
    [NSLayoutConstraint activateConstraints:@[
        [stack.leadingAnchor constraintEqualToAnchor:self.view.safeAreaLayoutGuide.leadingAnchor constant:24],
        [stack.trailingAnchor constraintEqualToAnchor:self.view.safeAreaLayoutGuide.trailingAnchor constant:-24],
        [stack.topAnchor constraintEqualToAnchor:self.view.safeAreaLayoutGuide.topAnchor constant:32],
        [volume.heightAnchor constraintEqualToConstant:44], [route.heightAnchor constraintEqualToConstant:64]]];
}
- (void)done { wp_ios_device_finish(self.operation, @"snapshot", wp_ios_device_snapshot()); }
- (void)presentationControllerDidDismiss:(UIPresentationController *)presentationController { [self done]; }
@end

void wp_ios_hardware_open(WPIOSOperation *operation, NSString *name) {
    if ([name isEqualToString:@"flashlight"]) {
        wp_ios_device_open_url(operation, [NSURL URLWithString:UIApplicationOpenSettingsURLString], @"The app's camera permission settings opened."); return;
    }
    if (![name isEqualToString:@"mediaVolume"] && ![name isEqualToString:@"audioOutput"] && ![name isEqualToString:@"cast"]) {
        [operation finish:0 first:0 second:0 message:@"iOS does not publish a direct settings route for this control."]; return;
    }
    UIViewController *presenter = wp_ios_presenter();
    if (!presenter || presenter.isBeingPresented || presenter.isBeingDismissed) {
        [operation finish:0 first:0 second:0 message:@"The active app cannot present the audio controls right now."]; return;
    }
    WPIOSAudioPanel *panel = [WPIOSAudioPanel new]; panel.operation = operation; panel.video = [name isEqualToString:@"cast"];
    UINavigationController *sheet = [[UINavigationController alloc] initWithRootViewController:panel];
    sheet.modalPresentationStyle = UIModalPresentationPageSheet;
    operation.cancelled = ^{
        panel.operation = nil;
        sheet.presentationController.delegate = nil;
        if (sheet.presentingViewController) [sheet dismissViewControllerAnimated:YES completion:nil];
    };
    [presenter presentViewController:sheet animated:YES completion:nil];
    sheet.presentationController.delegate = panel;
}

void wp_ios_hardware_suspend(void) {
    if (!ownsTorch) return;
    ownsTorch = NO;
    AVCaptureDevice *device = torchDevice();
    NSError *error;
    if (device.hasTorch && [device lockForConfiguration:&error]) {
        device.torchMode = AVCaptureTorchModeOff;
        [device unlockForConfiguration];
    }
}
