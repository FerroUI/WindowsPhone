#import <Foundation/Foundation.h>
#import <UIKit/UIKit.h>
#include <stdint.h>

typedef void (*WPIOSCompletion)(uint64_t, int, double, double, const char *);

@interface WPIOSOperation : NSObject
@property(nonatomic) uint64_t identifier;
@property(nonatomic) WPIOSCompletion completion;
@property(nonatomic) BOOL finished;
@property(nonatomic, strong) NSTimer *timer;
@property(nonatomic, copy) void (^cancelled)(void);
- (void)finish:(int)code first:(double)first second:(double)second message:(NSString *)message;
@end

WPIOSOperation *wp_ios_operation(uint64_t identifier, WPIOSCompletion callback, NSTimeInterval timeout);
UIViewController *wp_ios_presenter(void);
void wp_ios_cancel(uint64_t identifier);
int wp_ios_data_path(char *buffer, size_t length);
