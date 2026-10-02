#pragma once
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct InputKeyLinuxSettings {
    char language[32];
    char method[32];
    int simple_telex;
    int auto_restore;
    int smart_correction;
} InputKeyLinuxSettings;

void inputkey_linux_settings_load(InputKeyLinuxSettings *out);
void inputkey_linux_settings_save(const InputKeyLinuxSettings *settings);

#ifdef __cplusplus
}
#endif
