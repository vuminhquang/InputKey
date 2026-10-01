#ifndef INPUTKEY_LINUX_SHORTCUT_CONFIG_H
#define INPUTKEY_LINUX_SHORTCUT_CONFIG_H

#ifdef __cplusplus
extern "C" {
#endif

enum {
    INPUTKEY_SHORTCUT_CTRL = 1u << 0,
    INPUTKEY_SHORTCUT_ALT = 1u << 1,
    INPUTKEY_SHORTCUT_SHIFT = 1u << 2,
    INPUTKEY_SHORTCUT_SUPER = 1u << 3
};

typedef struct {
    unsigned modifiers;
    unsigned char key;
    int enabled;
} InputKeyShortcut;

void inputkey_shortcut_default(InputKeyShortcut *shortcut);
int inputkey_shortcut_parse(const char *value, InputKeyShortcut *shortcut);
int inputkey_shortcut_matches(const InputKeyShortcut *shortcut, unsigned modifiers, unsigned char key);
void inputkey_shortcut_reload(void);
int inputkey_shortcut_match_configured(unsigned modifiers, unsigned char key);

#ifdef __cplusplus
}
#endif
#endif
