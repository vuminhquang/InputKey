#define _POSIX_C_SOURCE 200809L
#include "shortcut_config.h"
#include <ctype.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>

static InputKeyShortcut cached = { INPUTKEY_SHORTCUT_CTRL, ' ', 1 };
static struct stat cached_stat;
static int have_stat;

void inputkey_shortcut_default(InputKeyShortcut *s) {
    if (s) { s->modifiers = INPUTKEY_SHORTCUT_CTRL; s->key = ' '; s->enabled = 1; }
}

static int equal_ci(const char *a, size_t n, const char *b) {
    size_t i;
    if (strlen(b) != n) return 0;
    for (i = 0; i < n; ++i) if (tolower((unsigned char)a[i]) != tolower((unsigned char)b[i])) return 0;
    return 1;
}

int inputkey_shortcut_parse(const char *value, InputKeyShortcut *out) {
    InputKeyShortcut s;
    char buf[128], *p, *token, *save = NULL;
    unsigned seen = 0;
    int key_seen = 0;
    size_t len;
    if (!value || !out) return 0;
    while (isspace((unsigned char)*value)) ++value;
    len = strlen(value);
    while (len && isspace((unsigned char)value[len - 1])) --len;
    if (len == 8 && equal_ci(value, len, "disabled")) { out->modifiers = 0; out->key = 0; out->enabled = 0; return 1; }
    if (!len || len >= sizeof(buf)) return 0;
    memcpy(buf, value, len); buf[len] = 0;
    s.enabled = 1; s.modifiers = 0; s.key = 0;
    for (p = buf; *p; ++p) if (*p == '+') *p = ' ';
    for (token = strtok_r(buf, " \t", &save); token; token = strtok_r(NULL, " \t", &save)) {
        size_t n = strlen(token); unsigned bit = 0;
        if (equal_ci(token,n,"ctrl") || equal_ci(token,n,"control")) bit = INPUTKEY_SHORTCUT_CTRL;
        else if (equal_ci(token,n,"alt")) bit = INPUTKEY_SHORTCUT_ALT;
        else if (equal_ci(token,n,"shift")) bit = INPUTKEY_SHORTCUT_SHIFT;
        else if (equal_ci(token,n,"super")) bit = INPUTKEY_SHORTCUT_SUPER;
        if (bit) { if (seen & bit) return 0; seen |= bit; s.modifiers |= bit; continue; }
        if (key_seen) return 0;
        key_seen = 1;
        if (equal_ci(token,n,"space")) s.key = ' ';
        else if (equal_ci(token,n,"semicolon")) s.key = ';';
        else if (n == 1 && (unsigned char)token[0] >= 0x21 && (unsigned char)token[0] <= 0x7e) s.key = (unsigned char)token[0];
        else return 0;
    }
    if (!s.modifiers || !key_seen) return 0;
    *out = s;
    return 1;
}

int inputkey_shortcut_matches(const InputKeyShortcut *s, unsigned modifiers, unsigned char key) {
    if (!s || !s->enabled || !s->modifiers) return 0;
    if (key >= 'A' && key <= 'Z') key = (unsigned char)(key + ('a' - 'A'));
    { unsigned char configured = s->key;
      if (configured >= 'A' && configured <= 'Z') configured = (unsigned char)(configured + ('a' - 'A'));
      return modifiers == s->modifiers && key == configured; }
}

static void config_path(char *path, size_t cap) {
    const char *base = getenv("XDG_CONFIG_HOME"), *home;
    if (base && *base) snprintf(path, cap, "%s/inputkey/config", base);
    else { home = getenv("HOME"); if (home && *home) snprintf(path, cap, "%s/.config/inputkey/config", home); else path[0] = 0; }
}

void inputkey_shortcut_reload(void) {
    char path[4096], line[512]; struct stat st; FILE *f;
    config_path(path, sizeof(path));
    if (!path[0] || stat(path, &st) != 0) { inputkey_shortcut_default(&cached); have_stat = 0; return; }
    if (have_stat && st.st_mtime == cached_stat.st_mtime && st.st_size == cached_stat.st_size) return;
    inputkey_shortcut_default(&cached);
    cached_stat = st; have_stat = 1;
    f = fopen(path, "r"); if (!f) return;
    while (fgets(line, sizeof(line), f)) {
        char *p = line, *eq;
        while (isspace((unsigned char)*p)) ++p;
        if (*p == '#' || !*p) continue;
        eq = strchr(p, '='); if (!eq) continue;
        *eq++ = 0;
        while (isspace((unsigned char)*eq)) ++eq;
        { char *end = p + strlen(p); while (end > p && isspace((unsigned char)end[-1])) *--end = 0; }
        if (strcmp(p, "literalize_shortcut") == 0) { InputKeyShortcut parsed; if (inputkey_shortcut_parse(eq, &parsed)) cached = parsed; break; }
    }
    fclose(f);
}

int inputkey_shortcut_match_configured(unsigned modifiers, unsigned char key) {
    return inputkey_shortcut_matches(&cached, modifiers, key);
}
