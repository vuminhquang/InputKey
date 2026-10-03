/* SPDX-License-Identifier: GPL-2.0-or-later */
(() => {
  'use strict';

  const MODIFIERS = {
    ctrl: 'Ctrl', alt: 'Alt', shift: 'Shift', meta: 'Meta'
  };

  function modifierName(key) {
    return ({ Control: 'ctrl', Alt: 'alt', Shift: 'shift', Meta: 'meta' })[key] || null;
  }

  function normalizeKey(value) {
    const raw = String(value || '').trim();
    if (!raw) return null;
    if (raw.length === 1) return raw.toUpperCase();
    const lower = raw.toLowerCase();
    const names = {
      space: 'Space', tab: 'Tab', enter: 'Enter', return: 'Enter',
      esc: 'Esc', escape: 'Esc', backspace: 'Backspace',
      delete: 'Delete', del: 'Delete', insert: 'Insert', ins: 'Insert',
      home: 'Home', end: 'End', pageup: 'PageUp', page_up: 'PageUp',
      pagedown: 'PageDown', page_down: 'PageDown',
      left: 'Left', right: 'Right', up: 'Up', down: 'Down'
    };
    if (names[lower]) return names[lower];
    if (/^f(?:[1-9]|1[0-2])$/i.test(raw)) return raw.toUpperCase();
    return null;
  }

  function parse(value) {
    const raw = String(value ?? '').trim();
    if (!raw || /^(off|none)$/i.test(raw)) {
      return { valid: true, disabled: true, modifiers: { ctrl:false, alt:false, shift:false, meta:false }, key: null };
    }
    const modifiers = { ctrl:false, alt:false, shift:false, meta:false };
    let key = null;
    for (const piece of raw.split('+')) {
      const token = piece.trim();
      if (!token) return { valid: false };
      const lower = token.toLowerCase();
      if (lower === 'ctrl' || lower === 'control') modifiers.ctrl = true;
      else if (lower === 'alt' || lower === 'option') modifiers.alt = true;
      else if (lower === 'shift') modifiers.shift = true;
      else if (lower === 'meta' || lower === 'cmd' || lower === 'command' || lower === 'win' || lower === 'windows' || lower === 'super') modifiers.meta = true;
      else {
        if (key) return { valid: false };
        key = normalizeKey(token);
        if (!key) return { valid: false };
      }
    }
    const count = Object.values(modifiers).filter(Boolean).length;
    if (!count) return { valid: false };
    if (modifiers.ctrl && modifiers.alt) return { valid: false };
    if (!key && count < 2) return { valid: false };
    return { valid: true, disabled: false, modifiers, key };
  }

  function normalize(value) {
    const parsed = parse(value);
    if (!parsed.valid) return null;
    if (parsed.disabled) return 'Off';
    const parts = [];
    for (const name of ['ctrl','alt','shift','meta']) if (parsed.modifiers[name]) parts.push(MODIFIERS[name]);
    if (parsed.key) parts.push(parsed.key);
    return parts.join('+');
  }

  function eventKey(event) {
    const key = event.key;
    if (key === ' ') return 'Space';
    if (key === 'Escape') return 'Esc';
    if (key?.startsWith('Arrow')) return key.slice(5);
    if (key?.length === 1) return key.toUpperCase();
    return normalizeKey(key);
  }

  function eventModifiers(event) {
    return { ctrl: !!event.ctrlKey, alt: !!event.altKey, shift: !!event.shiftKey, meta: !!event.metaKey };
  }

  function sameModifiers(expected, actual) {
    return ['ctrl','alt','shift','meta'].every(name => !!expected[name] === !!actual[name]);
  }

  function matchesKeyDown(parsed, event) {
    return !!parsed?.valid && !parsed.disabled && !!parsed.key &&
      !modifierName(event.key) && sameModifiers(parsed.modifiers, eventModifiers(event)) &&
      eventKey(event) === parsed.key;
  }

  globalThis.InputKeyShortcut = { parse, normalize, modifierName, sameModifiers, matchesKeyDown };
})();
