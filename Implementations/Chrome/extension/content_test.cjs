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
  activeElement: null,
  addEventListener(type, fn) { handlers.set(type, fn); }
};
globalThis.chrome = {
  runtime: { getURL() { return wasmPath; } },
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
require('./shortcut.js');

before(async () => {
  await InputKey.ready;
  require('./content.js');
  await new Promise(resolve => setImmediate(resolve));
});

function send(input, key, extras = {}) {
  document.activeElement = input;
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
  if (!event.prevented && key.length === 1 && !event.ctrlKey && !event.altKey && !event.metaKey) {
    input.setRangeText(key, input.selectionStart, input.selectionEnd);
  }
  return event;
}

function type(input, text) {
  for (const key of text) send(input, key);
}

test('Shift+Space commits only the physical keys', () => {
  const input = new Input();
  type(input, 'refer');
  assert.equal(input.value, 'rể');

  const event = send(input, ' ', { shiftKey: true, code: 'Space' });
  assert.equal(event.prevented, true);
  assert.equal(event.stopped, true);
  assert.equal(input.value, 'refer');
});

test('normal Space keeps word-finalization behavior', () => {
  const input = new Input();
  type(input, 'data');
  assert.equal(input.value, 'dât');
  send(input, ' ');
  assert.equal(input.value, 'data ');
});

test('Space decides whether an unfinished Vietnamese shape survives', () => {
  const input = new Input();
  type(input, 'thaas');
  assert.equal(input.value, 'thấ');
  send(input, ' ');
  assert.equal(input.value, 'thaas ');
});

test('boundary correction preserves the physical initial prefix', () => {
  const input = new Input();
  type(input, 'stop');
  send(input, ' ');
  assert.equal(input.value, 'stop ');
});

test('brackets stay punctuation instead of Vietnamese shape shortcuts', () => {
  const input = new Input();
  type(input, 'u');
  send(input, '[');
  assert.equal(input.value, 'u[');
  send(input, ']');
  assert.equal(input.value, 'u[]');
});

test('punctuation boundary shares Vietnamese correction policy', () => {
  const input = new Input();
  type(input, 'dduwocj');
  send(input, '.');
  assert.equal(input.value, 'được.');
});

test('shortcut boundary commits displayed text and passes the chord through', () => {
  const input = new Input();
  type(input, 'dd');
  assert.equal(input.value, 'đ');

  const shortcut = send(input, 'a', { ctrlKey: true, code: 'KeyA' });
  assert.equal(shortcut.prevented, false);
  assert.equal(input.value, 'đ');

  send(input, 'a');
  assert.equal(input.value, 'đa');
});

test('caret move boundary leaves the currently rendered text alone', () => {
  const input = new Input();
  type(input, 'dd');
  assert.equal(input.value, 'đ');
  const event = send(input, 'ArrowLeft', { code: 'ArrowLeft' });
  assert.equal(event.prevented, false);
  assert.equal(input.value, 'đ');
});

test('mouse caret move ends the active Root composition before relocation', () => {
  const input = new Input();
  document.activeElement = input;
  type(input, 'dd');
  const committed = input.value;

  handlers.get('mousedown')({ target: input, composedPath() { return [input]; } });
  input.selectionStart = 0;
  input.selectionEnd = 0;
  type(input, 'a');

  assert.equal(input.value, 'a' + committed);
});

test('selecting existing text and typing replaces the selection', () => {
  const input = new Input();
  type(input, 'rooif');
  assert.equal(input.value, 'rồi');

  handlers.get('mousedown')({ target: input, composedPath() { return [input]; } });
  input.selectionStart = 0;
  input.selectionEnd = input.value.length;

  type(input, 'abc');
  assert.equal(input.value, 'abc');
});

test('French Telex uses the same Root engine contract', () => {
  const engine = new InputKey.Engine({ language: 'fr', method: 'telex' });
  for (const key of 'oe') engine.character(key);
  assert.equal(engine.rendered, 'œ');
  engine.lifecycle('reset');
  for (const key of 'cc') engine.character(key);
  assert.equal(engine.rendered, 'ç');
  engine.lifecycle('reset');
  for (const key of 'ee') engine.character(key);
  assert.equal(engine.rendered, 'ê');
  engine.lifecycle('reset');
  for (const key of 'es') engine.character(key);
  assert.equal(engine.rendered, 'é');
  assert.equal(engine.spaceBoundary(), 'é ');
  engine.destroy();
});

test('toggle shortcut parser supports modifier-only and keyed chords', () => {
  assert.equal(InputKeyShortcut.normalize('control + shift'), 'Ctrl+Shift');
  assert.equal(InputKeyShortcut.normalize('Alt+z'), 'Alt+Z');
  assert.equal(InputKeyShortcut.normalize('Ctrl+Shift+k'), 'Ctrl+Shift+K');
  assert.equal(InputKeyShortcut.normalize('off'), 'Off');
  assert.equal(InputKeyShortcut.normalize('Ctrl+Alt+K'), null);
  assert.equal(InputKeyShortcut.normalize('Ctrl'), null);
});

test('Rust WASM is self-contained and exposes the language-neutral ABI', async () => {
  const module = await WebAssembly.compile(readFileSync(wasmPath));
  assert.deepEqual(WebAssembly.Module.imports(module), []);
  const names = new Set(WebAssembly.Module.exports(module).map(item => item.name));
  for (const name of [
    'inputkey_character_utf8',
    'inputkey_space_boundary',
    'inputkey_punctuation_boundary_utf8',
    'inputkey_caret_move_boundary_utf8',
    'inputkey_shortcut_boundary',
    'inputkey_composition_control_utf8',
    'inputkey_raw_boundary',
    'inputkey_lifecycle_utf8',
    'inputkey_accepts_character_utf8',
    'inputkey_create_ex',
    'inputkey_catalog_json'
  ]) assert.ok(names.has(name), name);
  assert.ok(![...names].some(name => name.startsWith('vk_')));
});

test('manifest uses only the Rust bridge and InputKey WASM', () => {
  const manifest = JSON.parse(readFileSync(path.join(__dirname, 'manifest.json'), 'utf8'));
  assert.equal(manifest.version, '5.2.6');
  assert.deepEqual(manifest.content_scripts[0].js, ['engine_wasm.js', 'shortcut.js', 'content.js']);
  assert.deepEqual(manifest.web_accessible_resources[0].resources, ['inputkey.wasm']);
});
