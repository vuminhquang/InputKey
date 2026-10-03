#import <Cocoa/Cocoa.h>

NS_ASSUME_NONNULL_BEGIN

extern NSString * const InputKeySettingsDidChangeNotification;

@interface InputKeySettingsWindowController : NSWindowController
+ (instancetype)sharedController;
- (void)show;
@end

NS_ASSUME_NONNULL_END
