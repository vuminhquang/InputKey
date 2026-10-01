#include "shortcut_config.h"
#include <assert.h>

int main(void) {
    InputKeyShortcut s;
    assert(inputkey_shortcut_parse("Ctrl+Space", &s));
    assert(inputkey_shortcut_matches(&s, INPUTKEY_SHORTCUT_CTRL, ' '));
    assert(!inputkey_shortcut_matches(&s, INPUTKEY_SHORTCUT_CTRL | INPUTKEY_SHORTCUT_SHIFT, ' '));
    assert(inputkey_shortcut_parse("Control+Shift+Semicolon", &s));
    assert(inputkey_shortcut_matches(&s, INPUTKEY_SHORTCUT_CTRL | INPUTKEY_SHORTCUT_SHIFT, ';'));
    assert(inputkey_shortcut_parse("Alt+q", &s));
    assert(inputkey_shortcut_matches(&s, INPUTKEY_SHORTCUT_ALT, 'Q'));
    assert(inputkey_shortcut_parse("Super+;", &s));
    assert(inputkey_shortcut_matches(&s, INPUTKEY_SHORTCUT_SUPER, ';'));
    assert(inputkey_shortcut_parse("disabled", &s));
    assert(!inputkey_shortcut_matches(&s, INPUTKEY_SHORTCUT_CTRL, ' '));
    assert(!inputkey_shortcut_parse("Space", &s));
    assert(!inputkey_shortcut_parse("Ctrl+Alt", &s));
    assert(!inputkey_shortcut_parse("Ctrl+Ctrl+x", &s));
    return 0;
}
