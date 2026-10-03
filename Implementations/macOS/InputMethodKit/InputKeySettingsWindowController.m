#import "InputKeySettingsWindowController.h"

#include <stdint.h>
#include <stdlib.h>

#include "inputkey.h"

NSString * const InputKeySettingsDidChangeNotification =
    @"InputKeySettingsDidChangeNotification";

@interface InputKeySettingsWindowController ()
@property(nonatomic, strong) NSPopUpButton *languagePopup;
@property(nonatomic, strong) NSPopUpButton *methodPopup;
@property(nonatomic, strong) NSButton *simpleTelex;
@property(nonatomic, strong) NSButton *autoRestore;
@property(nonatomic, strong) NSButton *smartCorrection;
@property(nonatomic, copy) NSArray<NSDictionary *> *languages;
@end

@implementation InputKeySettingsWindowController

+ (instancetype)sharedController {
    static InputKeySettingsWindowController *controller;
    static dispatch_once_t onceToken;
    dispatch_once(&onceToken, ^{
        controller = [[InputKeySettingsWindowController alloc] init];
    });
    return controller;
}

static NSString *InputKeyCatalogJSON(void) {
    const size_t required = inputkey_catalog_json(NULL, 0);
    uint8_t *buffer = calloc(required + 1, sizeof(uint8_t));
    if (buffer == NULL) return @"";
    inputkey_catalog_json(buffer, required + 1);
    NSString *result = [[NSString alloc] initWithBytes:buffer
                                                length:required
                                              encoding:NSUTF8StringEncoding];
    free(buffer);
    return result ?: @"";
}

- (instancetype)init {
    NSRect frame = NSMakeRect(0, 0, 460, 410);
    NSWindow *window = [[NSWindow alloc]
        initWithContentRect:frame
                  styleMask:(NSWindowStyleMaskTitled |
                             NSWindowStyleMaskClosable |
                             NSWindowStyleMaskMiniaturizable)
                    backing:NSBackingStoreBuffered
                      defer:NO];
    window.title = @"InputKey Settings";
    window.releasedWhenClosed = NO;

    self = [super initWithWindow:window];
    if (self != nil) {
        [self loadCatalog];
        [self buildControls];
        [self refreshControls];
    }
    return self;
}

- (void)loadCatalog {
    NSData *data = [InputKeyCatalogJSON() dataUsingEncoding:NSUTF8StringEncoding];
    if (data.length == 0) {
        self.languages = @[];
        return;
    }
    NSDictionary *root = [NSJSONSerialization JSONObjectWithData:data options:0 error:nil];
    NSArray *languages = [root isKindOfClass:[NSDictionary class]] ? root[@"languages"] : nil;
    self.languages = [languages isKindOfClass:[NSArray class]] ? languages : @[];
}

- (NSTextField *)label:(NSString *)text frame:(NSRect)frame {
    NSTextField *label = [NSTextField labelWithString:text];
    label.frame = frame;
    return label;
}

- (NSButton *)checkbox:(NSString *)title frame:(NSRect)frame action:(SEL)action {
    NSButton *button = [[NSButton alloc] initWithFrame:frame];
    button.buttonType = NSSwitchButton;
    button.title = title;
    button.target = self;
    button.action = action;
    return button;
}

- (void)buildControls {
    NSView *content = self.window.contentView;
    if (content == nil) return;

    NSTextField *title = [self label:@"InputKey" frame:NSMakeRect(24, 354, 270, 28)];
    title.font = [NSFont boldSystemFontOfSize:22.0];
    [content addSubview:title];

    NSString *version = [[NSBundle mainBundle] objectForInfoDictionaryKey:@"CFBundleShortVersionString"];
    NSTextField *versionLabel = [self
        label:[NSString stringWithFormat:@"Native settings · v%@", version ?: @""]
        frame:NSMakeRect(24, 332, 300, 20)];
    versionLabel.textColor = NSColor.secondaryLabelColor;
    [content addSubview:versionLabel];

    [content addSubview:[self label:@"Language" frame:NSMakeRect(24, 286, 110, 22)]];
    self.languagePopup = [[NSPopUpButton alloc] initWithFrame:NSMakeRect(150, 282, 280, 28)
                                                    pullsDown:NO];
    self.languagePopup.target = self;
    self.languagePopup.action = @selector(languageChanged:);
    [content addSubview:self.languagePopup];

    [content addSubview:[self label:@"Input method" frame:NSMakeRect(24, 244, 110, 22)]];
    self.methodPopup = [[NSPopUpButton alloc] initWithFrame:NSMakeRect(150, 240, 280, 28)
                                                  pullsDown:NO];
    self.methodPopup.target = self;
    self.methodPopup.action = @selector(methodChanged:);
    [content addSubview:self.methodPopup];

    self.simpleTelex = [self checkbox:@"Simple Telex"
                                frame:NSMakeRect(24, 192, 240, 24)
                               action:@selector(optionChanged:)];
    self.simpleTelex.identifier = @"simple_telex";
    [content addSubview:self.simpleTelex];

    self.autoRestore = [self checkbox:@"Auto Restore"
                                frame:NSMakeRect(24, 158, 240, 24)
                               action:@selector(optionChanged:)];
    self.autoRestore.identifier = @"auto_restore";
    [content addSubview:self.autoRestore];

    self.smartCorrection = [self checkbox:@"Smart correction"
                                    frame:NSMakeRect(24, 124, 240, 24)
                                   action:@selector(optionChanged:)];
    self.smartCorrection.identifier = @"smart_correction";
    [content addSubview:self.smartCorrection];

    NSTextField *boundary = [self
        label:@"Space commits the current token. Shift+Space keeps the physical key sequence and ends composition without inserting a space."
        frame:NSMakeRect(24, 58, 406, 50)];
    boundary.maximumNumberOfLines = 3;
    boundary.lineBreakMode = NSLineBreakByWordWrapping;
    boundary.textColor = NSColor.secondaryLabelColor;
    [content addSubview:boundary];

    NSButton *close = [[NSButton alloc] initWithFrame:NSMakeRect(340, 20, 90, 30)];
    close.title = @"Close";
    close.bezelStyle = NSBezelStyleRounded;
    close.target = self;
    close.action = @selector(closeWindow:);
    [content addSubview:close];
}

- (NSDictionary *)languageMetadata:(NSString *)languageID {
    for (NSDictionary *language in self.languages) {
        if ([language[@"id"] isEqualToString:languageID]) return language;
    }
    return self.languages.firstObject;
}

- (NSString *)defaultsKeyForOption:(NSString *)optionID {
    if ([optionID isEqualToString:@"simple_telex"]) return @"InputKeySimpleTelex";
    if ([optionID isEqualToString:@"auto_restore"]) return @"InputKeyAutoRestore";
    if ([optionID isEqualToString:@"smart_correction"]) return @"InputKeySmartCorrection";
    return [@"InputKeyOption." stringByAppendingString:optionID ?: @""];
}

- (BOOL)language:(NSDictionary *)language hasOption:(NSString *)optionID {
    for (NSDictionary *option in language[@"options"] ?: @[]) {
        if ([option[@"id"] isEqualToString:optionID]) return YES;
    }
    return NO;
}

- (BOOL)defaultForOption:(NSString *)optionID language:(NSDictionary *)language {
    for (NSDictionary *option in language[@"options"] ?: @[]) {
        if ([option[@"id"] isEqualToString:optionID]) {
            return [option[@"defaultEnabled"] boolValue];
        }
    }
    return NO;
}

- (void)refreshControls {
    NSUserDefaults *defaults = [NSUserDefaults standardUserDefaults];
    NSString *languageID = [defaults stringForKey:@"InputKeyLanguage"] ?: @"vi";
    NSDictionary *language = [self languageMetadata:languageID];
    if (language == nil) return;

    [self.languagePopup removeAllItems];
    NSInteger languageIndex = 0;
    NSInteger selectedLanguage = 0;
    for (NSDictionary *candidate in self.languages) {
        NSString *title = candidate[@"nativeName"] ?: candidate[@"displayName"] ?: candidate[@"id"];
        [self.languagePopup addItemWithTitle:title ?: @""];
        self.languagePopup.lastItem.representedObject = candidate[@"id"];
        if ([candidate[@"id"] isEqualToString:language[@"id"]]) {
            selectedLanguage = languageIndex;
        }
        languageIndex += 1;
    }
    [self.languagePopup selectItemAtIndex:selectedLanguage];

    [self.methodPopup removeAllItems];
    NSString *methodID = [defaults stringForKey:@"InputKeyMethod"] ?: language[@"defaultMethod"];
    NSInteger methodIndex = 0;
    NSInteger selectedMethod = 0;
    for (NSDictionary *method in language[@"methods"] ?: @[]) {
        [self.methodPopup addItemWithTitle:method[@"label"] ?: method[@"id"] ?: @""];
        self.methodPopup.lastItem.representedObject = method[@"id"];
        if ([method[@"id"] isEqualToString:methodID]) selectedMethod = methodIndex;
        methodIndex += 1;
    }
    if (self.methodPopup.numberOfItems > 0) {
        [self.methodPopup selectItemAtIndex:selectedMethod];
    }

    NSArray<NSButton *> *buttons = @[self.simpleTelex, self.autoRestore, self.smartCorrection];
    for (NSButton *button in buttons) {
        NSString *optionID = button.identifier;
        BOOL visible = [self language:language hasOption:optionID];
        button.hidden = !visible;
        if (visible) {
            NSString *key = [self defaultsKeyForOption:optionID];
            if ([defaults objectForKey:key] == nil) {
                [defaults setBool:[self defaultForOption:optionID language:language] forKey:key];
            }
            button.state = [defaults boolForKey:key] ? NSControlStateValueOn : NSControlStateValueOff;
        }
    }
}

- (void)notifySettingsChanged {
    [[NSNotificationCenter defaultCenter]
        postNotificationName:InputKeySettingsDidChangeNotification
                      object:self];
}

- (void)languageChanged:(id)sender {
    (void)sender;
    NSString *languageID = self.languagePopup.selectedItem.representedObject;
    NSDictionary *language = [self languageMetadata:languageID];
    if (language == nil) return;

    NSUserDefaults *defaults = [NSUserDefaults standardUserDefaults];
    [defaults setObject:language[@"id"] forKey:@"InputKeyLanguage"];
    [defaults setObject:language[@"defaultMethod"] ?: @"" forKey:@"InputKeyMethod"];
    for (NSDictionary *option in language[@"options"] ?: @[]) {
        NSString *optionID = option[@"id"];
        if (optionID.length) {
            [defaults setBool:[option[@"defaultEnabled"] boolValue]
                       forKey:[self defaultsKeyForOption:optionID]];
        }
    }
    [self refreshControls];
    [self notifySettingsChanged];
}

- (void)methodChanged:(id)sender {
    (void)sender;
    NSString *methodID = self.methodPopup.selectedItem.representedObject;
    if (!methodID.length) return;
    [[NSUserDefaults standardUserDefaults] setObject:methodID forKey:@"InputKeyMethod"];
    [self notifySettingsChanged];
}

- (void)optionChanged:(NSButton *)sender {
    NSString *optionID = sender.identifier;
    if (!optionID.length) return;
    [[NSUserDefaults standardUserDefaults]
        setBool:(sender.state == NSControlStateValueOn)
         forKey:[self defaultsKeyForOption:optionID]];
    [self notifySettingsChanged];
}

- (void)closeWindow:(id)sender {
    (void)sender;
    [self.window orderOut:nil];
}

- (void)show {
    [self loadCatalog];
    [self refreshControls];
    [NSApp setActivationPolicy:NSApplicationActivationPolicyAccessory];
    [NSApp activateIgnoringOtherApps:YES];
    [self.window center];
    [self.window makeKeyAndOrderFront:nil];
}

@end
