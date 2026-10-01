#import "InputKeyInputController.h"

#import <AppKit/AppKit.h>
#import <InputMethodKit/InputMethodKit.h>

#include <stdint.h>
#include <stdlib.h>
#include <string.h>

#include "inputkey.h"

typedef size_t (*InputKeyCommand)(uint64_t, uint8_t *, size_t);

static NSRange InputKeyNoReplacementRange(void) {
    return NSMakeRange(NSNotFound, NSNotFound);
}

static NSString *InputKeyStringFromCommand(uint64_t handle, InputKeyCommand command) {
    const size_t required = command(handle, NULL, 0);
    uint8_t *buffer = calloc(required + 1, sizeof(uint8_t));
    if (buffer == NULL) {
        return @"";
    }
    command(handle, buffer, required + 1);
    NSString *result = [[NSString alloc] initWithBytes:buffer
                                               length:required
                                             encoding:NSUTF8StringEncoding];
    free(buffer);
    return result ?: @"";
}

static NSString *InputKeyStringFromKey(uint64_t handle, NSString *key) {
    NSData *data = [key dataUsingEncoding:NSUTF8StringEncoding];
    const uint8_t *bytes = data.bytes;
    const size_t length = data.length;
    const size_t required = inputkey_key_utf8(handle, bytes, length, NULL, 0);
    uint8_t *buffer = calloc(required + 1, sizeof(uint8_t));
    if (buffer == NULL) {
        return @"";
    }
    inputkey_key_utf8(handle, bytes, length, buffer, required + 1);
    NSString *result = [[NSString alloc] initWithBytes:buffer
                                               length:required
                                             encoding:NSUTF8StringEncoding];
    free(buffer);
    return result ?: @"";
}

@interface InputKeyInputController () {
    uint64_t _core;
    BOOL _recordingLiteralizeShortcut;
}
@end

static NSString *const InputKeyShortcutEnabled = @"InputKeyLiteralizeShortcutEnabled";
static NSString *const InputKeyShortcutCode = @"InputKeyLiteralizeShortcutKeyCode";
static NSString *const InputKeyShortcutControl = @"InputKeyLiteralizeShortcutControl";
static NSString *const InputKeyShortcutOption = @"InputKeyLiteralizeShortcutOption";
static NSString *const InputKeyShortcutShift = @"InputKeyLiteralizeShortcutShift";
static NSString *const InputKeyShortcutCommand = @"InputKeyLiteralizeShortcutCommand";

@implementation InputKeyInputController

- (instancetype)initWithServer:(IMKServer *)server
                      delegate:(id)delegate
                        client:(id)inputClient {
    self = [super initWithServer:server delegate:delegate client:inputClient];
    if (self != nil) {
        [self rebuildCore];
    }
    return self;
}

- (void)dealloc {
    if (_core != 0) {
        inputkey_destroy(_core);
        _core = 0;
    }
}

- (NSString *)configuredMethod {
    NSString *method = [[NSUserDefaults standardUserDefaults] stringForKey:@"InputKeyMethod"];
    return [method isEqualToString:@"vni"] ? @"vni" : @"telex";
}

- (void)rebuildCore {
    if (_core != 0) {
        inputkey_destroy(_core);
    }
    NSUserDefaults *defaults = [NSUserDefaults standardUserDefaults];
    NSString *method = [self configuredMethod];
    const int simpleTelex = [defaults boolForKey:@"InputKeySimpleTelex"] ? 1 : 0;
    const int autoRestore = [defaults boolForKey:@"InputKeyAutoRestore"] ? 1 : 0;
    _core = inputkey_create(method.UTF8String, simpleTelex, autoRestore);
}

- (BOOL)hasActiveToken {
    return _core != 0 && inputkey_has_history(_core) != 0;
}

- (void)setMarkedText:(NSString *)text client:(id)sender {
    id<NSTextInputClient> client = (id<NSTextInputClient>)sender;
    [client setMarkedText:text
            selectedRange:NSMakeRange(text.length, 0)
         replacementRange:InputKeyNoReplacementRange()];
}

- (void)commitText:(NSString *)text client:(id)sender {
    id<NSTextInputClient> client = (id<NSTextInputClient>)sender;
    [client insertText:text replacementRange:InputKeyNoReplacementRange()];
}

- (void)commitCurrentTokenFinalizing:(BOOL)finalize client:(id)sender {
    if (![self hasActiveToken]) {
        return;
    }
    NSString *text = finalize
        ? InputKeyStringFromCommand(_core, inputkey_finalize)
        : InputKeyStringFromCommand(_core, inputkey_rendered);
    [self commitText:text client:sender];
    inputkey_reset(_core);
}

- (BOOL)isTokenCharacter:(unichar)c {
    if ([[self configuredMethod] isEqualToString:@"vni"]) {
        return (c >= 'A' && c <= 'Z') || (c >= 'a' && c <= 'z') || (c >= '0' && c <= '9');
    }
    return (c >= 'A' && c <= 'Z') || (c >= 'a' && c <= 'z') || c == '[' || c == ']';
}

- (BOOL)isNavigationKeyCode:(unsigned short)keyCode {
    switch (keyCode) {
        case 115: // Home
        case 116: // Page Up
        case 117: // Forward Delete
        case 119: // End
        case 121: // Page Down
        case 123: // Left
        case 124: // Right
        case 125: // Down
        case 126: // Up
            return YES;
        default:
            return NO;
    }
}

- (NSEventModifierFlags)configuredShortcutModifiers {
    NSUserDefaults *defaults = [NSUserDefaults standardUserDefaults];
    NSEventModifierFlags modifiers = 0;
    if ([defaults boolForKey:InputKeyShortcutControl]) modifiers |= NSEventModifierFlagControl;
    if ([defaults boolForKey:InputKeyShortcutOption]) modifiers |= NSEventModifierFlagOption;
    if ([defaults boolForKey:InputKeyShortcutShift]) modifiers |= NSEventModifierFlagShift;
    if ([defaults boolForKey:InputKeyShortcutCommand]) modifiers |= NSEventModifierFlagCommand;
    return modifiers;
}

- (NSString *)shortcutLabel {
    NSUserDefaults *defaults = [NSUserDefaults standardUserDefaults];
    NSMutableString *label = [NSMutableString string];
    if ([defaults boolForKey:InputKeyShortcutControl]) [label appendString:@"⌃"];
    if ([defaults boolForKey:InputKeyShortcutOption]) [label appendString:@"⌥"];
    if ([defaults boolForKey:InputKeyShortcutShift]) [label appendString:@"⇧"];
    if ([defaults boolForKey:InputKeyShortcutCommand]) [label appendString:@"⌘"];
    unsigned short code = (unsigned short)[defaults integerForKey:InputKeyShortcutCode];
    NSString *key = code == 41 ? @";" : (code == 49 ? @"Space" : [NSString stringWithFormat:@"Key %hu", code]);
    [label appendString:key];
    return label;
}

- (BOOL)recordShortcutFromEvent:(NSEvent *)event {
    NSEventModifierFlags flags = event.modifierFlags & NSEventModifierFlagDeviceIndependentFlagsMask;
    if (event.keyCode == 53) {
        _recordingLiteralizeShortcut = NO;
        return YES;
    }
    // Modifier-only key events do not select the key; wait for the next non-modifier.
    if (event.keyCode == 54 || event.keyCode == 55 || event.keyCode == 56 || event.keyCode == 57 ||
        event.keyCode == 58 || event.keyCode == 59 || event.keyCode == 60 || event.keyCode == 61 ||
        event.keyCode == 62 || event.keyCode == 63) return YES;
    NSEventModifierFlags modifiers = flags & (NSEventModifierFlagControl | NSEventModifierFlagOption |
        NSEventModifierFlagShift | NSEventModifierFlagCommand);
    if (modifiers == 0) return YES;
    NSUserDefaults *defaults = [NSUserDefaults standardUserDefaults];
    [defaults setInteger:event.keyCode forKey:InputKeyShortcutCode];
    [defaults setBool:(modifiers & NSEventModifierFlagControl) != 0 forKey:InputKeyShortcutControl];
    [defaults setBool:(modifiers & NSEventModifierFlagOption) != 0 forKey:InputKeyShortcutOption];
    [defaults setBool:(modifiers & NSEventModifierFlagShift) != 0 forKey:InputKeyShortcutShift];
    [defaults setBool:(modifiers & NSEventModifierFlagCommand) != 0 forKey:InputKeyShortcutCommand];
    [defaults setBool:YES forKey:InputKeyShortcutEnabled];
    _recordingLiteralizeShortcut = NO;
    return YES;
}

- (BOOL)handleEvent:(NSEvent *)event client:(id)sender {
    if (event.type != NSEventTypeKeyDown) {
        return NO;
    }

    if (_recordingLiteralizeShortcut) return [self recordShortcutFromEvent:event];

    NSEventModifierFlags flags =
        event.modifierFlags & NSEventModifierFlagDeviceIndependentFlagsMask;
    const BOOL control = (flags & NSEventModifierFlagControl) != 0;
    const BOOL option = (flags & NSEventModifierFlagOption) != 0;
    const BOOL command = (flags & NSEventModifierFlagCommand) != 0;

    NSUserDefaults *defaults = [NSUserDefaults standardUserDefaults];
    NSEventModifierFlags shortcutMask = NSEventModifierFlagControl | NSEventModifierFlagOption |
        NSEventModifierFlagShift | NSEventModifierFlagCommand;
    NSEventModifierFlags actualModifiers = flags & shortcutMask;
    if ([defaults boolForKey:InputKeyShortcutEnabled] &&
        event.keyCode == (unsigned short)[defaults integerForKey:InputKeyShortcutCode] &&
        actualModifiers == [self configuredShortcutModifiers]) {
        if ([self hasActiveToken]) {
            NSString *raw = InputKeyStringFromCommand(_core, inputkey_literalize_token);
            [self setMarkedText:raw client:sender];
            return YES;
        }
        return NO;
    }

    if (control || option || command) {
        [self commitCurrentTokenFinalizing:YES client:sender];
        return NO;
    }

    if (event.keyCode == 51) { // Backspace
        if (![self hasActiveToken]) {
            return NO;
        }
        NSString *next = InputKeyStringFromCommand(_core, inputkey_backspace);
        [self setMarkedText:next client:sender];
        return YES;
    }

    if (event.keyCode == 53) { // Escape
        if (![self hasActiveToken]) {
            return NO;
        }
        NSString *next = InputKeyStringFromCommand(_core, inputkey_escape);
        [self setMarkedText:next client:sender];
        return YES;
    }

    if ([self isNavigationKeyCode:event.keyCode]) {
        [self commitCurrentTokenFinalizing:NO client:sender];
        return NO;
    }

    // Tab / Return / keypad Enter finalize the token, then continue to the client.
    if (event.keyCode == 48 || event.keyCode == 36 || event.keyCode == 76) {
        [self commitCurrentTokenFinalizing:YES client:sender];
        return NO;
    }

    NSString *characters = event.characters ?: @"";
    if (characters.length != 1) {
        [self commitCurrentTokenFinalizing:YES client:sender];
        return NO;
    }

    const unichar c = [characters characterAtIndex:0];
    if (c > 0x7f || ![self isTokenCharacter:c]) {
        [self commitCurrentTokenFinalizing:YES client:sender];
        return NO;
    }

    NSString *next = InputKeyStringFromKey(_core, characters);
    [self setMarkedText:next client:sender];
    return YES;
}

- (void)commitComposition:(id)sender {
    [self commitCurrentTokenFinalizing:YES client:sender];
}

- (void)inputControllerWillClose {
    if (_core != 0) {
        inputkey_reset(_core);
    }
    [super inputControllerWillClose];
}

- (NSMenu *)menu {
    NSMenu *menu = [[NSMenu alloc] initWithTitle:@"InputKey"];
    NSString *method = [self configuredMethod];

    NSMenuItem *telex = [[NSMenuItem alloc] initWithTitle:@"Telex"
                                                   action:@selector(selectTelex:)
                                            keyEquivalent:@""];
    telex.target = self;
    telex.state = [method isEqualToString:@"telex"] ? NSControlStateValueOn : NSControlStateValueOff;
    [menu addItem:telex];

    NSMenuItem *vni = [[NSMenuItem alloc] initWithTitle:@"VNI"
                                                 action:@selector(selectVNI:)
                                          keyEquivalent:@""];
    vni.target = self;
    vni.state = [method isEqualToString:@"vni"] ? NSControlStateValueOn : NSControlStateValueOff;
    [menu addItem:vni];

    [menu addItem:[NSMenuItem separatorItem]];

    NSMenuItem *restore = [[NSMenuItem alloc] initWithTitle:@"Auto Restore"
                                                     action:@selector(toggleAutoRestore:)
                                              keyEquivalent:@""];
    restore.target = self;
    restore.state = [[NSUserDefaults standardUserDefaults] boolForKey:@"InputKeyAutoRestore"]
        ? NSControlStateValueOn
        : NSControlStateValueOff;
    [menu addItem:restore];

    [menu addItem:[NSMenuItem separatorItem]];
    NSUserDefaults *defaults = [NSUserDefaults standardUserDefaults];
    NSMenuItem *shortcut = [[NSMenuItem alloc] initWithTitle:[NSString stringWithFormat:@"Hoàn tác dấu của từ: %@", [self shortcutLabel]]
        action:@selector(recordLiteralizeShortcut:) keyEquivalent:@""];
    shortcut.target = self;
    [menu addItem:shortcut];
    NSMenuItem *enabled = [[NSMenuItem alloc] initWithTitle:@"Enable Literalize Shortcut"
        action:@selector(toggleLiteralizeShortcut:) keyEquivalent:@""];
    enabled.target = self;
    enabled.state = [defaults boolForKey:InputKeyShortcutEnabled] ? NSControlStateValueOn : NSControlStateValueOff;
    [menu addItem:enabled];
    NSMenuItem *reset = [[NSMenuItem alloc] initWithTitle:@"Reset shortcut → Control+;"
        action:@selector(resetLiteralizeShortcut:) keyEquivalent:@""];
    reset.target = self;
    [menu addItem:reset];

    return menu;
}

- (void)recordLiteralizeShortcut:(id)sender { (void)sender; _recordingLiteralizeShortcut = YES; }
- (void)toggleLiteralizeShortcut:(id)sender {
    (void)sender;
    NSUserDefaults *defaults = [NSUserDefaults standardUserDefaults];
    [defaults setBool:![defaults boolForKey:InputKeyShortcutEnabled] forKey:InputKeyShortcutEnabled];
}
- (void)resetLiteralizeShortcut:(id)sender {
    (void)sender;
    NSUserDefaults *defaults = [NSUserDefaults standardUserDefaults];
    [defaults setInteger:41 forKey:InputKeyShortcutCode];
    [defaults setBool:YES forKey:InputKeyShortcutControl];
    [defaults setBool:NO forKey:InputKeyShortcutOption];
    [defaults setBool:NO forKey:InputKeyShortcutShift];
    [defaults setBool:NO forKey:InputKeyShortcutCommand];
    [defaults setBool:YES forKey:InputKeyShortcutEnabled];
}

- (void)selectTelex:(id)sender {
    (void)sender;
    if ([[self configuredMethod] isEqualToString:@"telex"]) return;
    [self commitCurrentTokenFinalizing:YES client:[self client]];
    [[NSUserDefaults standardUserDefaults] setObject:@"telex" forKey:@"InputKeyMethod"];
    [self rebuildCore];
}

- (void)selectVNI:(id)sender {
    (void)sender;
    if ([[self configuredMethod] isEqualToString:@"vni"]) return;
    [self commitCurrentTokenFinalizing:YES client:[self client]];
    [[NSUserDefaults standardUserDefaults] setObject:@"vni" forKey:@"InputKeyMethod"];
    [self rebuildCore];
}

- (void)toggleAutoRestore:(id)sender {
    (void)sender;
    [self commitCurrentTokenFinalizing:YES client:[self client]];
    NSUserDefaults *defaults = [NSUserDefaults standardUserDefaults];
    [defaults setBool:![defaults boolForKey:@"InputKeyAutoRestore"]
               forKey:@"InputKeyAutoRestore"];
    [self rebuildCore];
}

@end
