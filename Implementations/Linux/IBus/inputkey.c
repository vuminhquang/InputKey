#include <ibus.h>
#include <stdint.h>
#include <string.h>
#include "inputkey.h"
#include "../ShortcutConfig/shortcut_config.h"
#include <stdlib.h>
#include <string.h>

typedef struct _InputKeyEngine {
    IBusEngine parent;
    uint64_t core;
} InputKeyEngine;
typedef struct _InputKeyEngineClass { IBusEngineClass parent; } InputKeyEngineClass;

G_DEFINE_TYPE(InputKeyEngine, inputkey_engine, IBUS_TYPE_ENGINE)

static char *take_core(size_t (*fn)(uint64_t,uint8_t*,size_t), uint64_t h) { uint8_t *b=g_malloc0(4096); size_t n=fn(h,b,4096); if(n>4095)n=4095; return (char*)b; }
static char *type_core(uint64_t h,const char *s) { uint8_t *b=g_malloc0(4096); size_t n=inputkey_key_utf8(h,(const uint8_t*)s,strlen(s),b,4096); if(n>4095)n=4095; return (char*)b; }
static void update_preedit(InputKeyEngine *self, char *s) {
    IBusText *text = ibus_text_new_from_string(s ? s : "");
    guint cursor = s ? (guint)g_utf8_strlen(s, -1) : 0;
    ibus_engine_update_preedit_text(IBUS_ENGINE(self), text, cursor, s && *s);
    if (s) g_free(s);
}
static void clear_preedit(InputKeyEngine *self) {
    IBusText *text = ibus_text_new_from_static_string("");
    ibus_engine_update_preedit_text(IBUS_ENGINE(self), text, 0, FALSE);
}
static void commit_core(InputKeyEngine *self, gboolean keep_displayed) {
    if (!inputkey_has_history(self->core)) return;
    char *s = keep_displayed ? take_core(inputkey_rendered,self->core) : take_core(inputkey_finalize,self->core);
    if (s && *s) ibus_engine_commit_text(IBUS_ENGINE(self), ibus_text_new_from_string(s));
    if (s) g_free(s);
    inputkey_reset(self->core);
    clear_preedit(self);
}
static void flush_core(InputKeyEngine *self) { commit_core(self, FALSE); }

static gboolean process_key_event(IBusEngine *engine, guint keyval, guint keycode, guint state) {
    (void)keycode;
    InputKeyEngine *self = (InputKeyEngine*)engine;
    if (state & IBUS_RELEASE_MASK) return FALSE;
    switch (keyval) {
    case IBUS_Left: case IBUS_Right: case IBUS_Up: case IBUS_Down:
    case IBUS_Home: case IBUS_End: case IBUS_Page_Up: case IBUS_Page_Down:
    case IBUS_Insert: case IBUS_Delete:
    case IBUS_KP_Left: case IBUS_KP_Right: case IBUS_KP_Up: case IBUS_KP_Down:
    case IBUS_KP_Home: case IBUS_KP_End: case IBUS_KP_Page_Up: case IBUS_KP_Page_Down:
    case IBUS_KP_Insert: case IBUS_KP_Delete:
        commit_core(self, TRUE);
        return FALSE;
    }
    unsigned shortcut_mods = 0;
    if (state & IBUS_CONTROL_MASK) shortcut_mods |= INPUTKEY_SHORTCUT_CTRL;
    if (state & IBUS_MOD1_MASK) shortcut_mods |= INPUTKEY_SHORTCUT_ALT;
    if (state & IBUS_SHIFT_MASK) shortcut_mods |= INPUTKEY_SHORTCUT_SHIFT;
    if (state & IBUS_SUPER_MASK) shortcut_mods |= INPUTKEY_SHORTCUT_SUPER;
    gunichar chord_char = ibus_keyval_to_unicode(keyval);
    if (chord_char <= 0x7f && inputkey_shortcut_match_configured(shortcut_mods, (unsigned char)chord_char)) {
        if (inputkey_has_history(self->core)) update_preedit(self, take_core(inputkey_literalize_token,self->core));
        return TRUE;
    }
    if (state & (IBUS_CONTROL_MASK | IBUS_MOD1_MASK | IBUS_SUPER_MASK | IBUS_META_MASK | IBUS_HYPER_MASK)) {
        flush_core(self); return FALSE;
    }
    if (keyval == IBUS_BackSpace) {
        if (!inputkey_has_history(self->core)) return FALSE;
        update_preedit(self, take_core(inputkey_backspace,self->core));
        return TRUE;
    }
    if (keyval == IBUS_Escape) {
        if (!inputkey_has_history(self->core)) return FALSE;
        update_preedit(self, take_core(inputkey_escape,self->core));
        return TRUE;
    }
    gunichar uc = ibus_keyval_to_unicode(keyval);
    if (uc >= 0x20 && uc <= 0x7e) {
        char utf8[8] = {0};
        gint n = g_unichar_to_utf8(uc, utf8); utf8[n] = 0;
        gboolean token_char = g_ascii_isalnum((gchar)uc) || uc == '[' || uc == ']';
        if (!token_char) { flush_core(self); return FALSE; }
        update_preedit(self, type_core(self->core, utf8));
        return TRUE;
    }
    flush_core(self);
    return FALSE;
}
static void reset_engine(IBusEngine *engine) {
    InputKeyEngine *self = (InputKeyEngine*)engine;
    inputkey_reset(self->core); clear_preedit(self);
    inputkey_shortcut_reload();
}
static void focus_out(IBusEngine *engine) { reset_engine(engine); }
static void disable_engine(IBusEngine *engine) { flush_core((InputKeyEngine*)engine); }
static void finalize_obj(GObject *obj) {
    InputKeyEngine *self = (InputKeyEngine*)obj;
    if (self->core) inputkey_destroy(self->core);
    G_OBJECT_CLASS(inputkey_engine_parent_class)->finalize(obj);
}
static void inputkey_engine_init(InputKeyEngine *self) {
    self->core = inputkey_create("telex", 0, 1);
    inputkey_shortcut_reload();
}
static void inputkey_engine_class_init(InputKeyEngineClass *klass) {
    IBusEngineClass *ec = IBUS_ENGINE_CLASS(klass);
    ec->process_key_event = process_key_event;
    ec->reset = reset_engine;
    ec->focus_out = focus_out;
    ec->disable = disable_engine;
    G_OBJECT_CLASS(klass)->finalize = finalize_obj;
}

static IBusBus *bus;
static IBusFactory *factory;
static void disconnected(IBusBus *b, gpointer data) { (void)b; (void)data; ibus_quit(); }
int main(int argc, char **argv) {
    (void)argc; (void)argv;
    ibus_init();
    bus = ibus_bus_new();
    g_signal_connect(bus, "disconnected", G_CALLBACK(disconnected), NULL);
    factory = ibus_factory_new(ibus_bus_get_connection(bus));
    ibus_factory_add_engine(factory, "inputkey", inputkey_engine_get_type());
    if (ibus_bus_is_connected(bus)) ibus_bus_request_name(bus, "org.freedesktop.IBus.InputKey", 0);
    ibus_main();
    g_object_unref(factory); g_object_unref(bus);
    return 0;
}
