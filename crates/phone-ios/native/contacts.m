#import "device.h"
#import <Contacts/Contacts.h>
#include <stdatomic.h>

static atomic_uint_fast64_t contactsGeneration;
void wp_ios_contacts_changed(void) { atomic_fetch_add(&contactsGeneration, 1); }

NSString *wp_ios_contacts_permission(void) {
    CNAuthorizationStatus status = [CNContactStore authorizationStatusForEntityType:CNEntityTypeContacts];
#if __IPHONE_OS_VERSION_MAX_ALLOWED >= 180000
    if (@available(iOS 18, *)) { if (status == CNAuthorizationStatusLimited) return @"limited"; }
#endif
    switch (status) {
        case CNAuthorizationStatusNotDetermined: return @"prompt";
        case CNAuthorizationStatusAuthorized: return @"granted";
        case CNAuthorizationStatusRestricted: return @"restricted";
        default: return @"denied";
    }
}

void wp_ios_contacts_authorize(WPIOSOperation *operation) {
    if (![NSBundle.mainBundle objectForInfoDictionaryKey:@"NSContactsUsageDescription"]) {
        [operation finish:0 first:0 second:0 message:@"The app package does not declare contacts usage."]; return;
    }
    if (![wp_ios_contacts_permission() isEqualToString:@"prompt"]) {
        wp_ios_device_finish(operation, @"snapshot", wp_ios_device_snapshot()); return;
    }
    CNContactStore *store = [CNContactStore new];
    // Retain the store until the prompt completes; cancellation suppresses all
    // subsequent admission while UIKit owns the non-dismissable system prompt.
    operation.cancelled = ^{ (void)store; };
    __weak WPIOSOperation *weak = operation;
    [store requestAccessForEntityType:CNEntityTypeContacts completionHandler:^(BOOL granted, NSError *error) {
        dispatch_async(dispatch_get_main_queue(), ^{
            WPIOSOperation *current = weak;
            if (!current || current.finished) return;
            if (error) [current finish:0 first:0 second:0 message:error.localizedDescription];
            else wp_ios_device_finish(current, @"snapshot", wp_ios_device_snapshot());
        });
    }];
}

static void readContacts(WPIOSOperation *operation, NSString *query, NSUInteger limit, NSUInteger retries) {
    NSString *permission = wp_ios_contacts_permission();
    if (![permission isEqualToString:@"granted"] && ![permission isEqualToString:@"limited"]) {
        [operation finish:0 first:0 second:0 message:@"Allow contacts access before reading the selected device contacts."]; return;
    }
    if (![query isKindOfClass:NSString.class] || !limit || limit > 1000) {
        [operation finish:0 first:0 second:0 message:@"The contacts query is invalid."]; return;
    }
    NSProgress *progress = [NSProgress progressWithTotalUnitCount:1];
    uint64_t generation = atomic_load(&contactsGeneration);
    operation.cancelled = ^{ [progress cancel]; };
    __weak WPIOSOperation *weak = operation;
    dispatch_async(dispatch_get_global_queue(QOS_CLASS_USER_INITIATED, 0), ^{
        @autoreleasepool {
            if (progress.cancelled) return;
            CNContactStore *store = [CNContactStore new];
            NSArray<id<CNKeyDescriptor>> *keys = @[CNContactIdentifierKey,
                CNContactGivenNameKey, CNContactFamilyNameKey, CNContactOrganizationNameKey,
                CNContactPhoneNumbersKey, CNContactEmailAddressesKey,
                [CNContactFormatter descriptorForRequiredKeysForStyle:CNContactFormatterStyleFullName]];
            CNContactFetchRequest *request = [[CNContactFetchRequest alloc] initWithKeysToFetch:keys];
            request.sortOrder = CNContactSortOrderUserDefault;
            request.unifyResults = YES;
            NSMutableArray *contacts = [NSMutableArray new];
            NSMutableSet *identifiers = [NSMutableSet new];
            __block NSError *failure;
            [store enumerateContactsWithFetchRequest:request error:&failure
                usingBlock:^(CNContact *contact, BOOL *stop) {
                    if (progress.cancelled) { *stop = YES; return; }
                    NSString *identifier = contact.identifier;
                    if (!identifier.length || [identifier lengthOfBytesUsingEncoding:NSUTF8StringEncoding] > 1024 ||
                        [identifiers containsObject:identifier]) return;
                    NSString *name = [CNContactFormatter stringFromContact:contact style:CNContactFormatterStyleFullName];
                    if (!name.length) name = contact.organizationName;
                    NSMutableArray *phones = [NSMutableArray new], *emails = [NSMutableArray new];
                    for (CNLabeledValue<CNPhoneNumber *> *entry in contact.phoneNumbers) {
                        NSString *number = wp_ios_text(entry.value.stringValue, 256);
                        if ([number stringByTrimmingCharactersInSet:NSCharacterSet.whitespaceAndNewlineCharacterSet].length &&
                            ![phones containsObject:number]) [phones addObject:number];
                        if (phones.count == 32) break;
                    }
                    for (CNLabeledValue<NSString *> *entry in contact.emailAddresses) {
                        NSString *address = wp_ios_text(entry.value, 512);
                        if ([address stringByTrimmingCharactersInSet:NSCharacterSet.whitespaceAndNewlineCharacterSet].length &&
                            ![emails containsObject:address]) [emails addObject:address];
                        if (emails.count == 32) break;
                    }
                    NSString *search = [@[name ?: @"", [phones componentsJoinedByString:@" "],
                        [emails componentsJoinedByString:@" "]] componentsJoinedByString:@" "];
                    if (query.length && [search rangeOfString:query options:NSCaseInsensitiveSearch | NSDiacriticInsensitiveSearch].location == NSNotFound) return;
                    [identifiers addObject:identifier];
                    [contacts addObject:@{@"id":identifier, @"name":wp_ios_text(name, 512),
                        @"phones":phones, @"emails":emails, @"favorite":@NO}];
                    if (contacts.count >= limit) *stop = YES;
                }];
            if (progress.cancelled) return;
            dispatch_async(dispatch_get_main_queue(), ^{
                WPIOSOperation *current = weak;
                if (!current || current.finished || progress.cancelled) return;
                // Permission can be revoked while the worker is enumerating.
                NSString *now = wp_ios_contacts_permission();
                if ([now isEqualToString:permission] && generation != atomic_load(&contactsGeneration) && retries) {
                    // Saving through CNContactStore can enqueue its change
                    // notification after the save completion. Re-enumerate a
                    // bounded number of times; never admit the stale result.
                    dispatch_after(dispatch_time(DISPATCH_TIME_NOW, 100 * NSEC_PER_MSEC), dispatch_get_main_queue(), ^{
                        WPIOSOperation *retry = weak;
                        if (retry && !retry.finished && !progress.cancelled)
                            readContacts(retry, query, limit, retries - 1);
                    });
                } else if (![now isEqualToString:permission] || generation != atomic_load(&contactsGeneration))
                    [current finish:0 first:0 second:0 message:@"The selected contacts changed while reading. Refresh to read the current selection."];
                else if (failure) [current finish:0 first:0 second:0 message:failure.localizedDescription];
                else wp_ios_device_finish(current, @"contacts", contacts);
            });
        }
    });
}

void wp_ios_contacts_read(WPIOSOperation *operation, NSString *query, NSUInteger limit) {
    readContacts(operation, query, limit, 2);
}

static NSArray<id<CNKeyDescriptor>> *editableKeys(void) {
    return @[CNContactIdentifierKey, CNContactGivenNameKey, CNContactFamilyNameKey,
        CNContactOrganizationNameKey,
        CNContactMiddleNameKey, CNContactNamePrefixKey, CNContactNameSuffixKey,
        CNContactPhoneNumbersKey, CNContactEmailAddressesKey,
        [CNContactFormatter descriptorForRequiredKeysForStyle:CNContactFormatterStyleFullName]];
}

static BOOL mayWrite(WPIOSOperation *operation) {
    NSString *status = wp_ios_contacts_permission();
    if ([status isEqualToString:@"granted"] || [status isEqualToString:@"limited"]) return YES;
    [operation finish:0 first:0 second:0 message:@"Allow access to the selected device contacts before changing them."];
    return NO;
}

static void writeContact(WPIOSOperation *operation, NSDictionary *values, NSString *identifier, BOOL deleting) {
    if (!mayWrite(operation)) return;
    NSProgress *progress = [NSProgress progressWithTotalUnitCount:1];
    operation.cancelled = ^{ [progress cancel]; };
    __weak WPIOSOperation *weak = operation;
    dispatch_async(dispatch_get_global_queue(QOS_CLASS_USER_INITIATED, 0), ^{
        @autoreleasepool {
            if (progress.cancelled) return;
            CNContactStore *store = [CNContactStore new];
            NSError *error;
            CNMutableContact *contact;
            if (identifier.length) {
                // An inaccessible identifier fails lookup under limited access;
                // no fallback creates or replaces a different contact.
                CNContact *original = [store unifiedContactWithIdentifier:identifier keysToFetch:editableKeys() error:&error];
                contact = [original mutableCopy];
            } else if (!deleting) contact = [CNMutableContact new];
            if (!contact) {
                dispatch_async(dispatch_get_main_queue(), ^{
                    [weak finish:0 first:0 second:0 message:error.localizedDescription ?: @"The selected device contact is no longer accessible."];
                }); return;
            }
            CNSaveRequest *save = [CNSaveRequest new];
            if (deleting) [save deleteContact:contact];
            else {
                NSString *name = values[@"name"] ?: @"";
                NSString *previous = [CNContactFormatter stringFromContact:contact style:CNContactFormatterStyleFullName] ?: @"";
                if (!previous.length) previous = contact.organizationName;
                if (![name isEqualToString:previous]) {
                    NSPersonNameComponentsFormatter *formatter = [NSPersonNameComponentsFormatter new];
                    NSPersonNameComponents *parts = [formatter personNameComponentsFromString:name];
                    contact.givenName = parts ? (parts.givenName ?: @"") : name;
                    contact.familyName = parts.familyName ?: @"";
                    contact.middleName = parts.middleName ?: @"";
                    contact.namePrefix = parts.namePrefix ?: @"";
                    contact.nameSuffix = parts.nameSuffix ?: @"";
                }
                NSMutableArray *phones = [NSMutableArray new], *emails = [NSMutableArray new];
                for (NSString *number in values[@"phones"]) {
                    CNLabeledValue *previous = nil;
                    for (CNLabeledValue<CNPhoneNumber *> *item in contact.phoneNumbers)
                        if ([item.value.stringValue isEqualToString:number]) { previous = item; break; }
                    [phones addObject:previous ?: [CNLabeledValue labeledValueWithLabel:CNLabelOther value:[CNPhoneNumber phoneNumberWithStringValue:number]]];
                }
                for (NSString *email in values[@"emails"]) {
                    CNLabeledValue *previous = nil;
                    for (CNLabeledValue<NSString *> *item in contact.emailAddresses)
                        if ([item.value isEqualToString:email]) { previous = item; break; }
                    [emails addObject:previous ?: [CNLabeledValue labeledValueWithLabel:CNLabelOther value:email]];
                }
                contact.phoneNumbers = phones; contact.emailAddresses = emails;
                // The public Contacts API does not expose the system Favorites
                // list. Never overwrite unrelated fields to emulate it.
                if (identifier.length) [save updateContact:contact]; else [save addContact:contact toContainerWithIdentifier:nil];
            }
            if (progress.cancelled) return;
            BOOL saved = [store executeSaveRequest:save error:&error];
            dispatch_async(dispatch_get_main_queue(), ^{
                WPIOSOperation *current = weak;
                if (!current || current.finished || progress.cancelled) return;
                if (saved) {
                    wp_ios_contacts_changed();
                    wp_ios_device_emit(@"contacts", @[]);
                    wp_ios_device_accept(current, deleting ? @"The selected device contact was deleted." : @"The device contact was saved.");
                } else [current finish:0 first:0 second:0 message:error.localizedDescription ?: @"iOS could not save the contact change."];
            });
        }
    });
}

void wp_ios_contacts_save(WPIOSOperation *operation, NSDictionary *contact) {
    if ([contact[@"favorite"] boolValue]) {
        [operation finish:0 first:0 second:0 message:@"The public iOS Contacts API cannot change the system Favorites list. Save the contact without requesting a Favorites change."]; return;
    }
    writeContact(operation, contact, contact[@"id"] ?: @"", NO);
}

void wp_ios_contacts_delete(WPIOSOperation *operation, NSString *identifier) {
    writeContact(operation, nil, identifier, YES);
}
