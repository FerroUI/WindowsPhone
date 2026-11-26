#import "bridge.h"

// All entry points and completions run on UIKit's main thread. Worker reads
// carry a cancellation token and marshal their result back before delivery.
NSString *wp_ios_text(NSString *text, NSUInteger maximumBytes);
void wp_ios_notifications_install(void);
uint32_t wp_ios_notification_policy(void);
void wp_ios_set_notification_policy(uint32_t policy);
NSDictionary *wp_ios_device_snapshot(void);
void wp_ios_device_finish(WPIOSOperation *operation, NSString *kind, id value);
void wp_ios_device_accept(WPIOSOperation *operation, NSString *message);
void wp_ios_device_emit(NSString *kind, id value);
void wp_ios_device_dirty(void);
void wp_ios_device_open_url(WPIOSOperation *operation, NSURL *url, NSString *success);

NSString *wp_ios_contacts_permission(void);
NSString *wp_ios_notifications_permission(void);
void wp_ios_contacts_authorize(WPIOSOperation *operation);
void wp_ios_contacts_read(WPIOSOperation *operation, NSString *query, NSUInteger limit);
void wp_ios_contacts_changed(void);
void wp_ios_contacts_save(WPIOSOperation *operation, NSDictionary *contact);
void wp_ios_contacts_delete(WPIOSOperation *operation, NSString *identifier);
NSArray *wp_ios_hardware_controls(void);
void wp_ios_hardware_set(WPIOSOperation *operation, NSDictionary *request);
void wp_ios_hardware_open(WPIOSOperation *operation, NSString *control);
void wp_ios_hardware_suspend(void);
void wp_ios_compose_message(WPIOSOperation *operation, NSArray *recipients, NSString *body);
void wp_ios_compose_multimedia(WPIOSOperation *operation, NSDictionary *request);
void wp_ios_dial(WPIOSOperation *operation, NSString *number, NSString *account);
void wp_ios_open_map(WPIOSOperation *operation, NSDictionary *request);
void wp_ios_location_permission(WPIOSOperation *operation);
NSString *wp_ios_location_authorization(void);
void wp_ios_location_monitor(int enabled);

#if WP_IOS_TELEPHONY
// The Swift adapter is compiled only by the explicit ios26-telephony feature.
// Each exported buffer is owned by the caller and released with free().
char *wp_ios_telephony_snapshot(void);
int wp_ios_telephony_command(uint64_t identifier, const char *request);
void wp_ios_telephony_cancel(uint64_t identifier);
void wp_ios_telephony_monitor(int enabled);
#endif
