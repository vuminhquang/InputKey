from __future__ import annotations
import ctypes
import pathlib
import sys

root = pathlib.Path(__file__).resolve().parents[2]
if len(sys.argv) > 1:
    lib_path = pathlib.Path(sys.argv[1]).resolve()
else:
    name = "inputkey_cabi.dll" if sys.platform == "win32" else ("libinputkey_cabi.dylib" if sys.platform == "darwin" else "libinputkey_cabi.so")
    lib_path = root / "target" / "release" / name
if not lib_path.exists():
    raise SystemExit(f"missing {lib_path}")

lib = ctypes.CDLL(str(lib_path))
U8P = ctypes.POINTER(ctypes.c_uint8)

lib.inputkey_create.argtypes = [ctypes.c_char_p, ctypes.c_int32, ctypes.c_int32]
lib.inputkey_create.restype = ctypes.c_uint64
lib.inputkey_create_ex.argtypes = [ctypes.c_char_p, ctypes.c_char_p, ctypes.c_char_p]
lib.inputkey_create_ex.restype = ctypes.c_uint64
lib.inputkey_destroy.argtypes = [ctypes.c_uint64]
lib.inputkey_catalog_json.argtypes = [U8P, ctypes.c_size_t]
lib.inputkey_catalog_json.restype = ctypes.c_size_t

for name in ("inputkey_space_boundary", "inputkey_shortcut_boundary", "inputkey_raw_boundary", "inputkey_rendered", "inputkey_raw"):
    fn = getattr(lib, name)
    fn.argtypes = [ctypes.c_uint64, U8P, ctypes.c_size_t]
    fn.restype = ctypes.c_size_t

for name in ("inputkey_character_utf8", "inputkey_punctuation_boundary_utf8", "inputkey_caret_move_boundary_utf8", "inputkey_composition_control_utf8", "inputkey_lifecycle_utf8"):
    fn = getattr(lib, name)
    fn.argtypes = [ctypes.c_uint64, U8P, ctypes.c_size_t, U8P, ctypes.c_size_t]
    fn.restype = ctypes.c_size_t

lib.inputkey_accepts_character_utf8.argtypes = [ctypes.c_uint64, U8P, ctypes.c_size_t]
lib.inputkey_accepts_character_utf8.restype = ctypes.c_int32

def bytes_arg(text: str):
    raw = text.encode("utf-8")
    return raw, (ctypes.c_uint8 * len(raw)).from_buffer_copy(raw)

def call0(fn, handle: int) -> str:
    buf = (ctypes.c_uint8 * 4096)()
    n = fn(handle, buf, len(buf))
    return bytes(buf[:n]).decode("utf-8")

def call1(fn, handle: int, value: str) -> str:
    raw, inp = bytes_arg(value)
    buf = (ctypes.c_uint8 * 4096)()
    n = fn(handle, inp, len(raw), buf, len(buf))
    return bytes(buf[:n]).decode("utf-8")

def type_text(handle: int, text: str) -> str:
    value = ""
    for ch in text:
        value = call1(lib.inputkey_character_utf8, handle, ch)
    return value

h = lib.inputkey_create(b"telex", 0, 1)
try:
    assert type_text(h, "refer") == "rể"
    assert call0(lib.inputkey_raw_boundary, h) == "refer"
    assert type_text(h, "s") == "s"
finally:
    lib.inputkey_destroy(h)

h = lib.inputkey_create(b"telex", 0, 1)
try:
    assert type_text(h, "urrl") == "url"
finally:
    lib.inputkey_destroy(h)

h = lib.inputkey_create(b"telex", 0, 1)
try:
    assert type_text(h, "dd") == "đ"
    assert call1(lib.inputkey_caret_move_boundary_utf8, h, "left") == "đ"
    assert type_text(h, "a") == "a"
finally:
    lib.inputkey_destroy(h)

h = lib.inputkey_create(b"telex", 0, 1)
try:
    assert type_text(h, "dd") == "đ"
    assert call0(lib.inputkey_shortcut_boundary, h) == "đ"
    assert type_text(h, "a") == "a"
finally:
    lib.inputkey_destroy(h)

buf = (ctypes.c_uint8 * 4096)()
n = lib.inputkey_catalog_json(buf, len(buf))
catalog = bytes(buf[:n]).decode("utf-8")
for language_id in ("vi", "fr", "da", "sv", "de"):
    assert f'"id":"{language_id}"' in catalog

h = lib.inputkey_create_ex(b"fr", b"telex", b"{}")
try:
    assert type_text(h, "ee") == "ê"
    call1(lib.inputkey_lifecycle_utf8, h, "reset")
    assert type_text(h, "es") == "é"
    call1(lib.inputkey_lifecycle_utf8, h, "reset")
    assert type_text(h, "oe") == "œ"
    call1(lib.inputkey_lifecycle_utf8, h, "reset")
    assert type_text(h, "cc") == "ç"
finally:
    lib.inputkey_destroy(h)

for language, raw, expected in [
    (b"da", "Koebenhavn", "København"),
    (b"sv", "saw", "så"),
    (b"de", "strasze", "straße"),
]:
    h = lib.inputkey_create_ex(language, b"telex", b"{}")
    try:
        assert type_text(h, raw) == expected
    finally:
        lib.inputkey_destroy(h)

print("cabi-smoke=ok")
