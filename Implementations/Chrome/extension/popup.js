/* SPDX-License-Identifier: GPL-2.0-or-later */
const DEFAULTS = {
  enabled: true,
  language: 'vi',
  method: 'telex',
  simpleTelex: false,
  autoRestore: true,
  smartCorrection: true,
  showToast: true
};
const $ = id => document.getElementById(id);
$('version').textContent = 'v' + chrome.runtime.getManifest().version;

let catalog = { languages: [] };
let current = { ...DEFAULTS };

const storageKey = id => ({
  simple_telex: 'simpleTelex',
  auto_restore: 'autoRestore',
  smart_correction: 'smartCorrection'
}[id] || id);

function languageById(id) {
  return catalog.languages.find(language => language.id === id) || catalog.languages[0];
}

function engineSettings(state = current) {
  return {
    language: state.language,
    method: state.method,
    simpleTelex: !!state.simpleTelex,
    autoRestore: !!state.autoRestore,
    smartCorrection: !!state.smartCorrection
  };
}

function setOptions(select, items, value) {
  select.replaceChildren(...items.map(item => {
    const option = document.createElement('option');
    option.value = item.id;
    option.textContent = item.label || item.nativeName || item.displayName || item.id;
    option.selected = item.id === value;
    return option;
  }));
}

function renderLanguageOptions(language, state) {
  const host = $('languageOptions');
  host.replaceChildren();
  for (const option of language?.options || []) {
    const label = document.createElement('label');
    label.className = 'check';
    const input = document.createElement('input');
    input.type = 'checkbox';
    input.dataset.optionId = option.id;
    const key = storageKey(option.id);
    input.checked = state[key] ?? option.defaultEnabled;
    input.addEventListener('change', async () => {
      await chrome.storage.local.set({ [key]: input.checked });
    });
    const span = document.createElement('span');
    span.textContent = option.label;
    label.append(input, span);
    host.append(label);
  }
}

function renderHint(language) {
  if (!language) {
    $('hint').textContent = '';
    return;
  }
  if (language.id === 'fr') {
    $('hint').innerHTML =
      '<strong>French Telex:</strong> ee→ê · es→é · ef→è · aa/ii/oo/uu→â/î/ô/û · ex/ix/ux/yx→ë/ï/ü/ÿ · oe→œ · ae→æ · cc→ç';
  } else {
    $('hint').innerHTML =
      '<strong>Vietnamese Telex:</strong> aa→â · aw→ă · dd→đ · s/f/r/x/j→dấu thanh<br><strong>VNI:</strong> 6/7/8/9→dấu chữ · 1–5→dấu thanh';
  }
}

function paint(state) {
  current = { ...DEFAULTS, ...state };
  const language = languageById(current.language);
  if (language && current.language !== language.id) current.language = language.id;

  setOptions($('language'), catalog.languages, current.language);

  const methodItems = language?.methods || [];
  const validMethod = methodItems.some(method => method.id === current.method)
    ? current.method
    : language?.defaultMethod;
  if (validMethod) current.method = validMethod;
  setOptions($('method'), methodItems, current.method);

  $('toggle').textContent = current.enabled
    ? (language?.id || 'ON').toUpperCase()
    : 'OFF';
  $('toggle').classList.toggle('off', !current.enabled);
  $('showToast').checked = current.showToast;
  renderLanguageOptions(language, current);
  renderHint(language);
}

async function load() {
  paint(await chrome.storage.local.get(DEFAULTS));
}

$('toggle').addEventListener('click', async () => {
  await chrome.storage.local.set({ enabled: !current.enabled });
});

$('language').addEventListener('change', async e => {
  const language = languageById(e.target.value);
  if (!language) return;
  const update = { language: language.id, method: language.defaultMethod };
  for (const option of language.options || []) {
    update[storageKey(option.id)] = option.defaultEnabled;
  }
  await chrome.storage.local.set(update);
});

$('method').addEventListener('change', e =>
  chrome.storage.local.set({ method: e.target.value })
);
$('showToast').addEventListener('change', e =>
  chrome.storage.local.set({ showToast: e.target.checked })
);
chrome.storage.onChanged.addListener((_, area) => {
  if (area === 'local') load();
});

(async () => {
  await InputKey.ready;
  catalog = InputKey.catalog();
  await load();

  const tryBox = $('try');
  const tester = new InputKey.Engine(engineSettings());
  let testerRendered = '';

  function replaceOwned(oldText, newText) {
    const end = tryBox.selectionStart;
    tryBox.setRangeText(
      newText,
      Math.max(0, end - oldText.length),
      tryBox.selectionEnd,
      'end'
    );
  }

  function configureTester() {
    tester.configure(engineSettings());
  }

  tryBox.addEventListener('keydown', e => {
    configureTester();

    if (
      e.code === 'Space' && e.shiftKey &&
      !e.ctrlKey && !e.metaKey && !e.altKey && tester.raw
    ) {
      e.preventDefault();
      const raw = tester.rawBoundary();
      replaceOwned(testerRendered, raw);
      testerRendered = '';
      return;
    }

    if (e.ctrlKey || e.metaKey || e.altKey || e.isComposing) {
      if (tester.raw) tester.shortcutBoundary();
      testerRendered = '';
      return;
    }

    if (e.key === 'Backspace' && tester.raw) {
      e.preventDefault();
      const next = tester.compositionControl('backspace');
      replaceOwned(testerRendered, next);
      testerRendered = next;
      return;
    }

    if (e.key === 'Escape' && tester.raw) {
      e.preventDefault();
      const next = tester.compositionControl('escape');
      replaceOwned(testerRendered, next);
      testerRendered = next;
      return;
    }

    if (['ArrowLeft','ArrowRight','ArrowUp','ArrowDown','Home','End','PageUp','PageDown','Delete','Insert','Tab','Enter'].includes(e.key)) {
      if (tester.raw) {
        const cause = ({ArrowLeft:'left',ArrowRight:'right',ArrowUp:'up',ArrowDown:'down',Home:'home',End:'end',PageUp:'page_up',PageDown:'page_down',Delete:'delete',Insert:'insert',Tab:'tab',Enter:'enter'})[e.key] || 'other';
        tester.caretMoveBoundary(cause);
      }
      testerRendered = '';
      return;
    }

    if (e.key.length !== 1) return;

    if (tester.acceptsCharacter(e.key)) {
      e.preventDefault();
      const next = tester.character(e.key);
      replaceOwned(testerRendered, next);
      testerRendered = next;
      return;
    }

    if (tester.raw) {
      e.preventDefault();
      const committed = e.key === ' ' ? tester.spaceBoundary() : tester.punctuationBoundary(e.key);
      replaceOwned(testerRendered, committed);
      testerRendered = '';
    }
  });

  tryBox.addEventListener('mousedown', () => {
    if (tester.raw) tester.caretMoveBoundary('mouse');
    testerRendered = '';
  });

  chrome.storage.onChanged.addListener((_, area) => {
    if (area === 'local') {
      tester.lifecycle('reset');
      testerRendered = '';
    }
  });
})();
