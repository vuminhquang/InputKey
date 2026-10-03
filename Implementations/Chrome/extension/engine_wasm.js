/* Adapter for InputKey's raw Rust WASM ABI. */
(() => {
  const DEFAULTS = {
    language: 'vi',
    method: 'telex',
    simpleTelex: false,
    autoRestore: true,
    smartCorrection: true
  };
  const decoder = new TextDecoder();
  const encoder = new TextEncoder();
  let wasm;

  const ready = (async () => {
    const url = chrome.runtime.getURL('inputkey.wasm');
    let result;
    try {
      result = await WebAssembly.instantiateStreaming(fetch(url), {});
    } catch (_) {
      result = await WebAssembly.instantiate(await (await fetch(url)).arrayBuffer(), {});
    }
    wasm = result.instance.exports;
  })();

  const norm = (opts = {}) => ({
    language: opts.language ?? DEFAULTS.language,
    method: opts.method ?? DEFAULTS.method,
    simpleTelex: opts.simpleTelex ?? DEFAULTS.simpleTelex,
    autoRestore: opts.autoRestore ?? DEFAULTS.autoRestore,
    smartCorrection: opts.smartCorrection ?? DEFAULTS.smartCorrection
  });

  function output() {
    const n = Number(wasm.inputkey_output_len());
    return decoder.decode(
      new Uint8Array(wasm.memory.buffer, Number(wasm.inputkey_output_ptr()), n)
    );
  }

  function withUtf8(value, fn) {
    const bytes = encoder.encode(value ?? '');
    if (!bytes.length) return fn(0, 0);
    const p = Number(wasm.inputkey_alloc(bytes.length));
    try {
      new Uint8Array(wasm.memory.buffer, p, bytes.length).set(bytes);
      return fn(p, bytes.length);
    } finally {
      wasm.inputkey_free(p, bytes.length);
    }
  }

  function createEx(opts) {
    const options = JSON.stringify({
      simple_telex: !!opts.simpleTelex,
      auto_restore: !!opts.autoRestore,
      smart_correction: !!opts.smartCorrection
    });
    return withUtf8(opts.language, (lp, ln) =>
      withUtf8(opts.method, (mp, mn) =>
        withUtf8(options, (op, on) =>
          Number(wasm.inputkey_create_ex(lp, ln, mp, mn, op, on))
        )
      )
    );
  }

  function keyCall(name, id, key) {
    return withUtf8(key, (p, n) => {
      wasm[name](id, p, n);
      return output();
    });
  }

  function catalog() {
    wasm.inputkey_catalog_json();
    return JSON.parse(output());
  }

  class InputKeyEngine {
    constructor(opts = {}) {
      this._opts = norm(opts);
      this._id = 0;
    }

    _ensure() {
      if (!this._id) this._id = createEx(this._opts);
      if (!this._id) throw new Error('InputKey could not create the selected language engine');
    }

    configure(opts = {}) {
      const next = norm(opts);
      if (JSON.stringify(next) !== JSON.stringify(this._opts)) {
        if (this._id) wasm.inputkey_destroy(this._id);
        this._id = 0;
        this._opts = next;
      }
    }

    acceptsCharacter(character) {
      this._ensure();
      return withUtf8(character, (p, n) => !!wasm.inputkey_accepts_character_utf8(this._id, p, n));
    }

    get raw() {
      this._ensure();
      wasm.inputkey_raw(this._id);
      return output();
    }

    get rendered() {
      this._ensure();
      wasm.inputkey_rendered(this._id);
      return output();
    }

    character(character) {
      this._ensure();
      return keyCall('inputkey_character_utf8', this._id, character);
    }

    spaceBoundary() {
      this._ensure();
      wasm.inputkey_space_boundary(this._id);
      return output();
    }

    punctuationBoundary(delimiter) {
      this._ensure();
      return keyCall('inputkey_punctuation_boundary_utf8', this._id, delimiter);
    }

    caretMoveBoundary(cause) {
      this._ensure();
      return keyCall('inputkey_caret_move_boundary_utf8', this._id, cause);
    }

    shortcutBoundary() {
      this._ensure();
      wasm.inputkey_shortcut_boundary(this._id);
      return output();
    }

    compositionControl(control) {
      this._ensure();
      return keyCall('inputkey_composition_control_utf8', this._id, control);
    }

    rawBoundary() {
      this._ensure();
      wasm.inputkey_raw_boundary(this._id);
      return output();
    }

    lifecycle(event) {
      if (!this._id) return '';
      return keyCall('inputkey_lifecycle_utf8', this._id, event);
    }

    destroy() {
      if (this._id) wasm.inputkey_destroy(this._id);
      this._id = 0;
    }
  }

  globalThis.InputKey = { ready, catalog, Engine: InputKeyEngine };
})();
