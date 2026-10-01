/* SPDX-License-Identifier: GPL-2.0-or-later */
const DEFAULTS = {
  enabled: true,
  method: 'telex',
  simpleTelex: false,
  autoRestore: true,
  showToast: true,
  literalizeShortcut: { enabled: true, code: 'Space', ctrl: true, shift: false, alt: false, meta: false }
};

async function refreshBadge() {
  const { enabled } = await chrome.storage.local.get(DEFAULTS);
  await chrome.action.setBadgeText({ text: enabled ? 'V' : 'E' });
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
  if (area === 'local' && changes.enabled) refreshBadge();
});
refreshBadge();
