/* SPDX-License-Identifier: GPL-2.0-or-later */
const DEFAULTS = {
  enabled: true,
  language: 'vi',
  method: 'telex',
  simpleTelex: false,
  autoRestore: true,
  smartCorrection: true,
  toggleShortcut: 'Ctrl+Shift',
  showToast: true
};

async function refreshBadge() {
  const { enabled, language } = await chrome.storage.local.get(DEFAULTS);
  const text = enabled ? String(language || 'vi').toUpperCase().slice(0, 2) : 'OFF';
  await chrome.action.setBadgeText({ text });
  await chrome.action.setBadgeBackgroundColor({ color: enabled ? '#16a34a' : '#6b7280' });
}

chrome.runtime.onInstalled.addListener(async () => {
  const current = await chrome.storage.local.get(DEFAULTS);
  await chrome.storage.local.set({ ...DEFAULTS, ...current });
  await chrome.storage.local.remove('doubleCancel');
  await refreshBadge();
});
chrome.runtime.onStartup.addListener(refreshBadge);
chrome.storage.onChanged.addListener((changes, area) => {
  if (area === 'local' && (changes.enabled || changes.language)) refreshBadge();
});
refreshBadge();
