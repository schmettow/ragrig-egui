//! Small form helpers for dense settings menus.
//!
//! Component menus in agent panels are long stacks of "label and widget"
//! groups and one-line notes. These helpers keep that layout consistent
//! without pulling a form/derive framework into the host:
//!
//! - [`setting_row`] — a label and its widget side by side,
//! - [`note`] — quiet secondary text (states, hints),
//! - [`warning`] — the same, in the warning colour (attention required).
//!
//! All of them are plain [`egui`] calls; the host keeps its own layout
//! decisions around them.

use egui::Ui;

/// Label column width in points — wide enough for labels like
/// "Context window (tokens)" at the default text size.
const LABEL_WIDTH: f32 = 132.0;

/// Render `label` and a widget side by side.
///
/// The widget closure receives a [`Ui`] continuing on the same row; widgets
/// that size to `ui.available_width()` fill the remaining space.
///
/// ```
/// # use ragrig_egui::form::setting_row;
/// # fn ui(ui: &mut egui::Ui, model: &mut String) {
/// setting_row(ui, "Model", |ui| {
///     ui.text_edit_singleline(model);
/// });
/// # }
/// ```
pub fn setting_row(ui: &mut Ui, label: &str, widget: impl FnOnce(&mut Ui)) {
    ui.horizontal(|ui| {
        ui.add_sized(
            egui::vec2(LABEL_WIDTH, ui.spacing().interact_size.y),
            egui::Label::new(label).truncate(),
        );
        widget(ui);
    });
}

/// Quiet one-line note under a setting or action.
pub fn note(ui: &mut Ui, text: &str) {
    ui.label(egui::RichText::new(text).weak());
}

/// One-line note in the warning colour — used when the user has to act
/// (a stale index, a missing backend, a disabled dependency).
pub fn warning(ui: &mut Ui, text: &str) {
    let color = ui.visuals().warn_fg_color;
    ui.label(egui::RichText::new(text).color(color));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn helpers_render_headless() {
        let ctx = egui::Context::default();
        let mut clicked = false;
        let _ = ctx.run_ui(egui::RawInput::default(), |ui| {
            setting_row(ui, "Model", |ui| {
                if ui.button("press me").clicked() {
                    clicked = true;
                }
            });
            note(ui, "a quiet note");
            warning(ui, "attention required");
        });
        assert!(!clicked, "no input, no click");
    }

    #[test]
    fn setting_row_runs_the_widget_closure() {
        let ctx = egui::Context::default();
        let mut ran = false;
        let _ = ctx.run_ui(egui::RawInput::default(), |ui| {
            setting_row(ui, "Label", |_ui| ran = true);
        });
        assert!(ran);
    }

    #[test]
    fn long_labels_do_not_panic() {
        let ctx = egui::Context::default();
        let _ = ctx.run_ui(egui::RawInput::default(), |ui| {
            setting_row(
                ui,
                "A label far longer than the label column will ever be",
                |ui| {
                    ui.label("value");
                },
            );
        });
    }
}
