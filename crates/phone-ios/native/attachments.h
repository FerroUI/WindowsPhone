#import "device.h"

@interface WPIOSAttachment : NSObject
@property(nonatomic, strong) NSURL *url;
@property(nonatomic, copy) NSString *name;
@property(nonatomic, copy) NSString *mime;
@property(nonatomic) NSUInteger bytes;
@property(nonatomic) NSTimeInterval expires;
@end

WPIOSAttachment *wp_ios_attachment(NSString *identifier);
void wp_ios_pick_attachments(WPIOSOperation *operation, NSString *clientIdentifier);
void wp_ios_discard_attachments(NSArray<NSString *> *identifiers);
void wp_ios_attachments_prune(void);
void wp_ios_attachments_close(void);
char *wp_ios_attachment_export(const char *identifier);
int wp_ios_preview_carrier_attachment(uint64_t identifier, const uint8_t *data, size_t length,
    const char *name, const char *mime);
