from __future__ import annotations
import ctypes
import pathlib
import sys

root = pathlib.Path(__file__).resolve().parents[2]
lib_path = pathlib.Path(sys.argv[1]).resolve() if len(sys.argv) > 1 else root / "target" / "release" / "inputkey_cabi.dll"
if not lib_path.exists():
    raise SystemExit(f"missing {lib_path}")
lib = ctypes.CDLL(str(lib_path))

lib.inputkey_create.argtypes = [ctypes.c_char_p, ctypes.c_int32, ctypes.c_int32]
lib.inputkey_create.restype = ctypes.c_uint64
lib.inputkey_create_ex.argtypes = [ctypes.c_char_p, ctypes.c_char_p, ctypes.c_char_p]
lib.inputkey_create_ex.restype = ctypes.c_uint64
lib.inputkey_destroy.argtypes = [ctypes.c_uint64]
lib.inputkey_catalog_json.argtypes = [ctypes.POINTER(ctypes.c_uint8), ctypes.c_size_t]
lib.inputkey_catalog_json.restype = ctypes.c_size_t

for name in (
    "inputkey_backspace", "inputkey_escape", "inputkey_finalize",
    "inputkey_commit_raw_boundary", "inputkey_natural_boundary", "inputkey_mouse_boundary",
    "inputkey_rendered", "inputkey_raw",
):
    fn = getattr(lib, name)
    fn.argtypes = [ctypes.c_uint64, ctypes.POINTER(ctypes.c_uint8), ctypes.c_size_t]
    fn.restype = ctypes.c_size_t

lib.inputkey_key_utf8.argtypes = [
    ctypes.c_uint64, ctypes.POINTER(ctypes.c_uint8), ctypes.c_size_t,
    ctypes.POINTER(ctypes.c_uint8), ctypes.c_size_t,
]
lib.inputkey_key_utf8.restype = ctypes.c_size_t

def call_text(fn, handle: int) -> str:
    buf = (ctypes.c_uint8 * 4096)()
    n = fn(handle, buf, len(buf))
    if n >= len(buf):
        raise AssertionError(f"output too long: {n}")
    return bytes(buf[:n]).decode("utf-8")

def type_text(handle: int, text: str) -> str:
    out = ""
    for ch in text:
        raw = ch.encode("utf-8")
        inp = (ctypes.c_uint8 * len(raw)).from_buffer_copy(raw)
        buf = (ctypes.c_uint8 * 4096)()
        n = lib.inputkey_key_utf8(handle, inp, len(raw), buf, len(buf))
        out = bytes(buf[:n]).decode("utf-8")
    return out

h = lib.inputkey_create(b"telex", 0, 1)
try:
    assert type_text(h, "refer") == "rể"
    assert call_text(lib.inputkey_commit_raw_boundary, h) == "refer"
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
    assert call_text(lib.inputkey_natural_boundary, h) == "đ"
    assert type_text(h, "a") == "a"
finally:
    lib.inputkey_destroy(h)

h = lib.inputkey_create(b"telex", 0, 1)
try:
    assert type_text(h, "dd") == "đ"
    assert call_text(lib.inputkey_mouse_boundary, h) == "đ"
    assert type_text(h, "a") == "a"
finally:
    lib.inputkey_destroy(h)

buf = (ctypes.c_uint8 * 4096)()
n = lib.inputkey_catalog_json(buf, len(buf))
catalog = bytes(buf[:n]).decode("utf-8")
assert '"id":"vi"' in catalog and '"id":"fr"' in catalog

h = lib.inputkey_create_ex(b"fr", b"telex", b"{}")
try:
    assert type_text(h, "ee") == "ê"
    lib.inputkey_reset(h)
    assert type_text(h, "es") == "é"
    lib.inputkey_reset(h)
    assert type_text(h, "oe") == "œ"
    lib.inputkey_reset(h)
    assert type_text(h, "cc") == "ç"
finally:
    lib.inputkey_destroy(h)

print("cabi-smoke=ok")
