#include <fcitx/action.h>
#include <fcitx/addonfactory.h>
#include <fcitx/addonmanager.h>
#include <fcitx/inputcontext.h>
#include <fcitx/inputcontextmanager.h>
#include <fcitx/inputcontextproperty.h>
#include <fcitx/inputmethodengine.h>
#include <fcitx/inputpanel.h>
#include <fcitx/instance.h>
#include <fcitx/menu.h>
#include <fcitx/text.h>
#include <fcitx/userinterfacemanager.h>
#include <fcitx-utils/key.h>
#include <fcitx-utils/keysym.h>
#include <fcitx-utils/utf8.h>

#include <algorithm>
#include <cstdint>
#include <cstdio>
#include <cstring>
#include <memory>
#include <string>
#include <utility>
#include <vector>

#include "inputkey.h"
#include "inputkey_settings.h"

namespace {

template <typename F>
static std::string take(F call) {
    std::vector<uint8_t> buffer(4096);
    auto n = call(buffer.data(), buffer.size());
    return std::string(
        reinterpret_cast<char *>(buffer.data()),
        std::min(n, buffer.size() - 1));
}

static std::string indexed(
    size_t (*fn)(size_t, uint8_t *, size_t), size_t index) {
    return take([&](uint8_t *out, size_t cap) {
        return fn(index, out, cap);
    });
}

static std::string member(
    size_t (*fn)(const char *, size_t, uint8_t *, size_t),
    const std::string &language,
    size_t index) {
    return take([&](uint8_t *out, size_t cap) {
        return fn(language.c_str(), index, out, cap);
    });
}

static std::string optionsJson(const InputKeyLinuxSettings &settings) {
    char json[192];
    std::snprintf(
        json,
        sizeof(json),
        "{\"simple_telex\":%s,\"auto_restore\":%s,\"smart_correction\":%s}",
        settings.simple_telex ? "true" : "false",
        settings.auto_restore ? "true" : "false",
        settings.smart_correction ? "true" : "false");
    return json;
}

static int *optionSlot(InputKeyLinuxSettings &settings, const std::string &id) {
    if (id == "simple_telex") return &settings.simple_telex;
    if (id == "auto_restore") return &settings.auto_restore;
    if (id == "smart_correction") return &settings.smart_correction;
    return nullptr;
}

static const int *optionSlot(
    const InputKeyLinuxSettings &settings, const std::string &id) {
    if (id == "simple_telex") return &settings.simple_telex;
    if (id == "auto_restore") return &settings.auto_restore;
    if (id == "smart_correction") return &settings.smart_correction;
    return nullptr;
}

class VKState : public fcitx::InputContextProperty {
public:
    VKState() = default;
    ~VKState() override {
        if (handle_) inputkey_destroy(handle_);
    }

    uint64_t handle() const { return handle_; }

    void rebuild(const InputKeyLinuxSettings &settings) {
        if (handle_) inputkey_destroy(handle_);
        const auto json = optionsJson(settings);
        handle_ = inputkey_create_ex(
            settings.language, settings.method, json.c_str());
    }

private:
    uint64_t handle_ = 0;
};

} // namespace

class InputKeyEngine;

class InputKeyChoiceAction : public fcitx::SimpleAction {
public:
    enum class Kind { Language, Method, Option };

    InputKeyChoiceAction(
        InputKeyEngine *engine,
        Kind kind,
        std::string language,
        std::string id,
        std::string label);

    bool isChecked(fcitx::InputContext *ic) const override;
    void activate(fcitx::InputContext *ic) override;

private:
    InputKeyEngine *engine_;
    Kind kind_;
    std::string language_;
    std::string id_;
};

class InputKeyEngine : public fcitx::InputMethodEngineV2 {
public:
    explicit InputKeyEngine(fcitx::Instance *instance)
        : instance_(instance),
          factory_([this](fcitx::InputContext &) {
              auto *state = new VKState();
              state->rebuild(settings_);
              return state;
          }) {
        inputkey_linux_settings_load(&settings_);
        validateSettings();
        instance_->inputContextManager().registerProperty(
            "inputkey-state", &factory_);
        buildActions();
    }

    bool choiceChecked(
        InputKeyChoiceAction::Kind kind,
        const std::string &language,
        const std::string &id) const {
        switch (kind) {
        case InputKeyChoiceAction::Kind::Language:
            return language == settings_.language;
        case InputKeyChoiceAction::Kind::Method:
            return language == settings_.language && id == settings_.method;
        case InputKeyChoiceAction::Kind::Option: {
            if (language != settings_.language) return false;
            const int *slot = optionSlot(settings_, id);
            return slot && *slot;
        }
        }
        return false;
    }

    void activateChoice(
        InputKeyChoiceAction::Kind kind,
        const std::string &language,
        const std::string &id) {
        if (kind == InputKeyChoiceAction::Kind::Language) {
            std::snprintf(
                settings_.language, sizeof(settings_.language), "%s",
                language.c_str());
            applyLanguageDefaults(language);
        } else if (kind == InputKeyChoiceAction::Kind::Method) {
            std::snprintf(
                settings_.language, sizeof(settings_.language), "%s",
                language.c_str());
            std::snprintf(
                settings_.method, sizeof(settings_.method), "%s",
                id.c_str());
        } else {
            std::snprintf(
                settings_.language, sizeof(settings_.language), "%s",
                language.c_str());
            if (int *slot = optionSlot(settings_, id)) {
                *slot = !*slot;
            }
        }

        inputkey_linux_settings_save(&settings_);
        instance_->inputContextManager().foreach([this](fcitx::InputContext *ic) {
            auto *state = ic->propertyFor(&factory_);
            if (state) state->rebuild(settings_);
            return true;
        });
    }

    void keyEvent(
        const fcitx::InputMethodEntry &,
        fcitx::KeyEvent &event) override {
        if (event.isRelease()) return;
        auto *ic = event.inputContext();
        auto *state = ic->propertyFor(&factory_);
        auto handle = state->handle();

        switch (event.key().sym()) {
        case FcitxKey_Left: case FcitxKey_Right:
        case FcitxKey_Up: case FcitxKey_Down:
        case FcitxKey_Home: case FcitxKey_End:
        case FcitxKey_Page_Up: case FcitxKey_Page_Down:
        case FcitxKey_Insert: case FcitxKey_Delete:
        case FcitxKey_Tab: case FcitxKey_Return: case FcitxKey_KP_Enter:
        case FcitxKey_KP_Left: case FcitxKey_KP_Right:
        case FcitxKey_KP_Up: case FcitxKey_KP_Down:
        case FcitxKey_KP_Home: case FcitxKey_KP_End:
        case FcitxKey_KP_Page_Up: case FcitxKey_KP_Page_Down:
        case FcitxKey_KP_Insert: case FcitxKey_KP_Delete:
            flush(ic, handle, true);
            return;
        default:
            break;
        }

        auto states = event.key().states();
        const bool rawBoundary =
            event.key().sym() == FcitxKey_space &&
            states.test(fcitx::KeyState::Shift) &&
            !states.testAny(fcitx::KeyStates{
                fcitx::KeyState::Ctrl,
                fcitx::KeyState::Alt,
                fcitx::KeyState::Super,
                fcitx::KeyState::Hyper});

        if (rawBoundary && inputkey_has_history(handle)) {
            auto raw = take([&](uint8_t *p, size_t n) {
                return inputkey_commit_raw_boundary(handle, p, n);
            });
            ic->commitString(raw);
            clearPreedit(ic);
            event.filterAndAccept();
            return;
        }

        if (states.testAny(fcitx::KeyStates{
                fcitx::KeyState::Ctrl,
                fcitx::KeyState::Alt,
                fcitx::KeyState::Super,
                fcitx::KeyState::Hyper})) {
            flush(ic, handle, true);
            return;
        }

        if (event.key().check(FcitxKey_BackSpace)) {
            if (inputkey_has_history(handle)) {
                updatePreedit(
                    ic,
                    take([&](uint8_t *p, size_t n) {
                        return inputkey_backspace(handle, p, n);
                    }));
                event.filterAndAccept();
            }
            return;
        }

        if (event.key().check(FcitxKey_Escape)) {
            if (inputkey_has_history(handle)) {
                updatePreedit(
                    ic,
                    take([&](uint8_t *p, size_t n) {
                        return inputkey_escape(handle, p, n);
                    }));
                event.filterAndAccept();
            }
            return;
        }

        auto text = fcitx::Key::keySymToUTF8(event.key().sym());
        if (text.size() == 1 && !event.key().isModifier()) {
            const auto c = static_cast<unsigned char>(text[0]);
            if (c >= 0x20 && c <= 0x7e) {
                if (inputkey_accepts_key_utf8(
                        handle,
                        reinterpret_cast<const uint8_t *>(text.data()),
                        text.size())) {
                    updatePreedit(
                        ic,
                        take([&](uint8_t *p, size_t n) {
                            return inputkey_key_utf8(
                                handle,
                                reinterpret_cast<const uint8_t *>(text.data()),
                                text.size(), p, n);
                        }));
                    event.filterAndAccept();
                    return;
                }

                if (inputkey_has_history(handle)) {
                    auto committed = take([&](uint8_t *p, size_t n) {
                        return inputkey_decision_boundary_utf8(
                            handle,
                            reinterpret_cast<const uint8_t *>(text.data()),
                            text.size(), p, n);
                    });
                    ic->commitString(committed);
                    clearPreedit(ic);
                    event.filterAndAccept();
                    return;
                }
            }
        }

        flush(ic, handle, true);
    }

    void activate(
        const fcitx::InputMethodEntry &,
        fcitx::InputContextEvent &event) override {
        event.inputContext()->statusArea().addAction(
            fcitx::StatusGroup::InputMethod, rootAction_.get());
    }

    void reset(
        const fcitx::InputMethodEntry &,
        fcitx::InputContextEvent &event) override {
        auto *ic = event.inputContext();
        auto *state = ic->propertyFor(&factory_);
        inputkey_reset(state->handle());
        clearPreedit(ic);
    }

    void deactivate(
        const fcitx::InputMethodEntry &,
        fcitx::InputContextEvent &event) override {
        auto *ic = event.inputContext();
        auto *state = ic->propertyFor(&factory_);
        if (event.type() == fcitx::EventType::InputContextFocusOut) {
            inputkey_reset(state->handle());
            clearPreedit(ic);
        } else {
            flush(ic, state->handle(), true);
        }
    }

private:
    size_t languageIndex(const std::string &language) const {
        const auto count = inputkey_language_count();
        for (size_t i = 0; i < count; ++i) {
            if (indexed(inputkey_language_id, i) == language) return i;
        }
        return static_cast<size_t>(-1);
    }

    void validateSettings() {
        if (languageIndex(settings_.language) == static_cast<size_t>(-1)) {
            if (inputkey_language_count() == 0) return;
            auto language = indexed(inputkey_language_id, 0);
            std::snprintf(
                settings_.language, sizeof(settings_.language),
                "%s", language.c_str());
            applyLanguageDefaults(language);
        }

        bool methodExists = false;
        const auto count = inputkey_method_count(settings_.language);
        for (size_t i = 0; i < count; ++i) {
            if (member(inputkey_method_id, settings_.language, i) ==
                settings_.method) {
                methodExists = true;
                break;
            }
        }
        if (!methodExists) {
            applyLanguageDefaults(settings_.language);
        }
    }

    void applyLanguageDefaults(const std::string &language) {
        const auto index = languageIndex(language);
        if (index == static_cast<size_t>(-1)) return;

        auto method = indexed(inputkey_language_default_method, index);
        std::snprintf(
            settings_.method, sizeof(settings_.method), "%s", method.c_str());

        const auto optionCount = inputkey_option_count(language.c_str());
        for (size_t i = 0; i < optionCount; ++i) {
            auto id = member(inputkey_option_id, language, i);
            if (int *slot = optionSlot(settings_, id)) {
                *slot = inputkey_option_default_enabled(
                    language.c_str(), i) != 0;
            }
        }
    }

    void buildActions() {
        rootAction_ = std::make_unique<fcitx::SimpleAction>();
        rootAction_->setShortText("InputKey");
        rootMenu_ = std::make_unique<fcitx::Menu>();
        rootAction_->setMenu(rootMenu_.get());
        instance_->userInterfaceManager().registerAction(
            "inputkey-settings", rootAction_.get());

        const auto languageCount = inputkey_language_count();
        for (size_t i = 0; i < languageCount; ++i) {
            auto language = indexed(inputkey_language_id, i);
            auto name = indexed(inputkey_language_name, i);

            auto languageAction = std::make_unique<InputKeyChoiceAction>(
                this,
                InputKeyChoiceAction::Kind::Language,
                language,
                language,
                std::string("Language · ") + name);
            instance_->userInterfaceManager().registerAction(
                std::string("inputkey-language-") + language,
                languageAction.get());
            rootMenu_->addAction(languageAction.get());
            actions_.push_back(std::move(languageAction));

            const auto methodCount = inputkey_method_count(language.c_str());
            for (size_t j = 0; j < methodCount; ++j) {
                auto method = member(inputkey_method_id, language, j);
                auto label = member(inputkey_method_label, language, j);
                auto action = std::make_unique<InputKeyChoiceAction>(
                    this,
                    InputKeyChoiceAction::Kind::Method,
                    language,
                    method,
                    name + " · " + label);
                instance_->userInterfaceManager().registerAction(
                    std::string("inputkey-method-") + language + "-" + method,
                    action.get());
                rootMenu_->addAction(action.get());
                actions_.push_back(std::move(action));
            }

            const auto optionCount = inputkey_option_count(language.c_str());
            for (size_t j = 0; j < optionCount; ++j) {
                auto option = member(inputkey_option_id, language, j);
                auto label = member(inputkey_option_label, language, j);
                auto action = std::make_unique<InputKeyChoiceAction>(
                    this,
                    InputKeyChoiceAction::Kind::Option,
                    language,
                    option,
                    name + " · " + label);
                instance_->userInterfaceManager().registerAction(
                    std::string("inputkey-option-") + language + "-" + option,
                    action.get());
                rootMenu_->addAction(action.get());
                actions_.push_back(std::move(action));
            }
        }
    }

    void updatePreedit(fcitx::InputContext *ic, const std::string &value) {
        fcitx::Text text;
        if (!value.empty()) {
            text.append(value, fcitx::TextFormatFlag::HighLight);
        }
        auto n = fcitx::utf8::lengthValidated(value);
        text.setCursor(
            (value.empty() || n == fcitx::utf8::INVALID_LENGTH) ? 0 : n);
        ic->inputPanel().setClientPreedit(text);
        ic->updatePreedit();
    }

    void clearPreedit(fcitx::InputContext *ic) {
        ic->inputPanel().setClientPreedit(fcitx::Text());
        ic->updatePreedit();
    }

    void flush(
        fcitx::InputContext *ic,
        uint64_t handle,
        bool keepDisplayed) {
        if (!inputkey_has_history(handle)) return;
        auto value = keepDisplayed
            ? take([&](uint8_t *p, size_t n) {
                  return inputkey_natural_boundary(handle, p, n);
              })
            : take([&](uint8_t *p, size_t n) {
                  return inputkey_finalize(handle, p, n);
              });
        if (!value.empty()) ic->commitString(value);
        if (!keepDisplayed) inputkey_reset(handle);
        clearPreedit(ic);
    }

    fcitx::Instance *instance_;
    InputKeyLinuxSettings settings_{};
    fcitx::FactoryFor<VKState> factory_;
    std::unique_ptr<fcitx::SimpleAction> rootAction_;
    std::unique_ptr<fcitx::Menu> rootMenu_;
    std::vector<std::unique_ptr<InputKeyChoiceAction>> actions_;

    friend class InputKeyChoiceAction;
};

InputKeyChoiceAction::InputKeyChoiceAction(
    InputKeyEngine *engine,
    Kind kind,
    std::string language,
    std::string id,
    std::string label)
    : engine_(engine),
      kind_(kind),
      language_(std::move(language)),
      id_(std::move(id)) {
    setShortText(std::move(label));
    setCheckable(true);
}

bool InputKeyChoiceAction::isChecked(fcitx::InputContext *) const {
    return engine_->choiceChecked(kind_, language_, id_);
}

void InputKeyChoiceAction::activate(fcitx::InputContext *) {
    engine_->activateChoice(kind_, language_, id_);
}

class InputKeyFactory : public fcitx::AddonFactory {
public:
    fcitx::AddonInstance *create(fcitx::AddonManager *manager) override {
        return new InputKeyEngine(manager->instance());
    }
};

#ifdef FCITX_ADDON_FACTORY_V2
FCITX_ADDON_FACTORY_V2(inputkey, InputKeyFactory);
#else
FCITX_ADDON_FACTORY(InputKeyFactory);
#endif
