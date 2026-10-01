/* SPDX-License-Identifier: GPL-2.0-or-later */
(async () => {
  await InputKey.ready;
  'use strict';

  const DEFAULTS = {
    enabled: true,
    method: 'telex',
    simpleTelex: false,
    autoRestore: true,
    showToast: true,
    literalizeShortcut: { ...InputKeyLiteralizeShortcut.DEFAULT_LITERALIZE_SHORTCUT }
  };

  let settings = { ...DEFAULTS };
  const states = new WeakMap();
  let internalEdit = false;
  let chord = { ctrl: false, shift: false, armed: false, used: false };

  chrome.storage.local.get(DEFAULTS, saved => {
    settings = { ...DEFAULTS, ...saved };
  });

  chrome.storage.onChanged.addListener((changes, area) => {
    if (area !== 'local') return;
    for (const [key, change] of Object.entries(changes)) settings[key] = change.newValue;
    resetAllKnownState();
  });

  function resetAllKnownState() {
    // WeakMap is intentionally not enumerable. Existing states self-reset on next focus/click.
  }

  function actualTarget(event) {
    const path = event.composedPath?.();
    return (path && path[0]) || event.target;
  }

  function findEditable(node) {
    if (!node || node.nodeType !== Node.ELEMENT_NODE) return null;
    const el = node;
    if (el instanceof HTMLInputElement) {
      const type = (el.type || 'text').toLowerCase();
      if (['password', 'checkbox', 'radio', 'button', 'submit', 'reset', 'file', 'color', 'date', 'datetime-local', 'month', 'time', 'week', 'range', 'number'].includes(type)) return null;
      if (el.readOnly || el.disabled) return null;
      return el;
    }
    if (el instanceof HTMLTextAreaElement) {
      if (el.readOnly || el.disabled) return null;
      return el;
    }
    const host = el.closest?.('[contenteditable="true"], [contenteditable="plaintext-only"]');
    if (host) return host;
    return null;
  }

  function getState(el) {
    let s = states.get(el);
    if (!s) {
      s = {
        engine: new InputKey.Engine(settings),
        lastRendered: ''
      };
      states.set(el, s);
    }
    s.engine.configure(settings);
    return s;
  }

  function resetState(el) {
    const s = states.get(el);
    if (s) {
      s.engine.reset();
      s.lastRendered = '';
    }
  }

  function dispatchInput(el, data, inputType = 'insertText') {
    try {
      el.dispatchEvent(new InputEvent('input', { bubbles: true, composed: true, inputType, data }));
    } catch {
      el.dispatchEvent(new Event('input', { bubbles: true, composed: true }));
    }
  }

  function replaceInputBeforeCursor(el, oldText, newText) {
    const start = el.selectionStart;
    const end = el.selectionEnd;
    if (start == null || end == null) return false;
    const deleteStart = Math.max(0, start - oldText.length);
    internalEdit = true;
    try {
      el.setRangeText(newText, deleteStart, end, 'end');
      dispatchInput(el, newText, oldText ? 'insertReplacementText' : 'insertText');
    } finally {
      internalEdit = false;
    }
    return true;
  }

  function previousTextPoint(root, container, offset, count) {
    let node = container;
    let localOffset = offset;

    if (node.nodeType === Node.TEXT_NODE) {
      if (localOffset >= count) return { node, offset: localOffset - count };
      count -= localOffset;
    }

    const walker = document.createTreeWalker(root, NodeFilter.SHOW_TEXT);
    const nodes = [];
    let n;
    while ((n = walker.nextNode())) nodes.push(n);
    let idx = node.nodeType === Node.TEXT_NODE ? nodes.indexOf(node) - 1 : nodes.length - 1;
    while (idx >= 0) {
      const t = nodes[idx];
      if (t.data.length >= count) return { node: t, offset: t.data.length - count };
      count -= t.data.length;
      idx--;
    }
    return null;
  }

  function replaceContentEditableBeforeCursor(root, oldText, newText) {
    const sel = root.ownerDocument.getSelection();
    if (!sel || sel.rangeCount === 0) return false;
    const current = sel.getRangeAt(0);
    if (!current.collapsed) current.deleteContents();

    const endContainer = current.endContainer;
    const endOffset = current.endOffset;
    const start = previousTextPoint(root, endContainer, endOffset, oldText.length);

    const range = root.ownerDocument.createRange();
    if (oldText.length && start) range.setStart(start.node, start.offset);
    else range.setStart(endContainer, endOffset);
    range.setEnd(endContainer, endOffset);
    sel.removeAllRanges();
    sel.addRange(range);

    internalEdit = true;
    try {
      let ok = false;
      try {
        ok = root.ownerDocument.execCommand('insertText', false, newText);
      } catch { /* fallback below */ }
      if (!ok) {
        range.deleteContents();
        const text = root.ownerDocument.createTextNode(newText);
        range.insertNode(text);
        range.setStartAfter(text);
        range.collapse(true);
        sel.removeAllRanges();
        sel.addRange(range);
        dispatchInput(root, newText, oldText ? 'insertReplacementText' : 'insertText');
      }
    } finally {
      internalEdit = false;
    }
    return true;
  }

  function replaceBeforeCursor(el, oldText, newText) {
    if (el instanceof HTMLInputElement || el instanceof HTMLTextAreaElement) {
      return replaceInputBeforeCursor(el, oldText, newText);
    }
    return replaceContentEditableBeforeCursor(el, oldText, newText);
  }

  function insertDelimiter(el, delimiter) {
    replaceBeforeCursor(el, '', delimiter);
  }

  function showToast(text) {
    if (!settings.showToast || document.documentElement == null) return;
    const id = '__vietnamese_keyboard_toast__';
    document.getElementById(id)?.remove();
    const div = document.createElement('div');
    div.id = id;
    div.textContent = text;
    Object.assign(div.style, {
      position: 'fixed', right: '18px', bottom: '18px', zIndex: '2147483647',
      padding: '9px 12px', borderRadius: '8px', background: '#1f2937', color: '#fff',
      font: '600 13px system-ui, sans-serif', boxShadow: '0 6px 24px rgba(0,0,0,.28)',
      pointerEvents: 'none', opacity: '1', transition: 'opacity .18s ease'
    });
    document.documentElement.appendChild(div);
    setTimeout(() => { div.style.opacity = '0'; setTimeout(() => div.remove(), 220); }, 900);
  }

  function toggleVietnamese() {
    settings.enabled = !settings.enabled;
    chrome.storage.local.set({ enabled: settings.enabled });
    showToast(settings.enabled ? 'InputKey: V' : 'InputKey: E');
  }

  function isPrintableKey(e) {
    return e.key && e.key.length === 1 && !e.ctrlKey && !e.metaKey && !e.altKey;
  }

  function isWordDelimiter(key) {
    return key.length === 1 && /[\s.,;:!?(){}<>"'`~@#$%^&*+=\\/|_-]/.test(key);
  }

  document.addEventListener('keydown', e => {
    if (e.key === 'Control') { chord.ctrl = true; if (chord.shift) chord.armed = true; return; }
    if (e.key === 'Shift') { chord.shift = true; if (chord.ctrl) chord.armed = true; return; }
    if (chord.ctrl || chord.shift) chord.used = true;

    if (e.altKey && !e.ctrlKey && !e.metaKey && e.key.toLowerCase() === 'z') {
      e.preventDefault();
      e.stopImmediatePropagation();
      toggleVietnamese();
      return;
    }

    const el = findEditable(actualTarget(e));
    if (!el) return;

    if (!settings.enabled) {
      resetState(el);
      return;
    }
    if (e.isComposing || e.keyCode === 229) return;

    if (InputKeyLiteralizeShortcut.matchesShortcut(e, settings.literalizeShortcut)) {
      const s = getState(el);
      if (s.engine.raw) {
        e.preventDefault();
        e.stopImmediatePropagation();
        const old = s.lastRendered;
        const raw = s.engine.literalizeToken();
        replaceBeforeCursor(el, old, raw);
        s.lastRendered = raw;
      }
      return;
    }

    if (e.ctrlKey || e.metaKey || e.altKey) {
      if (!['Shift', 'Control'].includes(e.key)) resetState(el);
      return;
    }

    const s = getState(el);

    if (e.key === 'Escape' && s.engine.raw) {
      e.preventDefault();
      const old = s.lastRendered;
      const raw = s.engine.escape();
      replaceBeforeCursor(el, old, raw);
      s.lastRendered = raw;
      return;
    }

    if (e.key === 'Backspace') {
      if (!s.engine.raw) return;
      e.preventDefault();
      const old = s.lastRendered;
      const next = s.engine.backspace();
      replaceBeforeCursor(el, old, next);
      s.lastRendered = next;
      return;
    }

    if (['ArrowLeft','ArrowRight','ArrowUp','ArrowDown','Home','End','PageUp','PageDown','Delete','Insert','Tab','Enter'].includes(e.key)) {
      resetState(el);
      return;
    }

    if (isPrintableKey(e)) {
      if (isWordDelimiter(e.key)) {
        if (s.engine.raw) {
          const old = s.lastRendered;
          const finalText = s.engine.finalize();
          if (finalText !== old) {
            e.preventDefault();
            replaceBeforeCursor(el, old, finalText);
            insertDelimiter(el, e.key);
          }
          resetState(el);
        }
        return;
      }

      // VNI digits and Telex letters are handled by the engine; other symbols reset.
      const valid = settings.method === 'vni' ? /^[A-Za-z0-9]$/.test(e.key) : /^[A-Za-z\[\]]$/.test(e.key);
      if (!valid) { resetState(el); return; }

      e.preventDefault();
      const old = s.lastRendered;
      const next = s.engine.type(e.key);
      replaceBeforeCursor(el, old, next);
      s.lastRendered = next;
    }
  }, true);

  document.addEventListener('keyup', e => {
    if (e.key === 'Control') chord.ctrl = false;
    if (e.key === 'Shift') chord.shift = false;
    if ((e.key === 'Control' || e.key === 'Shift') && chord.armed && !chord.used && (!chord.ctrl || !chord.shift)) {
      chord.armed = false;
      chord.used = true;
      toggleVietnamese();
    }
    if (!chord.ctrl && !chord.shift) chord = { ctrl: false, shift: false, armed: false, used: false };
  }, true);

  document.addEventListener('mousedown', e => {
    const el = findEditable(actualTarget(e));
    if (el) resetState(el);
  }, true);

  document.addEventListener('paste', e => {
    const el = findEditable(actualTarget(e));
    if (el) resetState(el);
  }, true);

  document.addEventListener('cut', e => {
    const el = findEditable(actualTarget(e));
    if (el) resetState(el);
  }, true);

  document.addEventListener('drop', e => {
    const el = findEditable(actualTarget(e));
    if (el) resetState(el);
  }, true);

  document.addEventListener('beforeinput', e => {
    if (internalEdit) return;
    const el = findEditable(actualTarget(e));
    if (!el) return;
    if (e.inputType && !['insertText'].includes(e.inputType)) resetState(el);
  }, true);
})();
