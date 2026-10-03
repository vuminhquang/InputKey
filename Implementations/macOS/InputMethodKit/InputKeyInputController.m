#import "InputKeyInputController.h"
#import "InputKeySettingsWindowController.h"

#import <AppKit/AppKit.h>
#import <InputMethodKit/InputMethodKit.h>

#include <stdint.h>
#include <stdlib.h>
#include <string.h>

#include "inputkey.h"

typedef size_t (*InputKeyCommand)(uint64_t, uint8_t *, size_t);
typedef size_t (*InputKeyArgumentCommand)(
    uint64_t, const uint8_t *, size_t, uint8_t *, size_t);

static NSRange InputKeyNoReplacementRange(void) {
    return NSMakeRange(NSNotFound, NSNotFound);
}

static NSString *InputKeyStringFromCommand(uint64_t handle, InputKeyCommand command) {
    const size_t required = command(handle, NULL, 0);
    uint8_t *buffer = calloc(required + 1, sizeof(uint8_t));
    if (buffer == NULL) return @"";
    command(handle, buffer, required + 1);
    NSString *result = [[NSString alloc] initWithBytes:buffer
                                               length:required
                                             encoding:NSUTF8StringEncoding];
    free(buffer);
    return result ?: @"";
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

static NSString *InputKeyStringFromArgument(
    uint64_t handle, InputKeyArgumentCommand command, NSString *argument) {
    NSData *data = [argument dataUsingEncoding:NSUTF8StringEncoding];
    const size_t required = command(handle, data.bytes, data.length, NULL, 0);
    uint8_t *buffer = calloc(required + 1, sizeof(uint8_t));
    if (buffer == NULL) return @"";
    command(handle, data.bytes, data.length, buffer, required + 1);
    NSString *result = [[NSString alloc] initWithBytes:buffer
                                               length:required
                                             encoding:NSUTF8StringEncoding];
    free(buffer);
    return result ?: @"";
}

static NSString *InputKeyStringFromKey(uint64_t handle, NSString *key) {
    return InputKeyStringFromArgument(handle, inputkey_character_utf8, key);
}

static NSString *InputKeyBoundary(uint64_t handle, NSString *delimiter) {
    if ([delimiter isEqualToString:@" "]) {
        return InputKeyStringFromCommand(handle, inputkey_space_boundary);
    }
    return InputKeyStringFromArgument(
        handle, inputkey_punctuation_boundary_utf8, delimiter);
}

static NSString *InputKeyCaretBoundary(uint64_t handle, NSString *cause) {
    return InputKeyStringFromArgument(
        handle, inputkey_caret_move_boundary_utf8, cause);
}

static NSString *InputKeyControl(uint64_t handle, NSString *control) {
    return InputKeyStringFromArgument(
        handle, inputkey_composition_control_utf8, control);
}

static NSString *InputKeyLifecycle(uint64_t handle, NSString *event) {
    return InputKeyStringFromArgument(handle, inputkey_lifecycle_utf8, event);
}

static BOOL InputKeyAcceptsKey(uint64_t handle, NSString *key) {
    NSData *data = [key dataUsingEncoding:NSUTF8StringEncoding];
    return inputkey_accepts_character_utf8(handle, data.bytes, data.length) != 0;
}

@interface InputKeyInputController () {
    uint64_t _core;
}
@end

@implementation InputKeyInputController

- (instancetype)initWithServer:(IMKServer *)server
                      delegate:(id)delegate
                        client:(id)inputClient {
    self = [super initWithServer:server delegate:delegate client:inputClient];
    if (self != nil) {
        [self rebuildCore];
        [[NSNotificationCenter defaultCenter]
            addObserver:self
               selector:@selector(settingsDidChange:)
                   name:InputKeySettingsDidChangeNotification
                 object:nil];
    }
    return self;
}

- (void)dealloc {
    [[NSNotificationCenter defaultCenter] removeObserver:self];
    if (_core != 0) {
        inputkey_destroy(_core);
        _core = 0;
    }
}

- (NSArray<NSDictionary *> *)languages {
    NSData *data = [InputKeyCatalogJSON() dataUsingEncoding:NSUTF8StringEncoding];
    if (data.length == 0) return @[];
    NSDictionary *root = [NSJSONSerialization JSONObjectWithData:data options:0 error:nil];
    NSArray *languages = [root isKindOfClass:[NSDictionary class]] ? root[@"languages"] : nil;
    return [languages isKindOfClass:[NSArray class]] ? languages : @[];
}

- (NSDictionary *)languageMetadata:(NSString *)languageID {
    for (NSDictionary *language in [self languages]) {
        if ([language[@"id"] isEqualToString:languageID]) return language;
    }
    return [self languages].firstObject;
}

- (NSString *)configuredLanguage {
    NSString *language = [[NSUserDefaults standardUserDefaults] stringForKey:@"InputKeyLanguage"];
    return language.length ? language : @"vi";
}

- (NSString *)configuredMethod {
    NSUserDefaults *defaults = [NSUserDefaults standardUserDefaults];
    NSString *method = [defaults stringForKey:@"InputKeyMethod"];
    NSDictionary *language = [self languageMetadata:[self configuredLanguage]];
    NSArray *methods = language[@"methods"];
    for (NSDictionary *candidate in methods) {
        if ([candidate[@"id"] isEqualToString:method]) return method;
    }
    NSString *fallback = language[@"defaultMethod"];
    return fallback.length ? fallback : @"telex";
}

- (NSString *)defaultsKeyForOption:(NSString *)optionID {
    if ([optionID isEqualToString:@"simple_telex"]) return @"InputKeySimpleTelex";
    if ([optionID isEqualToString:@"auto_restore"]) return @"InputKeyAutoRestore";
    if ([optionID isEqualToString:@"smart_correction"]) return @"InputKeySmartCorrection";
    return [@"InputKeyOption." stringByAppendingString:optionID ?: @""];
}

- (NSString *)optionsJSON {
    NSDictionary *language = [self languageMetadata:[self configuredLanguage]];
    NSMutableDictionary *values = [NSMutableDictionary dictionary];
    NSUserDefaults *defaults = [NSUserDefaults standardUserDefaults];
    for (NSDictionary *option in language[@"options"] ?: @[]) {
        NSString *optionID = option[@"id"];
        if (!optionID.length) continue;
        NSString *key = [self defaultsKeyForOption:optionID];
        if ([defaults objectForKey:key] == nil) {
            [defaults setBool:[option[@"defaultEnabled"] boolValue] forKey:key];
        }
        values[optionID] = @([defaults boolForKey:key]);
    }
    NSData *data = [NSJSONSerialization dataWithJSONObject:values options:0 error:nil];
    return data ? [[NSString alloc] initWithData:data encoding:NSUTF8StringEncoding] : @"{}";
}

- (void)rebuildCore {
    if (_core != 0) inputkey_destroy(_core);
    NSString *language = [self configuredLanguage];
    NSString *method = [self configuredMethod];
    NSString *options = [self optionsJSON];
    _core = inputkey_create_ex(language.UTF8String, method.UTF8String, options.UTF8String);
}

- (void)settingsDidChange:(NSNotification *)notification {
    (void)notification;
    [self commitCurrentTokenFinalizing:YES client:[self client]];
    [self rebuildCore];
}

- (void)showPreferences:(id)sender {
    (void)sender;
    [[InputKeySettingsWindowController sharedController] show];
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
    if (![self hasActiveToken]) return;
    NSString *text = finalize
        ? InputKeyLifecycle(_core, @"finalize")
        : InputKeyCaretBoundary(_core, @"other");
    [self commitText:text client:sender];
}

- (BOOL)isNavigationKeyCode:(unsigned short)keyCode {
    switch (keyCode) {
        case 115: case 116: case 117: case 119: case 121:
        case 123: case 124: case 125: case 126:
            return YES;
        default:
            return NO;
    }
}

- (NSString *)caretCauseForKeyCode:(unsigned short)keyCode {
    switch (keyCode) {
        case 123: return @"left";
        case 124: return @"right";
        case 126: return @"up";
        case 125: return @"down";
        case 115: return @"home";
        case 119: return @"end";
        case 116: return @"page_up";
        case 121: return @"page_down";
        case 117: return @"delete";
        case 48: return @"tab";
        case 36: case 76: return @"enter";
        default: return @"other";
    }
}

- (BOOL)mouseDownOnCharacterIndex:(NSUInteger)index
                          coordinate:(NSPoint)point
                        withModifier:(NSUInteger)flags
                    continueTracking:(BOOL *)keepTracking
                              client:(id)sender {
    (void)index;
    (void)point;
    (void)flags;
    if (keepTracking != NULL) *keepTracking = NO;
    if (![self hasActiveToken]) return NO;
    NSString *text = InputKeyCaretBoundary(_core, @"mouse");
    [self commitText:text client:sender];
    return NO;
}

- (BOOL)handleEvent:(NSEvent *)event client:(id)sender {
    if (event.type != NSEventTypeKeyDown) return NO;

    NSEventModifierFlags flags =
        event.modifierFlags & NSEventModifierFlagDeviceIndependentFlagsMask;
    const BOOL control = (flags & NSEventModifierFlagControl) != 0;
    const BOOL option = (flags & NSEventModifierFlagOption) != 0;
    const BOOL command = (flags & NSEventModifierFlagCommand) != 0;
    const BOOL shift = (flags & NSEventModifierFlagShift) != 0;

    if (event.keyCode == 49 && shift && !control && !option && !command && [self hasActiveToken]) {
        NSString *raw = InputKeyStringFromCommand(_core, inputkey_raw_boundary);
        [self commitText:raw client:sender];
        return YES;
    }

    if (control || command) {
        if ([self hasActiveToken]) {
            [self commitText:InputKeyStringFromCommand(_core, inputkey_shortcut_boundary)
                       client:sender];
        }
        return NO;
    }

    if (event.keyCode == 51) {
        if (![self hasActiveToken]) return NO;
        [self setMarkedText:InputKeyControl(_core, @"backspace") client:sender];
        return YES;
    }

    if (event.keyCode == 53) {
        if (![self hasActiveToken]) return NO;
        [self setMarkedText:InputKeyControl(_core, @"escape") client:sender];
        return YES;
    }

    if ([self isNavigationKeyCode:event.keyCode]
        || event.keyCode == 48 || event.keyCode == 36 || event.keyCode == 76) {
        if ([self hasActiveToken]) {
            [self commitText:InputKeyCaretBoundary(
                _core, [self caretCauseForKeyCode:event.keyCode]) client:sender];
        }
        return NO;
    }

    NSString *characters = event.characters ?: @"";
    if (characters.length != 1) {
        [self commitCurrentTokenFinalizing:NO client:sender];
        return NO;
    }

    if (InputKeyAcceptsKey(_core, characters)) {
        [self setMarkedText:InputKeyStringFromKey(_core, characters) client:sender];
        return YES;
    }

    if ([self hasActiveToken]) {
        [self commitText:InputKeyBoundary(_core, characters) client:sender];
        return YES;
    }

    return NO;
}

- (void)commitComposition:(id)sender {
    [self commitCurrentTokenFinalizing:YES client:sender];
}

- (void)inputControllerWillClose {
    if (_core != 0) {
        (void)InputKeyLifecycle(_core, @"context_destroyed");
    }
    [super inputControllerWillClose];
}

- (NSMenu *)menu {
    NSMenu *menu = [[NSMenu alloc] initWithTitle:@"InputKey"];
    NSString *currentLanguage = [self configuredLanguage];
    NSDictionary *activeLanguage = [self languageMetadata:currentLanguage];
    NSString *currentMethod = [self configuredMethod];

    NSMenu *languageMenu = [[NSMenu alloc] initWithTitle:@"Language"];
    for (NSDictionary *language in [self languages]) {
        NSString *title = language[@"nativeName"] ?: language[@"displayName"] ?: language[@"id"];
        NSMenuItem *item = [[NSMenuItem alloc] initWithTitle:title
                                                      action:@selector(selectLanguage:)
                                               keyEquivalent:@""];
        item.target = self;
        item.representedObject = language;
        item.state = [language[@"id"] isEqualToString:currentLanguage]
            ? NSControlStateValueOn : NSControlStateValueOff;
        [languageMenu addItem:item];
    }
    NSMenuItem *languageRoot = [[NSMenuItem alloc] initWithTitle:@"Language" action:nil keyEquivalent:@""];
    languageRoot.submenu = languageMenu;
    [menu addItem:languageRoot];

    NSMenu *methodMenu = [[NSMenu alloc] initWithTitle:@"Method"];
    for (NSDictionary *method in activeLanguage[@"methods"] ?: @[]) {
        NSMenuItem *item = [[NSMenuItem alloc] initWithTitle:method[@"label"] ?: method[@"id"]
                                                      action:@selector(selectMethod:)
                                               keyEquivalent:@""];
        item.target = self;
        item.representedObject = method[@"id"];
        item.state = [method[@"id"] isEqualToString:currentMethod]
            ? NSControlStateValueOn : NSControlStateValueOff;
        [methodMenu addItem:item];
    }
    NSMenuItem *methodRoot = [[NSMenuItem alloc] initWithTitle:@"Method" action:nil keyEquivalent:@""];
    methodRoot.submenu = methodMenu;
    [menu addItem:methodRoot];

    for (NSDictionary *option in activeLanguage[@"options"] ?: @[]) {
        NSString *optionID = option[@"id"];
        if (!optionID.length) continue;
        NSMenuItem *item = [[NSMenuItem alloc] initWithTitle:option[@"label"] ?: optionID
                                                      action:@selector(toggleOption:)
                                               keyEquivalent:@""];
        item.target = self;
        item.representedObject = optionID;
        item.state = [[NSUserDefaults standardUserDefaults]
            boolForKey:[self defaultsKeyForOption:optionID]]
            ? NSControlStateValueOn : NSControlStateValueOff;
        [menu addItem:item];
    }
    [menu addItem:[NSMenuItem separatorItem]];
    NSMenuItem *preferences = [[NSMenuItem alloc] initWithTitle:@"InputKey Settings..."
                                                         action:@selector(showPreferences:)
                                                  keyEquivalent:@""];
    preferences.target = self;
    [menu addItem:preferences];
    return menu;
}

- (void)selectLanguage:(NSMenuItem *)sender {
    NSDictionary *language = sender.representedObject;
    NSString *languageID = language[@"id"];
    if (!languageID.length) return;
    [self commitCurrentTokenFinalizing:YES client:[self client]];
    NSUserDefaults *defaults = [NSUserDefaults standardUserDefaults];
    [defaults setObject:languageID forKey:@"InputKeyLanguage"];
    [defaults setObject:language[@"defaultMethod"] ?: @"" forKey:@"InputKeyMethod"];
    for (NSDictionary *option in language[@"options"] ?: @[]) {
        NSString *optionID = option[@"id"];
        if (optionID.length) {
            [defaults setBool:[option[@"defaultEnabled"] boolValue]
                       forKey:[self defaultsKeyForOption:optionID]];
        }
    }
    [self rebuildCore];
}

- (void)selectMethod:(NSMenuItem *)sender {
    NSString *method = sender.representedObject;
    if (!method.length || [[self configuredMethod] isEqualToString:method]) return;
    [self commitCurrentTokenFinalizing:YES client:[self client]];
    [[NSUserDefaults standardUserDefaults] setObject:method forKey:@"InputKeyMethod"];
    [self rebuildCore];
}

- (void)toggleOption:(NSMenuItem *)sender {
    NSString *optionID = sender.representedObject;
    if (!optionID.length) return;
    [self commitCurrentTokenFinalizing:YES client:[self client]];
    NSUserDefaults *defaults = [NSUserDefaults standardUserDefaults];
    NSString *key = [self defaultsKeyForOption:optionID];
    [defaults setBool:![defaults boolForKey:key] forKey:key];
    [self rebuildCore];
}

@end
