#include <ibus.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>

#include "inputkey.h"
#include "inputkey_settings.h"

typedef struct _InputKeyEngine {
    IBusEngine parent;
    uint64_t core;
    InputKeyLinuxSettings settings;
} InputKeyEngine;

typedef struct _InputKeyEngineClass {
    IBusEngineClass parent;
} InputKeyEngineClass;

G_DEFINE_TYPE(InputKeyEngine, inputkey_engine, IBUS_TYPE_ENGINE)

static char *take_core(size_t (*fn)(uint64_t,uint8_t*,size_t), uint64_t h) {
    uint8_t *b = g_malloc0(4096);
    size_t n = fn(h, b, 4096);
    if (n > 4095) n = 4095;
    return (char *)b;
}

typedef size_t (*InputKeyArgCommand)(
    uint64_t, const uint8_t *, size_t, uint8_t *, size_t);

static char *arg_core(InputKeyArgCommand fn, uint64_t h, const char *s) {
    uint8_t *b = g_malloc0(4096);
    size_t n = fn(h, (const uint8_t *)s, strlen(s), b, 4096);
    if (n > 4095) n = 4095;
    return (char *)b;
}

static char *type_core(uint64_t h, const char *s) {
    return arg_core(inputkey_character_utf8, h, s);
}

static char *boundary_core(uint64_t h, const char *s) {
    if (strcmp(s, " ") == 0) return take_core(inputkey_space_boundary, h);
    return arg_core(inputkey_punctuation_boundary_utf8, h, s);
}

static char *caret_core(uint64_t h, const char *cause) {
    return arg_core(inputkey_caret_move_boundary_utf8, h, cause);
}

static char *control_core(uint64_t h, const char *control) {
    return arg_core(inputkey_composition_control_utf8, h, control);
}

static char *lifecycle_core(uint64_t h, const char *event) {
    return arg_core(inputkey_lifecycle_utf8, h, event);
}

static char *catalog_index_string(
    size_t (*fn)(size_t,uint8_t*,size_t), size_t index) {
    size_t n = fn(index, NULL, 0);
    char *b = g_malloc0(n + 1);
    fn(index, (uint8_t *)b, n + 1);
    return b;
}

static char *catalog_member_string(
    size_t (*fn)(const char*,size_t,uint8_t*,size_t),
    const char *language,
    size_t index) {
    size_t n = fn(language, index, NULL, 0);
    char *b = g_malloc0(n + 1);
    fn(language, index, (uint8_t *)b, n + 1);
    return b;
}

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

static void commit_text_owned(InputKeyEngine *self, char *s) {
    if (s && *s) {
        ibus_engine_commit_text(
            IBUS_ENGINE(self), ibus_text_new_from_string(s));
    }
    if (s) g_free(s);
    clear_preedit(self);
}

static void commit_core(InputKeyEngine *self, gboolean keep_displayed) {
    if (!self->core || !inputkey_has_history(self->core)) return;
    char *s = keep_displayed
        ? caret_core(self->core, "other")
        : lifecycle_core(self->core, "finalize");
    commit_text_owned(self, s);
}

static int *option_slot(InputKeyLinuxSettings *settings, const char *id) {
    if (strcmp(id, "simple_telex") == 0) return &settings->simple_telex;
    if (strcmp(id, "auto_restore") == 0) return &settings->auto_restore;
    if (strcmp(id, "smart_correction") == 0) return &settings->smart_correction;
    return NULL;
}

static void create_core(InputKeyEngine *self) {
    if (self->core) inputkey_destroy(self->core);
    char options[192];
    snprintf(options, sizeof(options),
             "{\"simple_telex\":%s,\"auto_restore\":%s,\"smart_correction\":%s}",
             self->settings.simple_telex ? "true" : "false",
             self->settings.auto_restore ? "true" : "false",
             self->settings.smart_correction ? "true" : "false");
    self->core = inputkey_create_ex(
        self->settings.language, self->settings.method, options);
}

static size_t language_index(const char *language) {
    size_t count = inputkey_language_count();
    for (size_t i = 0; i < count; ++i) {
        char *id = catalog_index_string(inputkey_language_id, i);
        gboolean same = strcmp(id, language) == 0;
        g_free(id);
        if (same) return i;
    }
    return (size_t)-1;
}

static void reset_language_defaults(InputKeyEngine *self, const char *language) {
    size_t index = language_index(language);
    if (index == (size_t)-1) return;
    char *method = catalog_index_string(inputkey_language_default_method, index);
    g_strlcpy(self->settings.method, method, sizeof(self->settings.method));
    g_free(method);

    size_t options = inputkey_option_count(language);
    for (size_t i = 0; i < options; ++i) {
        char *id = catalog_member_string(inputkey_option_id, language, i);
        int *slot = option_slot(&self->settings, id);
        if (slot) *slot = inputkey_option_default_enabled(language, i) != 0;
        g_free(id);
    }
}

static IBusProperty *radio_property(
    const char *key, const char *label, gboolean checked) {
    return ibus_property_new(
        key, PROP_TYPE_RADIO, ibus_text_new_from_string(label), "",
        NULL, TRUE, TRUE,
        checked ? PROP_STATE_CHECKED : PROP_STATE_UNCHECKED, NULL);
}

static void refresh_properties(InputKeyEngine *self) {
    IBusPropList *root = ibus_prop_list_new();

    IBusPropList *languages = ibus_prop_list_new();
    size_t language_count = inputkey_language_count();
    for (size_t i = 0; i < language_count; ++i) {
        char *id = catalog_index_string(inputkey_language_id, i);
        char *name = catalog_index_string(inputkey_language_name, i);
        char *key = g_strdup_printf("language:%s", id);
        ibus_prop_list_append(
            languages,
            radio_property(key, name, strcmp(id, self->settings.language) == 0));
        g_free(key);
        g_free(name);
        g_free(id);
    }
    ibus_prop_list_append(
        root,
        ibus_property_new(
            "language", PROP_TYPE_MENU, ibus_text_new_from_static_string("Language"),
            "", NULL, TRUE, TRUE, PROP_STATE_UNCHECKED, languages));

    IBusPropList *methods = ibus_prop_list_new();
    size_t method_count = inputkey_method_count(self->settings.language);
    for (size_t i = 0; i < method_count; ++i) {
        char *id = catalog_member_string(
            inputkey_method_id, self->settings.language, i);
        char *label = catalog_member_string(
            inputkey_method_label, self->settings.language, i);
        char *key = g_strdup_printf("method:%s", id);
        ibus_prop_list_append(
            methods,
            radio_property(key, label, strcmp(id, self->settings.method) == 0));
        g_free(key);
        g_free(label);
        g_free(id);
    }
    ibus_prop_list_append(
        root,
        ibus_property_new(
            "method", PROP_TYPE_MENU, ibus_text_new_from_static_string("Method"),
            "", NULL, TRUE, TRUE, PROP_STATE_UNCHECKED, methods));

    size_t option_count = inputkey_option_count(self->settings.language);
    for (size_t i = 0; i < option_count; ++i) {
        char *id = catalog_member_string(
            inputkey_option_id, self->settings.language, i);
        char *label = catalog_member_string(
            inputkey_option_label, self->settings.language, i);
        int *slot = option_slot(&self->settings, id);
        char *key = g_strdup_printf("option:%s", id);
        ibus_prop_list_append(
            root,
            ibus_property_new(
                key, PROP_TYPE_TOGGLE, ibus_text_new_from_string(label),
                "", NULL, TRUE, TRUE,
                slot && *slot ? PROP_STATE_CHECKED : PROP_STATE_UNCHECKED,
                NULL));
        g_free(key);
        g_free(label);
        g_free(id);
    }

    ibus_engine_register_properties(IBUS_ENGINE(self), root);
}

static void apply_settings(InputKeyEngine *self) {
    inputkey_linux_settings_save(&self->settings);
    create_core(self);
    refresh_properties(self);
    clear_preedit(self);
}

static const char *caret_cause(guint keyval) {
    switch (keyval) {
    case IBUS_Left: case IBUS_KP_Left: return "left";
    case IBUS_Right: case IBUS_KP_Right: return "right";
    case IBUS_Up: case IBUS_KP_Up: return "up";
    case IBUS_Down: case IBUS_KP_Down: return "down";
    case IBUS_Home: case IBUS_KP_Home: return "home";
    case IBUS_End: case IBUS_KP_End: return "end";
    case IBUS_Page_Up: case IBUS_KP_Page_Up: return "page_up";
    case IBUS_Page_Down: case IBUS_KP_Page_Down: return "page_down";
    case IBUS_Insert: case IBUS_KP_Insert: return "insert";
    case IBUS_Delete: case IBUS_KP_Delete: return "delete";
    case IBUS_Tab: return "tab";
    case IBUS_Return: case IBUS_KP_Enter: return "enter";
    default: return NULL;
    }
}

static gboolean process_key_event(
    IBusEngine *engine, guint keyval, guint keycode, guint state) {
    (void)keycode;
    InputKeyEngine *self = (InputKeyEngine *)engine;
    if (state & IBUS_RELEASE_MASK) return FALSE;

    const char *cause = caret_cause(keyval);
    if (cause) {
        if (inputkey_has_history(self->core)) {
            commit_text_owned(self, caret_core(self->core, cause));
        }
        return FALSE;
    }

    gboolean raw_boundary = keyval == IBUS_space
        && (state & IBUS_SHIFT_MASK)
        && !(state & (IBUS_CONTROL_MASK | IBUS_MOD1_MASK |
                      IBUS_SUPER_MASK | IBUS_META_MASK | IBUS_HYPER_MASK));
    if (raw_boundary && inputkey_has_history(self->core)) {
        commit_text_owned(
            self, take_core(inputkey_raw_boundary, self->core));
        return TRUE;
    }

    gboolean ctrl = (state & IBUS_CONTROL_MASK) != 0;
    gboolean alt = (state & IBUS_MOD1_MASK) != 0;
    gboolean command_chord =
        (state & (IBUS_CONTROL_MASK | IBUS_MOD1_MASK |
                  IBUS_SUPER_MASK | IBUS_META_MASK | IBUS_HYPER_MASK))
        && !(ctrl && alt);
    if (command_chord) {
        if (inputkey_has_history(self->core)) {
            commit_text_owned(self, take_core(inputkey_shortcut_boundary, self->core));
        }
        return FALSE;
    }

    if (keyval == IBUS_BackSpace) {
        if (!inputkey_has_history(self->core)) return FALSE;
        update_preedit(self, control_core(self->core, "backspace"));
        return TRUE;
    }

    if (keyval == IBUS_Escape) {
        if (!inputkey_has_history(self->core)) return FALSE;
        update_preedit(self, control_core(self->core, "escape"));
        return TRUE;
    }

    gunichar uc = ibus_keyval_to_unicode(keyval);
    if (uc >= 0x20 && uc <= 0x7e) {
        char utf8[8] = {0};
        gint n = g_unichar_to_utf8(uc, utf8);
        utf8[n] = 0;

        if (inputkey_accepts_character_utf8(
                self->core, (const uint8_t *)utf8, (size_t)n)) {
            update_preedit(self, type_core(self->core, utf8));
            return TRUE;
        }
        if (inputkey_has_history(self->core)) {
            commit_text_owned(self, boundary_core(self->core, utf8));
            return TRUE;
        }
        return FALSE;
    }

    commit_core(self, TRUE);
    return FALSE;
}

static void property_activate(
    IBusEngine *engine, const gchar *prop_name, guint prop_state) {
    InputKeyEngine *self = (InputKeyEngine *)engine;
    if (inputkey_has_history(self->core)) {
        commit_text_owned(self, lifecycle_core(self->core, "language_changed"));
    }

    if (g_str_has_prefix(prop_name, "language:")) {
        const char *language = prop_name + strlen("language:");
        g_strlcpy(
            self->settings.language, language, sizeof(self->settings.language));
        reset_language_defaults(self, language);
    } else if (g_str_has_prefix(prop_name, "method:")) {
        g_strlcpy(
            self->settings.method,
            prop_name + strlen("method:"),
            sizeof(self->settings.method));
    } else if (g_str_has_prefix(prop_name, "option:")) {
        const char *id = prop_name + strlen("option:");
        int *slot = option_slot(&self->settings, id);
        if (slot) *slot = prop_state == PROP_STATE_CHECKED;
    }
    apply_settings(self);
}

static void reset_engine(IBusEngine *engine) {
    InputKeyEngine *self = (InputKeyEngine *)engine;
    char *ignored = lifecycle_core(self->core, "reset");
    g_free(ignored);
    clear_preedit(self);
}

static void focus_in(IBusEngine *engine) {
    refresh_properties((InputKeyEngine *)engine);
}

static void focus_out(IBusEngine *engine) {
    InputKeyEngine *self = (InputKeyEngine *)engine;
    if (inputkey_has_history(self->core)) {
        commit_text_owned(self, lifecycle_core(self->core, "focus_lost"));
    } else {
        clear_preedit(self);
    }
}

static void enable_engine(IBusEngine *engine) {
    refresh_properties((InputKeyEngine *)engine);
}

static void disable_engine(IBusEngine *engine) {
    InputKeyEngine *self = (InputKeyEngine *)engine;
    if (inputkey_has_history(self->core)) {
        commit_text_owned(self, lifecycle_core(self->core, "disabled"));
    }
}

static void finalize_obj(GObject *obj) {
    InputKeyEngine *self = (InputKeyEngine *)obj;
    if (self->core) inputkey_destroy(self->core);
    G_OBJECT_CLASS(inputkey_engine_parent_class)->finalize(obj);
}

static void inputkey_engine_init(InputKeyEngine *self) {
    self->core = 0;
    inputkey_linux_settings_load(&self->settings);
    create_core(self);
}

static void inputkey_engine_class_init(InputKeyEngineClass *klass) {
    IBusEngineClass *ec = IBUS_ENGINE_CLASS(klass);
    ec->process_key_event = process_key_event;
    ec->property_activate = property_activate;
    ec->focus_in = focus_in;
    ec->focus_out = focus_out;
    ec->enable = enable_engine;
    ec->disable = disable_engine;
    ec->reset = reset_engine;
    G_OBJECT_CLASS(klass)->finalize = finalize_obj;
}

static IBusBus *bus;
static IBusFactory *factory;

static void disconnected(IBusBus *b, gpointer data) {
    (void)b;
    (void)data;
    ibus_quit();
}

int main(int argc, char **argv) {
    (void)argc;
    (void)argv;
    ibus_init();
    bus = ibus_bus_new();
    g_signal_connect(bus, "disconnected", G_CALLBACK(disconnected), NULL);
    factory = ibus_factory_new(ibus_bus_get_connection(bus));
    ibus_factory_add_engine(factory, "inputkey", inputkey_engine_get_type());
    if (ibus_bus_is_connected(bus)) {
        ibus_bus_request_name(bus, "org.freedesktop.IBus.InputKey", 0);
    }
    ibus_main();
    g_object_unref(factory);
    g_object_unref(bus);
    return 0;
}
