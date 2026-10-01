/* SPDX-License-Identifier: GPL-2.0-or-later */
const DEFAULTS = { enabled:true, method:'telex', simpleTelex:false, autoRestore:true, showToast:true, literalizeShortcut:{ ...InputKeyLiteralizeShortcut.DEFAULT_LITERALIZE_SHORTCUT } };
const $ = id => document.getElementById(id);
$('version').textContent = 'v' + chrome.runtime.getManifest().version;
let currentSettings = { ...DEFAULTS };
let recordingShortcut = false;

function paint(s) {
  currentSettings = { ...s, literalizeShortcut: InputKeyLiteralizeShortcut.normalizeShortcut(s.literalizeShortcut) };
  $('toggle').textContent = s.enabled ? 'V' : 'E';
  $('toggle').classList.toggle('off', !s.enabled);
  $('method').value = s.method;
  $('simpleTelex').checked = s.simpleTelex;
  $('simpleTelex').disabled = s.method !== 'telex';
  $('autoRestore').checked = s.autoRestore;
  $('showToast').checked = s.showToast;
  $('shortcutRecord').textContent = recordingShortcut ? 'Nhấn tổ hợp phím…' : InputKeyLiteralizeShortcut.shortcutLabel(currentSettings.literalizeShortcut);
  $('shortcutEnabled').checked = currentSettings.literalizeShortcut.enabled;
}

async function load() {
  const s = await chrome.storage.local.get(DEFAULTS);
  paint(s);
}

$('toggle').addEventListener('click', async () => {
  const { enabled } = await chrome.storage.local.get(DEFAULTS);
  await chrome.storage.local.set({ enabled: !enabled });
  load();
});
$('method').addEventListener('change', e => chrome.storage.local.set({ method: e.target.value }).then(load));
$('simpleTelex').addEventListener('change', e => chrome.storage.local.set({ simpleTelex: e.target.checked }));
$('autoRestore').addEventListener('change', e => chrome.storage.local.set({ autoRestore: e.target.checked }));
$('showToast').addEventListener('change', e => chrome.storage.local.set({ showToast: e.target.checked }));
$('shortcutRecord').addEventListener('click', () => {
  recordingShortcut = true;
  $('shortcutStatus').textContent = 'Nhấn tổ hợp phím cần dùng; Esc để hủy.';
  $('shortcutRecord').textContent = 'Nhấn tổ hợp phím…';
});
$('shortcutEnabled').addEventListener('change', e => {
  const shortcut = { ...currentSettings.literalizeShortcut, enabled: e.target.checked };
  chrome.storage.local.set({ literalizeShortcut: shortcut });
});
$('shortcutReset').addEventListener('click', () => {
  recordingShortcut = false;
  chrome.storage.local.set({ literalizeShortcut: { ...InputKeyLiteralizeShortcut.DEFAULT_LITERALIZE_SHORTCUT } });
  $('shortcutStatus').textContent = 'Đã đặt lại Ctrl + Space.';
});
document.addEventListener('keydown', async e => {
  if (!recordingShortcut) return;
  e.preventDefault();
  e.stopPropagation();
  if (['Control', 'Alt', 'Shift', 'Meta'].includes(e.key)) return;
  if (e.key === 'Escape') {
    recordingShortcut = false;
    $('shortcutStatus').textContent = 'Đã hủy ghi phím tắt.';
    $('shortcutRecord').textContent = InputKeyLiteralizeShortcut.shortcutLabel(currentSettings.literalizeShortcut);
    return;
  }
  const result = InputKeyLiteralizeShortcut.validateRecordedShortcut({ enabled: true, code: e.code, ctrl: e.ctrlKey, shift: e.shiftKey, alt: e.altKey, meta: e.metaKey });
  if (!result.valid) {
    $('shortcutStatus').textContent = result.error;
    return;
  }
  recordingShortcut = false;
  $('shortcutStatus').textContent = 'Đã lưu phím tắt.';
  await chrome.storage.local.set({ literalizeShortcut: result.shortcut });
});
chrome.storage.onChanged.addListener(load);
load();


(async () => {
  await InputKey.ready;
// Small self-contained tester inside the popup (content scripts do not run on chrome-extension:// pages).
const tryBox = $('try');
const tester = new InputKey.Engine({ method:'telex', simpleTelex:false });
let testerRendered = '';

function testerSettings() {
  return {
    method: $('method').value,
    simpleTelex: $('simpleTelex').checked,
    autoRestore: $('autoRestore').checked
  };
}

tryBox.addEventListener('keydown', e => {
  const shortcut = currentSettings.literalizeShortcut;
  if (InputKeyLiteralizeShortcut.matchesShortcut(e, shortcut) && tester.raw) {
    e.preventDefault();
    const old = testerRendered;
    const next = tester.literalizeToken();
    const end = tryBox.selectionStart;
    tryBox.setRangeText(next, Math.max(0, end - old.length), tryBox.selectionEnd, 'end');
    testerRendered = next;
    return;
  }
  if (e.ctrlKey || e.metaKey || e.altKey || e.isComposing) return;
  tester.configure(testerSettings());
  if (e.key === 'Backspace' && tester.raw) {
    e.preventDefault();
    const old = testerRendered;
    const next = tester.backspace();
    const end = tryBox.selectionStart;
    tryBox.setRangeText(next, Math.max(0, end - old.length), tryBox.selectionEnd, 'end');
    testerRendered = next;
    return;
  }
  if (e.key === 'Escape' && tester.raw) {
    e.preventDefault();
    const old = testerRendered;
    const next = tester.escape();
    const end = tryBox.selectionStart;
    tryBox.setRangeText(next, Math.max(0, end - old.length), tryBox.selectionEnd, 'end');
    testerRendered = next;
    return;
  }
  if (e.key.length !== 1) {
    tester.reset(); testerRendered = ''; return;
  }
  if (/\s|[.,;:!?(){}<>"'`~@#$%^&*+=\\/|_-]/.test(e.key)) {
    if (tester.raw) {
      const old = testerRendered;
      const finalText = tester.finalize();
      if (finalText !== old) {
        e.preventDefault();
        const end = tryBox.selectionStart;
        tryBox.setRangeText(finalText + e.key, Math.max(0, end - old.length), tryBox.selectionEnd, 'end');
      }
    }
    tester.reset(); testerRendered = ''; return;
  }
  const valid = $('method').value === 'vni' ? /^[A-Za-z0-9]$/.test(e.key) : /^[A-Za-z\[\]]$/.test(e.key);
  if (!valid) { tester.reset(); testerRendered = ''; return; }
  e.preventDefault();
  const old = testerRendered;
  const next = tester.type(e.key);
  const end = tryBox.selectionStart;
  tryBox.setRangeText(next, Math.max(0, end - old.length), tryBox.selectionEnd, 'end');
  testerRendered = next;
});
tryBox.addEventListener('click', () => { tester.reset(); testerRendered = ''; });

})();
