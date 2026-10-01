const assert = require('node:assert/strict');
const { readFileSync } = require('node:fs');
const path = require('node:path');
const { test, before } = require('node:test');

const wasmPath = process.env.INPUTKEY_WASM || path.join(__dirname, 'inputkey.wasm');
const handlers = new Map();
let savedSettings = {};
const storageChangedListeners = [];

class Input {
  constructor() {
    this.nodeType = 1;
    this.type = 'text';
    this.value = '';
    this.selectionStart = 0;
    this.selectionEnd = 0;
    this.readOnly = false;
    this.disabled = false;
  }
  setRangeText(text, start, end) {
    this.value = this.value.slice(0, start) + text + this.value.slice(end);
    this.selectionStart = this.selectionEnd = start + text.length;
  }
  dispatchEvent() {}
}

globalThis.HTMLInputElement = Input;
globalThis.HTMLTextAreaElement = class extends Input {};
globalThis.Node = { ELEMENT_NODE: 1, TEXT_NODE: 3 };
globalThis.NodeFilter = { SHOW_TEXT: 4 };
globalThis.document = {
  documentElement: null,
  addEventListener(type, fn) { handlers.set(type, fn); }
};
globalThis.chrome = {
  runtime: {
    getURL() { return wasmPath; }
  },
  storage: {
    local: {
      get(defaults, callback) { callback({ ...defaults, ...savedSettings, showToast: false }); },
      set(values) { savedSettings = { ...savedSettings, ...values }; }
    },
    onChanged: { addListener(fn) { storageChangedListeners.push(fn); } }
  }
};
globalThis.fetch = async file => new Response(readFileSync(file), {
  headers: { 'Content-Type': 'application/wasm' }
});

require('./engine_wasm.js');
require('./literalize_shortcut.js');

before(async () => {
  await InputKey.ready;
  require('./content.js');
  await new Promise(resolve => setImmediate(resolve));
});

function send(input, key, extras = {}) {
  const event = {
    target: input,
    key,
    code: extras.code || (key === ' ' ? 'Space' : ''),
    ctrlKey: Boolean(extras.ctrlKey),
    altKey: Boolean(extras.altKey),
    metaKey: Boolean(extras.metaKey),
    shiftKey: Boolean(extras.shiftKey),
    isComposing: false,
    keyCode: 0,
    prevented: false,
    stopped: false,
    composedPath() { return [input]; },
    preventDefault() { this.prevented = true; },
    stopImmediatePropagation() { this.stopped = true; }
  };
  handlers.get('keydown')(event);
  if (!event.prevented && key.length === 1) {
    input.setRangeText(key, input.selectionStart, input.selectionEnd);
  }
  return event;
}

function type(input, text) {
  for (const key of text) send(input, key);
}

test('Ctrl+Space literalizes the whole active token without inserting space', () => {
  const input = new Input();
  type(input, 'refer');
  assert.equal(input.value, 'rể');

  const event = send(input, ' ', { ctrlKey: true, code: 'Space' });
  assert.equal(event.prevented, true);
  assert.equal(event.stopped, true);
  assert.equal(input.value, 'refer');

  send(input, 's');
  assert.equal(input.value, 'refers');
});

test('literalize shortcut helpers normalize and match exact physical chords', () => {
  const shortcuts = InputKeyLiteralizeShortcut;
  const base = shortcuts.normalizeShortcut(undefined);
  assert.deepEqual(base, { enabled: true, code: 'Space', ctrl: true, shift: false, alt: false, meta: false });
  const event = { code: 'Space', ctrlKey: true, shiftKey: false, altKey: false, metaKey: false };
  assert.equal(shortcuts.matchesShortcut(event, base), true);
  assert.equal(shortcuts.matchesShortcut({ ...event, shiftKey: true }, base), false);
  assert.equal(shortcuts.matchesShortcut(event, { ...base, enabled: false }), false);
  assert.equal(shortcuts.matchesShortcut({ ...event, code: 'Semicolon', ctrlKey: false, altKey: true }, { code: 'Semicolon', ctrl: false, alt: true }), true);
  assert.equal(shortcuts.validateRecordedShortcut({ code: 'KeyA' }).valid, false);
  assert.equal(shortcuts.shortcutLabel(base), 'Ctrl + Space');
});

test('configured shortcut literalizes current token and disabled shortcut does not capture', () => {
  const input = new Input();
  type(input, 'refer');
  const enabledShortcut = { enabled: true, code: 'Semicolon', ctrl: false, shift: false, alt: true, meta: false };
  savedSettings.literalizeShortcut = enabledShortcut;
  for (const listener of storageChangedListeners) listener({ literalizeShortcut: { newValue: enabledShortcut } }, 'local');

  assert.equal(input.value, 'rể');
  const event = send(input, ';', { code: 'Semicolon', altKey: true });
  assert.equal(event.prevented, true);
  assert.equal(event.stopped, true);
  assert.equal(input.value, 'refer');

  const disabledShortcut = { ...enabledShortcut, enabled: false };
  savedSettings.literalizeShortcut = disabledShortcut;
  for (const listener of storageChangedListeners) listener({ literalizeShortcut: { newValue: disabledShortcut } }, 'local');
  const disabledEvent = send(input, ';', { code: 'Semicolon', altKey: true });
  assert.equal(disabledEvent.prevented, false);
});

test('Rust WASM is self-contained and exposes only the InputKey ABI', async () => {
  const module = await WebAssembly.compile(readFileSync(wasmPath));
  assert.deepEqual(WebAssembly.Module.imports(module), []);
  const names = new Set(WebAssembly.Module.exports(module).map(item => item.name));
  assert.ok(names.has('inputkey_literalize_token'));
  assert.ok(![...names].some(name => name.startsWith('vk_')));
});

test('manifest uses the Rust bridge and InputKey WASM', () => {
  const manifest = JSON.parse(readFileSync(path.join(__dirname, 'manifest.json'), 'utf8'));
  assert.equal(manifest.version, '5.0.0');
  assert.deepEqual(manifest.content_scripts[0].js, ['engine_wasm.js', 'literalize_shortcut.js', 'content.js']);
  assert.deepEqual(manifest.web_accessible_resources[0].resources, ['inputkey.wasm']);
});
