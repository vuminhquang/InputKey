#import <Cocoa/Cocoa.h>
#import <InputMethodKit/InputMethodKit.h>

int main(int argc, const char *argv[]) {
    (void)argc;
    (void)argv;

    @autoreleasepool {
        [[NSUserDefaults standardUserDefaults] registerDefaults:@{
            @"InputKeyLanguage": @"vi",
            @"InputKeyMethod": @"telex",
            @"InputKeySimpleTelex": @NO,
            @"InputKeyAutoRestore": @YES,
            @"InputKeySmartCorrection": @YES,
        }];

        NSApplication *app = [NSApplication sharedApplication];
        [app setActivationPolicy:NSApplicationActivationPolicyAccessory];

        NSString *bundleIdentifier = [[NSBundle mainBundle] bundleIdentifier];
        if (bundleIdentifier.length == 0) {
            bundleIdentifier = @"org.inputkey.inputmethod";
        }

        IMKServer *server = [[IMKServer alloc]
            initWithName:@"InputKey_Connection"
            bundleIdentifier:bundleIdentifier];
        if (server == nil) {
            NSLog(@"InputKey: failed to initialize IMKServer");
            return 1;
        }

        [app run];
        (void)server;
    }
    return 0;
}
