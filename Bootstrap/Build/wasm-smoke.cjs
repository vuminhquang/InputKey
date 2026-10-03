const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const wasmPath = process.argv[2] || path.join(__dirname, '../../../dist/inputkey.wasm');

(async () => {
  const { instance } = await WebAssembly.instantiate(fs.readFileSync(wasmPath), {});
  const w = instance.exports;
  const decoder = new TextDecoder();
  const encoder = new TextEncoder();
  function out() {
    return decoder.decode(new Uint8Array(w.memory.buffer, w.inputkey_output_ptr(), w.inputkey_output_len()));
  }
  function utf8Call(name, h, text) {
    const bytes = encoder.encode(text);
    const p = w.inputkey_alloc(bytes.length);
    new Uint8Array(w.memory.buffer, p, bytes.length).set(bytes);
    w[name](h, p, bytes.length);
    w.inputkey_free(p, bytes.length);
    return out();
  }
  function type(h, text) {
    let value = '';
    for (const ch of text) value = utf8Call('inputkey_character_utf8', h, ch);
    return value;
  }

  const h = w.inputkey_create(0, 0, 1);
  assert.equal(type(h, 'refer'), 'rể');
  w.inputkey_raw_boundary(h);
  assert.equal(out(), 'refer');
  w.inputkey_destroy(h);

  const h2 = w.inputkey_create(0, 0, 1);
  assert.equal(type(h2, 'urrl'), 'url');
  w.inputkey_destroy(h2);

  console.log('WASM smoke passed');
})().catch(e => { console.error(e); process.exitCode = 1; });
