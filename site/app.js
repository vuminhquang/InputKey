const demoContent = {
  vi: {
    label: "Bạn gõ",
    raw: "dduwocj",
    resultLabel: "InputKey hiểu",
    output: "được",
    caption: "Smart Correction xử lý thứ tự modifier khi bạn kết thúc từ."
  },
  fr: {
    label: "Vous tapez",
    raw: "garccon",
    resultLabel: "InputKey écrit",
    output: "garçon",
    caption: "InputKey French Telex transforme les caractères sans quitter le clavier ASCII."
  }
};

const demoButtons = document.querySelectorAll("[data-demo-language]");
const demoLabel = document.getElementById("demo-label");
const demoRaw = document.getElementById("demo-raw");
const demoResultLabel = document.getElementById("demo-result-label");
const demoOutput = document.getElementById("demo-output");
const demoCaption = document.getElementById("demo-caption");

for (const button of demoButtons) {
  button.addEventListener("click", () => {
    const language = button.dataset.demoLanguage;
    const content = demoContent[language];
    if (!content) return;

    for (const item of demoButtons) {
      item.classList.toggle("active", item === button);
    }

    demoLabel.textContent = content.label;
    demoRaw.textContent = content.raw;
    demoResultLabel.textContent = content.resultLabel;
    demoOutput.textContent = content.output;
    demoCaption.textContent = content.caption;
  });
}

const guideTabs = document.querySelectorAll("[data-guide]");
const guidePanels = document.querySelectorAll("[data-guide-panel]");

for (const tab of guideTabs) {
  tab.addEventListener("click", () => {
    const guide = tab.dataset.guide;

    for (const item of guideTabs) {
      const active = item === tab;
      item.classList.toggle("active", active);
      item.setAttribute("aria-selected", String(active));
    }

    for (const panel of guidePanels) {
      panel.classList.toggle("active", panel.dataset.guidePanel === guide);
    }
  });
}
