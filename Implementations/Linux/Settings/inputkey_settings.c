#include "inputkey_settings.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <sys/types.h>

static void copy_value(char *out, size_t cap, const char *value) {
    if (!out || cap == 0) return;
    snprintf(out, cap, "%s", value ? value : "");
}

static int parse_bool(const char *value, int fallback) {
    if (!value) return fallback;
    if (strcmp(value, "1") == 0 || strcmp(value, "true") == 0) return 1;
    if (strcmp(value, "0") == 0 || strcmp(value, "false") == 0) return 0;
    return fallback;
}

static int config_paths(char *dir, size_t dir_cap, char *file, size_t file_cap) {
    const char *xdg = getenv("XDG_CONFIG_HOME");
    if (xdg && *xdg) {
        snprintf(dir, dir_cap, "%s/inputkey", xdg);
    } else {
        const char *home = getenv("HOME");
        if (!home || !*home) return 0;
        snprintf(dir, dir_cap, "%s/.config/inputkey", home);
    }
    snprintf(file, file_cap, "%s/settings.conf", dir);
    return 1;
}

void inputkey_linux_settings_load(InputKeyLinuxSettings *out) {
    if (!out) return;
    memset(out, 0, sizeof(*out));
    copy_value(out->language, sizeof(out->language), "vi");
    copy_value(out->method, sizeof(out->method), "telex");
    out->simple_telex = 0;
    out->auto_restore = 1;
    out->smart_correction = 1;

    char dir[1024], path[1200];
    if (!config_paths(dir, sizeof(dir), path, sizeof(path))) return;
    FILE *f = fopen(path, "r");
    if (!f) return;

    char line[256];
    while (fgets(line, sizeof(line), f)) {
        char *newline = strpbrk(line, "\r\n");
        if (newline) *newline = 0;
        if (strncmp(line, "language=", 9) == 0) {
            copy_value(out->language, sizeof(out->language), line + 9);
        } else if (strncmp(line, "method=", 7) == 0) {
            copy_value(out->method, sizeof(out->method), line + 7);
        } else if (strncmp(line, "simple_telex=", 13) == 0) {
            out->simple_telex = parse_bool(line + 13, out->simple_telex);
        } else if (strncmp(line, "auto_restore=", 13) == 0) {
            out->auto_restore = parse_bool(line + 13, out->auto_restore);
        } else if (strncmp(line, "smart_correction=", 17) == 0) {
            out->smart_correction = parse_bool(line + 17, out->smart_correction);
        }
    }
    fclose(f);
}

void inputkey_linux_settings_save(const InputKeyLinuxSettings *settings) {
    if (!settings) return;
    char dir[1024], path[1200];
    if (!config_paths(dir, sizeof(dir), path, sizeof(path))) return;

    char parent[1024];
    snprintf(parent, sizeof(parent), "%s", dir);
    char *slash = strrchr(parent, '/');
    if (slash) {
        *slash = 0;
        if (*parent) (void)mkdir(parent, 0700);
    }
    (void)mkdir(dir, 0700);

    FILE *f = fopen(path, "w");
    if (!f) return;
    fprintf(f, "language=%s\nmethod=%s\nsimple_telex=%d\nauto_restore=%d\nsmart_correction=%d\n",
            settings->language[0] ? settings->language : "vi",
            settings->method[0] ? settings->method : "telex",
            settings->simple_telex != 0,
            settings->auto_restore != 0,
            settings->smart_correction != 0);
    fclose(f);
}
