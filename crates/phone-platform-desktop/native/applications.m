#import <AppKit/AppKit.h>
#import <Foundation/Foundation.h>
#include <stdlib.h>
#include <string.h>

// The Rust caller frees returned buffers with phone_desktop_free. Discovery
// runs on the bounded host worker; NSWorkspace activation runs on the UI thread.
static char *copyText(NSString *text) {
    if (!text) return NULL;
    return strdup(text.UTF8String ?: "The application service failed.");
}
void phone_desktop_free(void *value) { free(value); }

static BOOL appURL(NSURL *url) {
    NSNumber *directory = nil;
    return [url.pathExtension caseInsensitiveCompare:@"app"] == NSOrderedSame &&
        [url getResourceValue:&directory forKey:NSURLIsDirectoryKey error:nil] && directory.boolValue;
}
static BOOL booleanValue(id value) {
    return ([value isKindOfClass:NSNumber.class] || [value isKindOfClass:NSString.class]) && [value boolValue];
}

char *phone_desktop_catalog(char **error) {
    @autoreleasepool {
        if (error) *error = NULL;
        NSFileManager *files = NSFileManager.defaultManager;
        NSMutableArray<NSURL *> *roots = [[files URLsForDirectory:NSApplicationDirectory
            inDomains:NSLocalDomainMask | NSSystemDomainMask | NSUserDomainMask] mutableCopy];
        // Modern macOS separates the sealed system applications from /Applications.
        [roots addObject:[NSURL fileURLWithPath:@"/System/Applications" isDirectory:YES]];
        NSMutableSet<NSString *> *seen = [NSMutableSet set];
        NSMutableArray<NSDictionary *> *apps = [NSMutableArray array];
        NSUInteger visited = 0;
        for (NSURL *root in roots) {
            NSDirectoryEnumerator<NSURL *> *entries = [files enumeratorAtURL:root
                includingPropertiesForKeys:@[NSURLIsDirectoryKey, NSURLIsPackageKey, NSURLFileSizeKey]
                options:NSDirectoryEnumerationSkipsHiddenFiles | NSDirectoryEnumerationSkipsPackageDescendants
                errorHandler:^BOOL(NSURL *url, NSError *readError) { (void)url; (void)readError; return YES; }];
            for (NSURL *entry in entries) {
                if (++visited > 32768) {
                    if (error) *error = copyText(@"The application directories exceed the discovery limit.");
                    return NULL;
                }
                if (entries.level > 4) { [entries skipDescendants]; continue; }
                if (!appURL(entry)) continue;
                [entries skipDescendants];
                NSURL *url = entry.URLByResolvingSymlinksInPath.URLByStandardizingPath;
                NSString *path = url.path;
                if (!path || [seen containsObject:path]) continue;
                [seen addObject:path];
                NSURL *infoURL = [url URLByAppendingPathComponent:@"Contents/Info.plist"];
                NSNumber *size = nil;
                if (![infoURL getResourceValue:&size forKey:NSURLFileSizeKey error:nil] ||
                    size.unsignedLongLongValue > 1024 * 1024) continue;
                NSData *data = [NSData dataWithContentsOfURL:infoURL options:0 error:nil];
                if (!data || data.length > 1024 * 1024) continue;
                id info = [NSPropertyListSerialization propertyListWithData:data options:NSPropertyListImmutable format:nil error:nil];
                if (![info isKindOfClass:NSDictionary.class] || ![info[@"CFBundlePackageType"] isEqual:@"APPL"]) continue;
                // Agents/background helpers are not user-facing launchable apps.
                if (booleanValue(info[@"LSUIElement"]) || booleanValue(info[@"LSBackgroundOnly"])) continue;
                NSString *name = [files displayNameAtPath:path];
                if ([name.pathExtension caseInsensitiveCompare:@"app"] == NSOrderedSame) name = name.stringByDeletingPathExtension;
                if (!name.length) name = info[@"CFBundleDisplayName"] ?: info[@"CFBundleName"];
                if (![name isKindOfClass:NSString.class] || !name.length) continue;
                [apps addObject:@{@"identity":path, @"title":name}];
                if (apps.count > 4096) {
                    if (error) *error = copyText(@"The installed application catalog exceeds its limit.");
                    return NULL;
                }
            }
        }
        NSData *json = [NSJSONSerialization dataWithJSONObject:apps options:0 error:nil];
        if (!json || json.length > 8 * 1024 * 1024) {
            if (error) *error = copyText(@"The application catalog could not be encoded within its limit.");
            return NULL;
        }
        char *result = malloc(json.length + 1);
        if (!result) { if (error) *error = copyText(@"The application catalog could not allocate memory."); return NULL; }
        memcpy(result, json.bytes, json.length);
        result[json.length] = 0;
        return result;
    }
}

typedef void (*PhoneApplicationCompletion)(void *context, int accepted, const char *error);
void phone_desktop_launch(const char *path, void *context, PhoneApplicationCompletion completion) {
    @autoreleasepool {
        if (![NSThread isMainThread]) { completion(context, 0, "Application activation must run on the main thread."); return; }
        NSString *value = path ? [NSString stringWithUTF8String:path] : nil;
        NSURL *url = value ? [NSURL fileURLWithPath:value isDirectory:YES] : nil;
        if (!url || !appURL(url) || ![url.URLByResolvingSymlinksInPath.URLByStandardizingPath.path isEqualToString:value]) {
            completion(context, 0, "That application has moved or is no longer installed. Refresh the device applications."); return;
        }
        NSWorkspaceOpenConfiguration *config = NSWorkspaceOpenConfiguration.configuration;
        config.activates = YES;
        config.createsNewApplicationInstance = NO;
        config.addsToRecentItems = NO;
        // No arguments, environment overrides, or privileged launch flags.
        [NSWorkspace.sharedWorkspace openApplicationAtURL:url configuration:config
            completionHandler:^(NSRunningApplication *application, NSError *error) {
                NSString *message = error.localizedDescription;
                if (message.length > 4096) message = [message substringToIndex:4096];
                completion(context, application != nil && error == nil, message.UTF8String);
            }];
    }
}

int phone_desktop_open_settings(void) {
    @autoreleasepool {
        if (![NSThread isMainThread]) return 0;
        // Public LaunchServices activation of the application, rather than
        // undocumented preference-pane URL schemes that vary across releases.
        for (NSString *path in @[@"/System/Applications/System Settings.app", @"/System/Applications/System Preferences.app", @"/Applications/System Preferences.app"]) {
            NSURL *url = [NSURL fileURLWithPath:path isDirectory:YES];
            if (appURL(url)) return [NSWorkspace.sharedWorkspace openURL:url] ? 1 : 0;
        }
        return 0;
    }
}
