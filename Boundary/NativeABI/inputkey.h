#pragma once
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

#define INPUTKEY_ABI_VERSION 4

uint64_t inputkey_create(const char *method, int simple_telex, int auto_restore);
uint64_t inputkey_create_ex(const char *language, const char *method, const char *options_json);
void inputkey_destroy(uint64_t handle);

/* Return the full UTF-8 byte length. If out/cap are supplied, write a NUL-terminated
   prefix that fits in cap bytes. Call again with a larger buffer if return >= cap. */
size_t inputkey_catalog_json(uint8_t *out, size_t cap);
int inputkey_accepts_character_utf8(uint64_t handle, const uint8_t *bytes, size_t len);
size_t inputkey_character_utf8(uint64_t handle, const uint8_t *bytes, size_t len, uint8_t *out, size_t cap);
size_t inputkey_space_boundary(uint64_t handle, uint8_t *out, size_t cap);
size_t inputkey_punctuation_boundary_utf8(uint64_t handle, const uint8_t *delimiter, size_t len, uint8_t *out, size_t cap);
size_t inputkey_caret_move_boundary_utf8(uint64_t handle, const uint8_t *cause, size_t len, uint8_t *out, size_t cap);
size_t inputkey_shortcut_boundary(uint64_t handle, uint8_t *out, size_t cap);
size_t inputkey_composition_control_utf8(uint64_t handle, const uint8_t *control, size_t len, uint8_t *out, size_t cap);
size_t inputkey_raw_boundary(uint64_t handle, uint8_t *out, size_t cap);
size_t inputkey_lifecycle_utf8(uint64_t handle, const uint8_t *lifecycle, size_t len, uint8_t *out, size_t cap);
size_t inputkey_rendered(uint64_t handle, uint8_t *out, size_t cap);
size_t inputkey_raw(uint64_t handle, uint8_t *out, size_t cap);
int inputkey_has_history(uint64_t handle);
size_t inputkey_language_count(void);
size_t inputkey_language_id(size_t index, uint8_t *out, size_t cap);
size_t inputkey_language_name(size_t index, uint8_t *out, size_t cap);
size_t inputkey_language_default_method(size_t index, uint8_t *out, size_t cap);
size_t inputkey_method_count(const char *language);
size_t inputkey_method_id(const char *language, size_t index, uint8_t *out, size_t cap);
size_t inputkey_method_label(const char *language, size_t index, uint8_t *out, size_t cap);
size_t inputkey_option_count(const char *language);
size_t inputkey_option_id(const char *language, size_t index, uint8_t *out, size_t cap);
size_t inputkey_option_label(const char *language, size_t index, uint8_t *out, size_t cap);
int inputkey_option_default_enabled(const char *language, size_t index);

#ifdef __cplusplus
}
#endif
