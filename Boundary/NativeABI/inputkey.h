#pragma once
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

#define INPUTKEY_ABI_VERSION 2

uint64_t inputkey_create(const char *method, int simple_telex, int auto_restore);
void inputkey_destroy(uint64_t handle);

/* Return the full UTF-8 byte length. If out/cap are supplied, write a NUL-terminated
   prefix that fits in cap bytes. Call again with a larger buffer if return >= cap. */
size_t inputkey_key_utf8(uint64_t handle, const uint8_t *bytes, size_t len, uint8_t *out, size_t cap);
size_t inputkey_backspace(uint64_t handle, uint8_t *out, size_t cap);
size_t inputkey_escape(uint64_t handle, uint8_t *out, size_t cap);
size_t inputkey_finalize(uint64_t handle, uint8_t *out, size_t cap);
size_t inputkey_literalize_token(uint64_t handle, uint8_t *out, size_t cap);
size_t inputkey_rendered(uint64_t handle, uint8_t *out, size_t cap);
size_t inputkey_raw(uint64_t handle, uint8_t *out, size_t cap);
void inputkey_reset(uint64_t handle);
int inputkey_has_history(uint64_t handle);

#ifdef __cplusplus
}
#endif
