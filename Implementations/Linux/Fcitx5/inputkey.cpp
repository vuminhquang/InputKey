#include <fcitx/addonfactory.h>
#include <fcitx/addonmanager.h>
#include <fcitx/inputcontext.h>
#include <fcitx/inputmethodengine.h>
#include <fcitx/instance.h>
#include <fcitx/inputcontextproperty.h>
#include <fcitx/inputpanel.h>
#include <fcitx-utils/key.h>
#include <fcitx-utils/keysym.h>
#include <fcitx/text.h>
#include <fcitx-utils/utf8.h>
#include <cctype>
#include <string>
#include "inputkey.h"
#include "../ShortcutConfig/shortcut_config.h"
#include <vector>
#include <algorithm>

namespace {
class VKState : public fcitx::InputContextProperty {
public:
    VKState() : h_(inputkey_create("telex", 0, 1)) {}
    ~VKState() override { if (h_) inputkey_destroy(h_); }
    uint64_t h() const { return h_; }
private:
    uint64_t h_;
};

template <typename F> static std::string take(F call) { std::vector<uint8_t> b(4096); auto n=call(b.data(),b.size()); return std::string(reinterpret_cast<char*>(b.data()), std::min(n,b.size()-1)); }
static std::string type(uint64_t h, const std::string &s) { return take([&](uint8_t *p,size_t n){return inputkey_key_utf8(h,reinterpret_cast<const uint8_t*>(s.data()),s.size(),p,n);}); }
}

class InputKeyEngine : public fcitx::InputMethodEngineV2 {
public:
    explicit InputKeyEngine(fcitx::Instance *instance)
        : instance_(instance), factory_([](fcitx::InputContext&) { return new VKState(); }) {
        inputkey_shortcut_reload();
        instance_->inputContextManager().registerProperty("inputkey-state", &factory_);
    }

    void keyEvent(const fcitx::InputMethodEntry &, fcitx::KeyEvent &event) override {
        if (event.isRelease()) return;
        auto *ic = event.inputContext();
        auto *state = ic->propertyFor(&factory_);
        auto h = state->h();

        switch (event.key().sym()) {
        case FcitxKey_Left: case FcitxKey_Right: case FcitxKey_Up: case FcitxKey_Down:
        case FcitxKey_Home: case FcitxKey_End: case FcitxKey_Page_Up: case FcitxKey_Page_Down:
        case FcitxKey_Insert: case FcitxKey_Delete:
        case FcitxKey_KP_Left: case FcitxKey_KP_Right: case FcitxKey_KP_Up: case FcitxKey_KP_Down:
        case FcitxKey_KP_Home: case FcitxKey_KP_End: case FcitxKey_KP_Page_Up: case FcitxKey_KP_Page_Down:
        case FcitxKey_KP_Insert: case FcitxKey_KP_Delete:
            flush(ic, h, true);
            return;
        }

        auto states = event.key().states();
        unsigned shortcut_mods = 0;
        if (states.test(fcitx::KeyState::Ctrl)) shortcut_mods |= INPUTKEY_SHORTCUT_CTRL;
        if (states.test(fcitx::KeyState::Alt)) shortcut_mods |= INPUTKEY_SHORTCUT_ALT;
        if (states.test(fcitx::KeyState::Shift)) shortcut_mods |= INPUTKEY_SHORTCUT_SHIFT;
        if (states.test(fcitx::KeyState::Super)) shortcut_mods |= INPUTKEY_SHORTCUT_SUPER;
        auto chord_text = fcitx::Key::keySymToUTF8(event.key().sym());
        if (chord_text.size() == 1 && inputkey_shortcut_match_configured(shortcut_mods, static_cast<unsigned char>(chord_text[0]))) {
            if (inputkey_has_history(h)) updatePreedit(ic, take([&](uint8_t*p,size_t n){return inputkey_literalize_token(h,p,n);}));
            event.filterAndAccept(); return;
        }
        if (states.testAny(fcitx::KeyStates{fcitx::KeyState::Ctrl, fcitx::KeyState::Alt,
                                           fcitx::KeyState::Super, fcitx::KeyState::Hyper})) {
            flush(ic, h);
            return;
        }

        if (event.key().check(FcitxKey_BackSpace)) {
            if (inputkey_has_history(h)) {
                updatePreedit(ic, take([&](uint8_t*p,size_t n){return inputkey_backspace(h,p,n);}));
                event.filterAndAccept();
            }
            return;
        }
        if (event.key().check(FcitxKey_Escape)) {
            if (inputkey_has_history(h)) {
                updatePreedit(ic, take([&](uint8_t*p,size_t n){return inputkey_escape(h,p,n);}));
                event.filterAndAccept();
            }
            return;
        }

        auto text = fcitx::Key::keySymToUTF8(event.key().sym());
        if (text.size() == 1) {
            unsigned char c = static_cast<unsigned char>(text[0]);
            if ((c >= 0x20 && c <= 0x7e) && !event.key().isModifier()) {
                // Separators finalize the preedit and then pass through normally.
                if (!(std::isalnum(c) || c == '[' || c == ']')) {
                    flush(ic, h);
                    return;
                }
                updatePreedit(ic, type(h, text));
                event.filterAndAccept();
                return;
            }
        }

        flush(ic, h);
    }

    void reset(const fcitx::InputMethodEntry &, fcitx::InputContextEvent &event) override {
        inputkey_shortcut_reload();
        auto *ic = event.inputContext();
        auto *state = ic->propertyFor(&factory_);
        inputkey_reset(state->h());
        clearPreedit(ic);
    }

    void deactivate(const fcitx::InputMethodEntry &, fcitx::InputContextEvent &event) override {
        auto *ic = event.inputContext();
        auto *state = ic->propertyFor(&factory_);
        if (event.type() == fcitx::EventType::InputContextFocusOut) {
            inputkey_reset(state->h());
            clearPreedit(ic);
        } else {
            flush(ic, state->h());
        }
    }

private:
    void updatePreedit(fcitx::InputContext *ic, const std::string &s) {
        fcitx::Text text;
        if (!s.empty()) text.append(s, fcitx::TextFormatFlag::HighLight);
        auto n = fcitx::utf8::lengthValidated(s);
        text.setCursor((s.empty() || n == fcitx::utf8::INVALID_LENGTH) ? 0 : n);
        ic->inputPanel().setClientPreedit(text);
        ic->updatePreedit();
    }
    void clearPreedit(fcitx::InputContext *ic) {
        ic->inputPanel().setClientPreedit(fcitx::Text());
        ic->updatePreedit();
    }
    void flush(fcitx::InputContext *ic, uint64_t h, bool keepDisplayed = false) {
        if (!inputkey_has_history(h)) return;
        auto s = keepDisplayed ? take([&](uint8_t*p,size_t n){return inputkey_rendered(h,p,n);}) : take([&](uint8_t*p,size_t n){return inputkey_finalize(h,p,n);});
        if (!s.empty()) ic->commitString(s);
        inputkey_reset(h);
        clearPreedit(ic);
    }

    fcitx::Instance *instance_;
    fcitx::FactoryFor<VKState> factory_;
};

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
