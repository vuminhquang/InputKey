/* Small adapter for InputKey's raw Rust WASM ABI. */
(() => {
  const DEFAULTS = { method: 'telex', simpleTelex: false, autoRestore: true };
  const decoder = new TextDecoder(); const encoder = new TextEncoder(); let wasm;
  const ready = (async () => {
    const url = chrome.runtime.getURL('inputkey.wasm');
    let result; try { result = await WebAssembly.instantiateStreaming(fetch(url), {}); }
    catch (_) { result = await WebAssembly.instantiate(await (await fetch(url)).arrayBuffer(), {}); }
    wasm = result.instance.exports;
  })();
  const norm = (opts = {}) => ({
    method: opts.method ?? DEFAULTS.method,
    simpleTelex: opts.simpleTelex ?? DEFAULTS.simpleTelex,
    autoRestore: opts.autoRestore ?? DEFAULTS.autoRestore
  });
  function output() { const n = Number(wasm.inputkey_output_len()); return decoder.decode(new Uint8Array(wasm.memory.buffer, Number(wasm.inputkey_output_ptr()), n)); }
  class InputKeyEngine {
    constructor(opts = {}) { this._opts = norm(opts); this._id = 0; }
    _ensure() { if (!this._id) this._id = wasm.inputkey_create(this._opts.method === 'vni' ? 1 : 0, this._opts.simpleTelex ? 1 : 0, this._opts.autoRestore ? 1 : 0); }
    configure(opts = {}) { const next = norm(opts); if (JSON.stringify(next) !== JSON.stringify(this._opts)) { if (this._id) wasm.inputkey_destroy(this._id); this._id = 0; this._opts = next; } }
    get raw() { this._ensure(); wasm.inputkey_raw(this._id); return output(); }
    get rendered() { this._ensure(); wasm.inputkey_rendered(this._id); return output(); }
    type(key) { this._ensure(); const bytes = encoder.encode(key); const p = wasm.inputkey_alloc(bytes.length); new Uint8Array(wasm.memory.buffer, p, bytes.length).set(bytes); wasm.inputkey_key_utf8(this._id, p, bytes.length); wasm.inputkey_free(p, bytes.length); return output(); }
    backspace() { this._ensure(); wasm.inputkey_backspace(this._id); return output(); }
    escape() { this._ensure(); wasm.inputkey_escape(this._id); return output(); }
    finalize() { this._ensure(); wasm.inputkey_finalize(this._id); return output(); }
    literalizeToken() { this._ensure(); wasm.inputkey_literalize_token(this._id); return output(); }
    reset() { if (this._id) wasm.inputkey_reset(this._id); }
  }
  globalThis.InputKey = { ready, Engine: InputKeyEngine };
})();
