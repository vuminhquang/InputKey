/* SPDX-License-Identifier: GPL-2.0-or-later */
// Chrome extension leaf helpers for the user-configurable literalize chord.
(() => {
  'use strict';
  const DEFAULT_LITERALIZE_SHORTCUT = Object.freeze({ enabled: true, code: 'Space', ctrl: true, shift: false, alt: false, meta: false });
  const MODIFIERS = ['ctrl', 'shift', 'alt', 'meta'];
  function normalizeShortcut(value) {
    if (value == null || typeof value !== 'object') return { ...DEFAULT_LITERALIZE_SHORTCUT };
    const input = value && typeof value === 'object' ? value : {};
    return {
      enabled: input.enabled !== false,
      code: typeof input.code === 'string' && input.code ? input.code : DEFAULT_LITERALIZE_SHORTCUT.code,
      ctrl: Boolean(input.ctrl), shift: Boolean(input.shift), alt: Boolean(input.alt), meta: Boolean(input.meta)
    };
  }
  function matchesShortcut(event, value) {
    const shortcut = normalizeShortcut(value);
    return shortcut.enabled && event.code === shortcut.code && MODIFIERS.every(mod => Boolean(event[`${mod}Key`]) === shortcut[mod]);
  }
  function shortcutLabel(value) {
    const shortcut = normalizeShortcut(value);
    const parts = [];
    if (shortcut.ctrl) parts.push('Ctrl');
    if (shortcut.alt) parts.push('Alt');
    if (shortcut.shift) parts.push('Shift');
    if (shortcut.meta) parts.push('Meta');
    const names = { Space: 'Space', Semicolon: ';', Escape: 'Escape', Backquote: '`', Minus: '-', Equal: '=', BracketLeft: '[', BracketRight: ']', Backslash: '\\', Quote: "'", Comma: ',', Period: '.', Slash: '/' };
    parts.push(names[shortcut.code] || shortcut.code.replace(/^Key/, '').replace(/^Digit/, ''));
    return parts.join(' + ');
  }
  function validateRecordedShortcut(value) {
    const shortcut = normalizeShortcut(value);
    return MODIFIERS.some(mod => shortcut[mod]) ? { valid: true, shortcut } : { valid: false, error: 'Hãy dùng ít nhất một phím Ctrl, Alt, Shift hoặc Meta.' };
  }
  globalThis.InputKeyLiteralizeShortcut = { DEFAULT_LITERALIZE_SHORTCUT, normalizeShortcut, matchesShortcut, shortcutLabel, validateRecordedShortcut };
})();
