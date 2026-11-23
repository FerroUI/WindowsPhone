#import "attachments.h"
#import <UniformTypeIdentifiers/UniformTypeIdentifiers.h>
#include <string.h>

static NSMutableDictionary<NSString *, WPIOSAttachment *> *leases;
static NSURL *ownedDirectory;
static const NSUInteger attachmentLimit = 8 * 1024 * 1024;
static const NSUInteger selectionLimit = 24 * 1024 * 1024;

static void pickerResult(WPIOSOperation *operation, NSString *client, NSArray *attachments, NSString *error) {
    wp_ios_device_finish(operation, @"draftAttachments", @{@"client_id":client,
        @"attachments":attachments ?: NSNull.null, @"error":error ?: NSNull.null});
}

@implementation WPIOSAttachment
@end

static NSURL *directory(NSError **error) {
    if (ownedDirectory) return ownedDirectory;
    NSFileManager *manager = NSFileManager.defaultManager;
    NSURL *cache = [manager URLForDirectory:NSCachesDirectory inDomain:NSUserDomainMask appropriateForURL:nil create:YES error:error];
    NSURL *value = [[cache URLByAppendingPathComponent:@"PhoneMessageAttachments" isDirectory:YES]
        URLByAppendingPathComponent:NSUUID.UUID.UUIDString isDirectory:YES];
    if (![manager createDirectoryAtURL:value withIntermediateDirectories:YES attributes:@{
        NSFilePosixPermissions:@0700, NSFileProtectionKey:NSFileProtectionCompleteUntilFirstUserAuthentication} error:error]) return nil;
    ownedDirectory = value;
    return value;
}

void wp_ios_discard_attachments(NSArray<NSString *> *identifiers) {
    for (NSString *identifier in identifiers) {
        WPIOSAttachment *lease = leases[identifier];
        if (lease) [NSFileManager.defaultManager removeItemAtURL:lease.url error:nil];
        [leases removeObjectForKey:identifier];
    }
}

void wp_ios_attachments_prune(void) {
    NSTimeInterval now = NSProcessInfo.processInfo.systemUptime;
    NSMutableArray *expired = [NSMutableArray new];
    for (NSString *identifier in leases) if (leases[identifier].expires <= now) [expired addObject:identifier];
    wp_ios_discard_attachments(expired);
}

void wp_ios_attachments_close(void) {
    [leases removeAllObjects];
    if (ownedDirectory) [NSFileManager.defaultManager removeItemAtURL:ownedDirectory error:nil];
    ownedDirectory = nil;
}

WPIOSAttachment *wp_ios_attachment(NSString *identifier) {
    wp_ios_attachments_prune();
    WPIOSAttachment *lease = leases[identifier];
    if (!lease) return nil;
    NSNumber *regular, *symbolic, *size;
    [lease.url getResourceValue:&regular forKey:NSURLIsRegularFileKey error:nil];
    [lease.url getResourceValue:&symbolic forKey:NSURLIsSymbolicLinkKey error:nil];
    [lease.url getResourceValue:&size forKey:NSURLFileSizeKey error:nil];
    if (!regular.boolValue || symbolic.boolValue || size.unsignedLongLongValue != lease.bytes ||
        ![lease.url.URLByDeletingLastPathComponent.path isEqualToString:ownedDirectory.path]) return nil;
    return lease;
}

@interface WPIOSAttachmentPicker : NSObject <UIDocumentPickerDelegate, UIAdaptivePresentationControllerDelegate>
@property(nonatomic, weak) WPIOSOperation *operation;
@property(nonatomic, strong) NSProgress *progress;
@property(nonatomic) BOOL importing;
@property(nonatomic, copy) NSString *clientIdentifier;
@end
@implementation WPIOSAttachmentPicker
- (void)documentPickerWasCancelled:(UIDocumentPickerViewController *)controller {
    pickerResult(self.operation, self.clientIdentifier, nil, nil);
}
- (void)presentationControllerDidDismiss:(UIPresentationController *)controller {
    if (!self.importing) pickerResult(self.operation, self.clientIdentifier, nil, nil);
}
- (void)documentPicker:(UIDocumentPickerViewController *)controller didPickDocumentsAtURLs:(NSArray<NSURL *> *)urls {
    WPIOSOperation *operation = self.operation;
    if (!operation || operation.finished) return;
    self.importing = YES;
    if (!urls.count || urls.count > 8) {
        pickerResult(operation, self.clientIdentifier, nil, @"Choose between one and eight message attachments."); return;
    }
    NSError *error;
    NSURL *root = directory(&error);
    if (!root) { pickerResult(operation, self.clientIdentifier, nil, error.localizedDescription ?: @"The app could not create its private attachment directory."); return; }
    NSString *client = self.clientIdentifier;
    NSProgress *progress = self.progress;
    __weak WPIOSOperation *weak = operation;
    dispatch_async(dispatch_get_global_queue(QOS_CLASS_USER_INITIATED, 0), ^{
        NSMutableDictionary *imported = [NSMutableDictionary new];
        NSMutableArray *descriptors = [NSMutableArray new];
        __block NSError *failure;
        __block NSUInteger total = 0;
        for (NSURL *source in urls) {
            if (progress.cancelled || failure) break;
            BOOL scoped = [source startAccessingSecurityScopedResource];
            NSFileCoordinator *coordinator = [[NSFileCoordinator alloc] initWithFilePresenter:nil];
            [coordinator coordinateReadingItemAtURL:source options:0 error:&failure byAccessor:^(NSURL *url) {
                NSNumber *regular, *symbolic, *size;
                if (![url getResourceValue:&regular forKey:NSURLIsRegularFileKey error:&failure] || !regular.boolValue ||
                    ![url getResourceValue:&symbolic forKey:NSURLIsSymbolicLinkKey error:&failure] || symbolic.boolValue ||
                    ![url getResourceValue:&size forKey:NSURLFileSizeKey error:&failure] || !size.unsignedLongLongValue ||
                    size.unsignedLongLongValue > attachmentLimit || total + size.unsignedLongLongValue > selectionLimit) {
                    if (!failure) failure = [NSError errorWithDomain:@"PhoneAttachments" code:1 userInfo:@{NSLocalizedDescriptionKey:@"Attachments must be regular files up to 8 MiB each and 24 MiB per selection."}];
                    return;
                }
                if (progress.cancelled) return;
                NSFileHandle *handle = [NSFileHandle fileHandleForReadingFromURL:url error:&failure];
                NSData *bytes = [handle readDataUpToLength:attachmentLimit + 1 error:&failure];
                [handle closeAndReturnError:nil];
                if (!bytes || bytes.length != size.unsignedLongLongValue || bytes.length > attachmentLimit) {
                    if (!failure) failure = [NSError errorWithDomain:@"PhoneAttachments" code:2 userInfo:@{NSLocalizedDescriptionKey:@"The selected attachment changed while it was being copied."}];
                    return;
                }
                NSString *fileName = NSUUID.UUID.UUIDString;
                NSString *identifier = [@"draft-attachment:" stringByAppendingString:fileName];
                NSURL *destination = [root URLByAppendingPathComponent:fileName];
                if (![bytes writeToURL:destination options:NSDataWritingAtomic | NSDataWritingFileProtectionCompleteUntilFirstUserAuthentication error:&failure]) return;
                WPIOSAttachment *lease = [WPIOSAttachment new]; lease.url = destination;
                lease.name = wp_ios_text(url.lastPathComponent, 255); lease.bytes = bytes.length;
                UTType *type; [url getResourceValue:&type forKey:NSURLContentTypeKey error:nil];
                lease.mime = type.preferredMIMEType ?: @"application/octet-stream";
                lease.expires = NSProcessInfo.processInfo.systemUptime + 900;
                imported[identifier] = lease; total += bytes.length;
                [descriptors addObject:@{@"id":identifier, @"name":lease.name, @"mime":lease.mime, @"bytes":@(lease.bytes)}];
            }];
            if (scoped) [source stopAccessingSecurityScopedResource];
        }
        dispatch_async(dispatch_get_main_queue(), ^{
            WPIOSOperation *current = weak;
            wp_ios_attachments_prune();
            NSUInteger retainedBytes = 0; for (WPIOSAttachment *lease in leases.allValues) retainedBytes += lease.bytes;
            BOOL cancelled = !current || current.finished || progress.cancelled;
            if (cancelled || failure || leases.count + imported.count > 16 || retainedBytes + total > 64 * 1024 * 1024) {
                for (WPIOSAttachment *lease in imported.allValues) [NSFileManager.defaultManager removeItemAtURL:lease.url error:nil];
                if (!cancelled) pickerResult(current, client, nil, failure.localizedDescription ?: @"Discard unused draft attachments before importing more files.");
                return;
            }
            if (!leases) leases = [NSMutableDictionary new];
            [leases addEntriesFromDictionary:imported];
            pickerResult(current, client, descriptors, nil);
        });
    });
}
@end

void wp_ios_pick_attachments(WPIOSOperation *operation, NSString *clientIdentifier) {
    UIViewController *presenter = wp_ios_presenter();
    if (!presenter || presenter.isBeingPresented || presenter.isBeingDismissed) {
        pickerResult(operation, clientIdentifier, nil, @"The active app cannot open the attachment picker right now."); return;
    }
    WPIOSAttachmentPicker *delegate = [WPIOSAttachmentPicker new]; delegate.operation = operation;
    delegate.clientIdentifier = clientIdentifier;
    delegate.progress = [NSProgress progressWithTotalUnitCount:1];
    UIDocumentPickerViewController *picker = [[UIDocumentPickerViewController alloc] initForOpeningContentTypes:@[UTTypeData] asCopy:YES];
    picker.allowsMultipleSelection = YES; picker.delegate = delegate;
    operation.cancelled = ^{
        [delegate.progress cancel]; delegate.operation = nil; picker.delegate = nil;
        picker.presentationController.delegate = nil;
        if (picker.presentingViewController) [picker dismissViewControllerAnimated:YES completion:nil];
    };
    [presenter presentViewController:picker animated:YES completion:nil];
    picker.presentationController.delegate = delegate;
}

char *wp_ios_attachment_export(const char *identifier) {
    if (!NSThread.isMainThread || !identifier) return NULL;
    WPIOSAttachment *lease = wp_ios_attachment([NSString stringWithUTF8String:identifier]);
    if (!lease) return NULL;
    NSData *data = [NSData dataWithContentsOfURL:lease.url];
    if (!data || data.length != lease.bytes) return NULL;
    NSDictionary *value = @{@"name":lease.name, @"mime":lease.mime, @"data":[data base64EncodedStringWithOptions:0]};
    NSData *json = [NSJSONSerialization dataWithJSONObject:value options:0 error:nil];
    NSString *text = [[NSString alloc] initWithData:json encoding:NSUTF8StringEncoding];
    return text ? strdup(text.UTF8String) : NULL;
}

int wp_ios_preview_carrier_attachment(uint64_t identifier, const uint8_t *bytes, size_t length, const char *name, const char *mime) {
    (void)mime; // The native activity infers its preview type from the safe name.
    WPIOSOperation *operation = wp_ios_operation_lookup(identifier);
    if (!operation || operation.finished) return 0;
    if (!bytes || !length || length > 24 * 1024 * 1024) {
        [operation finish:0 first:0 second:0 message:@"The selected attachment exceeds the 24 MiB native preview limit."]; return 1;
    }
    UIViewController *presenter = wp_ios_presenter();
    if (!presenter || presenter.isBeingPresented) {
        [operation finish:0 first:0 second:0 message:@"The app cannot present the attachment right now."]; return 1;
    }
    NSError *error;
    NSURL *root = directory(&error);
    NSString *displayName = wp_ios_text(name ? [NSString stringWithUTF8String:name] : @"attachment", 255).lastPathComponent;
    if (!displayName.length || [displayName isEqualToString:@"."] || [displayName isEqualToString:@".."]) displayName = @"attachment";
    NSURL *folder = [root URLByAppendingPathComponent:NSUUID.UUID.UUIDString isDirectory:YES];
    if (!root || ![NSFileManager.defaultManager createDirectoryAtURL:folder withIntermediateDirectories:YES attributes:nil error:&error]) {
        [operation finish:0 first:0 second:0 message:error.localizedDescription]; return 1;
    }
    NSURL *file = [folder URLByAppendingPathComponent:displayName];
    NSData *data = [NSData dataWithBytes:bytes length:length];
    if (![data writeToURL:file options:NSDataWritingAtomic | NSDataWritingFileProtectionCompleteUntilFirstUserAuthentication error:&error]) {
        [NSFileManager.defaultManager removeItemAtURL:folder error:nil];
        [operation finish:0 first:0 second:0 message:error.localizedDescription]; return 1;
    }
    UIActivityViewController *sheet = [[UIActivityViewController alloc] initWithActivityItems:@[file] applicationActivities:nil];
    __weak UIActivityViewController *weakSheet = sheet;
    __weak WPIOSOperation *weak = operation;
    operation.cancelled = ^{
        UIActivityViewController *current = weakSheet;
        if (current.presentingViewController) [current dismissViewControllerAnimated:YES completion:nil];
        [NSFileManager.defaultManager removeItemAtURL:folder error:nil];
    };
    sheet.completionWithItemsHandler = ^(UIActivityType type, BOOL completed, NSArray *items, NSError *failure) {
        if (failure) [weak finish:0 first:0 second:0 message:failure.localizedDescription];
        else wp_ios_device_accept(weak, completed ? @"The selected attachment activity completed." : @"Attachment preview cancelled.");
    };
    if (sheet.popoverPresentationController) {
        sheet.popoverPresentationController.sourceView = presenter.view;
        sheet.popoverPresentationController.sourceRect = CGRectMake(CGRectGetMidX(presenter.view.bounds), CGRectGetMaxY(presenter.view.bounds)-1, 1, 1);
    }
    [presenter presentViewController:sheet animated:YES completion:nil];
    return 1;
}
