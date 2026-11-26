#import <Foundation/Foundation.h>
#import <Network/Network.h>
#include <stdint.h>

typedef void (*WPIOSNetworkUpdate)(uint64_t, int);

@interface WPIOSNetworkMonitor : NSObject
@property(nonatomic, strong) nw_path_monitor_t monitor;
@end
@implementation WPIOSNetworkMonitor
@end

static NSMutableDictionary<NSNumber *, WPIOSNetworkMonitor *> *monitors;

int wp_ios_network_start(uint64_t identifier, WPIOSNetworkUpdate callback) {
    if (!NSThread.isMainThread || !identifier || !callback) return 0;
    if (!monitors) monitors = [NSMutableDictionary new];
    if (monitors.count >= 16 || monitors[@(identifier)]) return 0;
    nw_path_monitor_t monitor = nw_path_monitor_create();
    if (!monitor) return 0;
    WPIOSNetworkMonitor *owner = [WPIOSNetworkMonitor new];
    owner.monitor = monitor;
    monitors[@(identifier)] = owner;
    __weak WPIOSNetworkMonitor *weak = owner;
    nw_path_monitor_set_queue(monitor, dispatch_get_main_queue());
    nw_path_monitor_set_update_handler(monitor, ^(nw_path_t path) {
        WPIOSNetworkMonitor *current = weak;
        if (!current || monitors[@(identifier)] != current) return;
        int value = -1;
        switch (nw_path_get_status(path)) {
            case nw_path_status_satisfied: value = 1; break;
            case nw_path_status_unsatisfied: value = 0; break;
            // A satisfiable path can be activated by a connection attempt.
            // Do not turn this or an invalid/unknown observation into offline.
            default: break;
        }
        // This reports path availability, not Internet/service reachability.
        // This observer does not initiate connections or request permission.
        callback(identifier, value);
    });
    nw_path_monitor_start(monitor);
    return 1;
}

void wp_ios_network_cancel(uint64_t identifier) {
    NSCAssert(NSThread.isMainThread, @"Network observation lifetime requires the UIKit thread");
    WPIOSNetworkMonitor *owner = monitors[@(identifier)];
    if (!owner) return;
    [monitors removeObjectForKey:@(identifier)];
    nw_path_monitor_cancel(owner.monitor);
    owner.monitor = nil;
}
