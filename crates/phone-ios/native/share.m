#import "bridge.h"
#include <unistd.h>

static int present_items(WPIOSOperation *operation, NSArray *items) {
    UIViewController *presenter = wp_ios_presenter();
    if (!presenter || [presenter isKindOfClass:UIActivityViewController.class] || presenter.isBeingPresented) {
        [operation finish:0 first:0 second:0 message:@"The iOS share sheet is unavailable while another presentation is active."];
        return 1;
    }
    if (!items.count) {
        [operation finish:0 first:0 second:0 message:@"There is no content to share."];
        return 1;
    }
    UIActivityViewController *sheet = [[UIActivityViewController alloc] initWithActivityItems:items applicationActivities:nil];
    __weak UIActivityViewController *weakSheet = sheet;
    operation.cancelled = ^{
        UIActivityViewController *sheet = weakSheet;
        if (sheet.presentingViewController) [sheet dismissViewControllerAnimated:NO completion:nil];
    };
    __weak WPIOSOperation *weakOperation = operation;
    sheet.completionWithItemsHandler = ^(UIActivityType type, BOOL completed, NSArray *returned, NSError *error) {
        WPIOSOperation *operation = weakOperation;
        if (!operation) return;
        [operation finish:error ? 0 : 1 first:completed ? 1 : 0 second:0
            message:error.localizedDescription ?: (completed ? @"The selected share activity completed." : @"Sharing cancelled.")];
    };
    UIPopoverPresentationController *popover = sheet.popoverPresentationController;
    if (popover) {
        popover.sourceView = presenter.view;
        CGRect bounds = presenter.view.bounds;
        popover.sourceRect = CGRectMake(CGRectGetMidX(bounds), CGRectGetMaxY(bounds) - 1, 1, 1);
        popover.permittedArrowDirections = UIPopoverArrowDirectionAny;
    }
    [presenter presentViewController:sheet animated:YES completion:nil];
    return 1;
}

int wp_ios_share(uint64_t identifier, WPIOSCompletion callback, const char *text, const char *url) {
    WPIOSOperation *operation = wp_ios_operation(identifier, callback, 120);
    if (!operation) return 0;
    NSMutableArray *items = [NSMutableArray new];
    NSString *message = text ? [NSString stringWithUTF8String:text] : nil;
    NSString *address = url ? [NSString stringWithUTF8String:url] : nil;
    if (message.length) [items addObject:message];
    if (address.length) {
        NSURL *value = [NSURL URLWithString:address];
        if (value) [items addObject:value];
    }
    return present_items(operation, items);
}

int wp_ios_share_file(uint64_t identifier, WPIOSCompletion callback, const char *file_path) {
    WPIOSOperation *operation = wp_ios_operation(identifier, callback, 120);
    if (!operation) return 0;
    NSString *path = file_path ? [NSString stringWithUTF8String:file_path] : nil;
    char directory[4096];
    if (!wp_ios_data_path(directory, sizeof(directory))) {
        [operation finish:0 first:0 second:0 message:@"The app's media directory is unavailable."];
        return 1;
    }
    NSString *owner = [NSString stringWithUTF8String:directory];
    NSString *root = [owner stringByAppendingPathComponent:@"assets"];
    NSString *resolved = path.stringByResolvingSymlinksInPath;
    NSString *home = [NSHomeDirectory().stringByResolvingSymlinksInPath stringByAppendingString:@"/"];
    NSString *parent = path.stringByDeletingLastPathComponent;
    NSFileManager *manager = NSFileManager.defaultManager;
    NSDictionary *attributes = path ? [manager attributesOfItemAtPath:path error:nil] : nil;
    NSDictionary *parentAttributes = parent ? [manager attributesOfItemAtPath:parent error:nil] : nil;
    NSDictionary *ownerAttributes = [manager attributesOfItemAtPath:owner error:nil];
    NSString *name = path.lastPathComponent;
    NSRange assetName = [name rangeOfString:@"^[A-Za-z0-9][A-Za-z0-9_-]{0,127}\\.(jpg|jpeg|png|gif|webp|mp3|wav|flac|ogg|opus|m4a|aac|wma|mp4|m4v|mov|mkv|webm|avi|wmv|mpg|mpeg)$"
        options:NSRegularExpressionSearch];
    unsigned long long size = [attributes[NSFileSize] unsignedLongLongValue];
    // Only a validated host-owned asset may reach this native-only hook.
    // A public URI activation or platform request cannot supply a file path.
    if (!path.isAbsolutePath || ![resolved hasPrefix:home] ||
        ![resolved.stringByDeletingLastPathComponent isEqualToString:root.stringByResolvingSymlinksInPath] ||
        assetName.location != 0 || assetName.length != name.length ||
        ![attributes[NSFileType] isEqualToString:NSFileTypeRegular] ||
        [attributes[NSFileOwnerAccountID] unsignedIntValue] != getuid() ||
        ![parentAttributes[NSFileType] isEqualToString:NSFileTypeDirectory] ||
        ![ownerAttributes[NSFileType] isEqualToString:NSFileTypeDirectory] ||
        size == 0 || size > 128ULL * 1024 * 1024) {
        [operation finish:0 first:0 second:0 message:@"Only a regular app-owned media asset can be shared."];
        return 1;
    }
    NSURL *file = [NSURL fileURLWithPath:path isDirectory:NO];
    return present_items(operation, @[file]);
}
