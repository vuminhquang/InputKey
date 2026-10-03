#include <gtk/gtk.h>

#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "inputkey.h"
#include "inputkey_settings.h"

#ifndef INPUTKEY_VERSION
#define INPUTKEY_VERSION "dev"
#endif

static InputKeyLinuxSettings settings;
static GtkComboBoxText *language_combo;
static GtkComboBoxText *method_combo;
static GtkWidget *simple_telex;
static GtkWidget *auto_restore;
static GtkWidget *smart_correction;
static gboolean refreshing = FALSE;

typedef size_t (*IndexedStringFn)(size_t, uint8_t *, size_t);
typedef size_t (*LanguageIndexedStringFn)(const char *, size_t, uint8_t *, size_t);

static char *indexed_string(IndexedStringFn fn, size_t index) {
    size_t required = fn(index, NULL, 0);
    char *value = g_malloc0(required + 1);
    if (!value) return NULL;
    fn(index, (uint8_t *)value, required + 1);
    return value;
}

static char *language_indexed_string(
    LanguageIndexedStringFn fn, const char *language, size_t index) {
    size_t required = fn(language, index, NULL, 0);
    char *value = g_malloc0(required + 1);
    if (!value) return NULL;
    fn(language, index, (uint8_t *)value, required + 1);
    return value;
}

static int *option_slot(const char *id) {
    if (g_strcmp0(id, "simple_telex") == 0) return &settings.simple_telex;
    if (g_strcmp0(id, "auto_restore") == 0) return &settings.auto_restore;
    if (g_strcmp0(id, "smart_correction") == 0) return &settings.smart_correction;
    return NULL;
}

static GtkWidget *option_widget(const char *id) {
    if (g_strcmp0(id, "simple_telex") == 0) return simple_telex;
    if (g_strcmp0(id, "auto_restore") == 0) return auto_restore;
    if (g_strcmp0(id, "smart_correction") == 0) return smart_correction;
    return NULL;
}

static size_t language_index(const char *language) {
    size_t count = inputkey_language_count();
    for (size_t i = 0; i < count; ++i) {
        char *id = indexed_string(inputkey_language_id, i);
        gboolean match = id && g_strcmp0(id, language) == 0;
        g_free(id);
        if (match) return i;
    }
    return (size_t)-1;
}

static void reset_language_defaults(const char *language) {
    size_t index = language_index(language);
    if (index == (size_t)-1) return;

    char *method = indexed_string(inputkey_language_default_method, index);
    if (method) {
        g_strlcpy(settings.method, method, sizeof(settings.method));
        g_free(method);
    }

    size_t options = inputkey_option_count(language);
    for (size_t i = 0; i < options; ++i) {
        char *id = language_indexed_string(inputkey_option_id, language, i);
        if (id) {
            int *slot = option_slot(id);
            if (slot) *slot = inputkey_option_default_enabled(language, i) != 0;
        }
        g_free(id);
    }
}

static void save_settings(void) {
    inputkey_linux_settings_save(&settings);
}

static void refresh_methods(void) {
    refreshing = TRUE;
    gtk_combo_box_text_remove_all(method_combo);

    const char *language = settings.language;
    size_t count = inputkey_method_count(language);
    gboolean selected = FALSE;
    for (size_t i = 0; i < count; ++i) {
        char *id = language_indexed_string(inputkey_method_id, language, i);
        char *label = language_indexed_string(inputkey_method_label, language, i);
        if (id) {
            gtk_combo_box_text_append(method_combo, id, label ? label : id);
            if (g_strcmp0(id, settings.method) == 0) selected = TRUE;
        }
        g_free(id);
        g_free(label);
    }

    if (!selected && count > 0) {
        size_t index = language_index(language);
        char *fallback = index == (size_t)-1
            ? NULL
            : indexed_string(inputkey_language_default_method, index);
        if (fallback) {
            g_strlcpy(settings.method, fallback, sizeof(settings.method));
            save_settings();
            g_free(fallback);
        }
    }

    gtk_combo_box_set_active_id(GTK_COMBO_BOX(method_combo), settings.method);
    refreshing = FALSE;
}

static void refresh_options(void) {
    GtkWidget *known[] = { simple_telex, auto_restore, smart_correction };
    for (size_t i = 0; i < G_N_ELEMENTS(known); ++i) {
        gtk_widget_hide(known[i]);
    }

    const char *language = settings.language;
    size_t count = inputkey_option_count(language);
    refreshing = TRUE;
    for (size_t i = 0; i < count; ++i) {
        char *id = language_indexed_string(inputkey_option_id, language, i);
        char *label = language_indexed_string(inputkey_option_label, language, i);
        GtkWidget *widget = id ? option_widget(id) : NULL;
        int *slot = id ? option_slot(id) : NULL;
        if (widget && slot) {
            gtk_button_set_label(GTK_BUTTON(widget), label ? label : id);
            gtk_toggle_button_set_active(GTK_TOGGLE_BUTTON(widget), *slot != 0);
            gtk_widget_show(widget);
        }
        g_free(id);
        g_free(label);
    }
    refreshing = FALSE;
}

static void refresh_all(void) {
    refreshing = TRUE;
    gtk_combo_box_text_remove_all(language_combo);

    size_t count = inputkey_language_count();
    gboolean selected = FALSE;
    for (size_t i = 0; i < count; ++i) {
        char *id = indexed_string(inputkey_language_id, i);
        char *name = indexed_string(inputkey_language_name, i);
        if (id) {
            gtk_combo_box_text_append(language_combo, id, name ? name : id);
            if (g_strcmp0(id, settings.language) == 0) selected = TRUE;
        }
        g_free(id);
        g_free(name);
    }

    if (!selected && count > 0) {
        char *fallback = indexed_string(inputkey_language_id, 0);
        if (fallback) {
            g_strlcpy(settings.language, fallback, sizeof(settings.language));
            reset_language_defaults(settings.language);
            save_settings();
            g_free(fallback);
        }
    }
    gtk_combo_box_set_active_id(GTK_COMBO_BOX(language_combo), settings.language);
    refreshing = FALSE;

    refresh_methods();
    refresh_options();
}

static void language_changed(GtkComboBox *combo, gpointer user_data) {
    (void)user_data;
    if (refreshing) return;

    const char *language = gtk_combo_box_get_active_id(combo);
    if (!language || !*language) return;
    g_strlcpy(settings.language, language, sizeof(settings.language));
    reset_language_defaults(language);
    save_settings();
    refresh_methods();
    refresh_options();
}

static void method_changed(GtkComboBox *combo, gpointer user_data) {
    (void)user_data;
    if (refreshing) return;

    const char *method = gtk_combo_box_get_active_id(combo);
    if (!method || !*method) return;
    g_strlcpy(settings.method, method, sizeof(settings.method));
    save_settings();
}

static void option_changed(GtkToggleButton *button, gpointer user_data) {
    if (refreshing) return;
    const char *id = user_data;
    int *slot = option_slot(id);
    if (!slot) return;
    *slot = gtk_toggle_button_get_active(button) ? 1 : 0;
    save_settings();
}

static GtkWidget *field_label(const char *text) {
    GtkWidget *label = gtk_label_new(text);
    gtk_widget_set_halign(label, GTK_ALIGN_START);
    return label;
}

int main(int argc, char **argv) {
    gtk_init(&argc, &argv);
    inputkey_linux_settings_load(&settings);

    GtkWidget *window = gtk_window_new(GTK_WINDOW_TOPLEVEL);
    gtk_window_set_title(GTK_WINDOW(window), "InputKey Settings");
    gtk_window_set_default_size(GTK_WINDOW(window), 480, 430);
    gtk_container_set_border_width(GTK_CONTAINER(window), 24);
    g_signal_connect(window, "destroy", G_CALLBACK(gtk_main_quit), NULL);

    GtkWidget *root = gtk_box_new(GTK_ORIENTATION_VERTICAL, 14);
    gtk_container_add(GTK_CONTAINER(window), root);

    GtkWidget *title = gtk_label_new(NULL);
    gtk_label_set_markup(GTK_LABEL(title), "<span size='xx-large' weight='bold'>InputKey</span>");
    gtk_widget_set_halign(title, GTK_ALIGN_START);
    gtk_box_pack_start(GTK_BOX(root), title, FALSE, FALSE, 0);

    char version[128];
    g_snprintf(version, sizeof(version), "Native Linux settings · v%s", INPUTKEY_VERSION);
    GtkWidget *version_label = gtk_label_new(version);
    gtk_widget_set_halign(version_label, GTK_ALIGN_START);
    gtk_style_context_add_class(gtk_widget_get_style_context(version_label), "dim-label");
    gtk_box_pack_start(GTK_BOX(root), version_label, FALSE, FALSE, 0);

    GtkWidget *grid = gtk_grid_new();
    gtk_grid_set_row_spacing(GTK_GRID(grid), 12);
    gtk_grid_set_column_spacing(GTK_GRID(grid), 18);
    gtk_box_pack_start(GTK_BOX(root), grid, FALSE, FALSE, 8);

    gtk_grid_attach(GTK_GRID(grid), field_label("Language"), 0, 0, 1, 1);
    language_combo = GTK_COMBO_BOX_TEXT(gtk_combo_box_text_new());
    gtk_widget_set_hexpand(GTK_WIDGET(language_combo), TRUE);
    gtk_grid_attach(GTK_GRID(grid), GTK_WIDGET(language_combo), 1, 0, 1, 1);

    gtk_grid_attach(GTK_GRID(grid), field_label("Input method"), 0, 1, 1, 1);
    method_combo = GTK_COMBO_BOX_TEXT(gtk_combo_box_text_new());
    gtk_widget_set_hexpand(GTK_WIDGET(method_combo), TRUE);
    gtk_grid_attach(GTK_GRID(grid), GTK_WIDGET(method_combo), 1, 1, 1, 1);

    simple_telex = gtk_check_button_new_with_label("Simple Telex");
    auto_restore = gtk_check_button_new_with_label("Auto Restore");
    smart_correction = gtk_check_button_new_with_label("Smart correction");
    gtk_box_pack_start(GTK_BOX(root), simple_telex, FALSE, FALSE, 0);
    gtk_box_pack_start(GTK_BOX(root), auto_restore, FALSE, FALSE, 0);
    gtk_box_pack_start(GTK_BOX(root), smart_correction, FALSE, FALSE, 0);

    GtkWidget *hint = gtk_label_new(
        "Space commits the current token. Shift+Space keeps the physical key sequence "
        "and ends composition without inserting a space.");
    gtk_label_set_line_wrap(GTK_LABEL(hint), TRUE);
    gtk_widget_set_halign(hint, GTK_ALIGN_START);
    gtk_style_context_add_class(gtk_widget_get_style_context(hint), "dim-label");
    gtk_box_pack_start(GTK_BOX(root), hint, FALSE, FALSE, 8);

    GtkWidget *note = gtk_label_new(
        "Changes are stored locally and are picked up when InputKey is activated or focused.");
    gtk_label_set_line_wrap(GTK_LABEL(note), TRUE);
    gtk_widget_set_halign(note, GTK_ALIGN_START);
    gtk_box_pack_start(GTK_BOX(root), note, FALSE, FALSE, 0);

    GtkWidget *close = gtk_button_new_with_label("Close");
    gtk_widget_set_halign(close, GTK_ALIGN_END);
    g_signal_connect_swapped(close, "clicked", G_CALLBACK(gtk_widget_destroy), window);
    gtk_box_pack_end(GTK_BOX(root), close, FALSE, FALSE, 0);

    g_signal_connect(language_combo, "changed", G_CALLBACK(language_changed), NULL);
    g_signal_connect(method_combo, "changed", G_CALLBACK(method_changed), NULL);
    g_signal_connect(simple_telex, "toggled", G_CALLBACK(option_changed), "simple_telex");
    g_signal_connect(auto_restore, "toggled", G_CALLBACK(option_changed), "auto_restore");
    g_signal_connect(smart_correction, "toggled", G_CALLBACK(option_changed), "smart_correction");

    refresh_all();
    gtk_widget_show_all(window);
    refresh_options();

    gtk_main();
    return 0;
}
