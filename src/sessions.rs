//! Session picker for the agent panel.
//!
//! [`SessionRow`] renders the conventional line above the transcript: a *New*
//! button, a dropdown of saved conversations, and a refresh action at the
//! bottom of the list. It is the visible half of ragrig's session store —
//! switching a conversation means handing a session id back to the host,
//! which loads it into its worker.
//!
//! The host owns the store and the labels (they are usually localized): it
//! passes [`SessionEntry`] rows — pre-formatted, e.g. `12 turns · 3 h · about
//! the methods section` — and maps the returned [`SessionAction`].
//!
//! ```no_run
//! # use ragrig_egui::sessions::{SessionAction, SessionEntry, SessionRow};
//! # fn ui(ui: &mut egui::Ui, entries: &[SessionEntry], current: Option<&str>) {
//! let row = SessionRow::new(entries, current).busy(false);
//! match row.show(ui) {
//!     Some(SessionAction::New) => {}     // start a fresh conversation
//!     Some(SessionAction::Refresh) => {} // re-read the store
//!     Some(SessionAction::Load(id)) => {} // load that conversation
//!     None => {}
//! }
//! # }
//! ```

use egui::Ui;

/// One saved conversation in the picker.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionEntry {
    /// Identifier the host uses to load the session (ragrig: the session id).
    pub id: String,
    /// Row text, pre-formatted and localized by the host.
    pub label: String,
}

/// Labels of the picker, including the button captions (prepend a glyph from
/// your icon font where you want one).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionLabels {
    /// Caption of the *New* button.
    pub new: String,
    /// Tooltip of the *New* button.
    pub new_hint: String,
    /// Shown in the dropdown while no conversation is active.
    pub current: String,
    /// Shown in an empty dropdown.
    pub empty: String,
    /// Caption of the refresh action (bottom of the dropdown).
    pub refresh: String,
}

impl Default for SessionLabels {
    fn default() -> Self {
        Self {
            new: "New".to_string(),
            new_hint: "Start a new conversation".to_string(),
            current: "Conversation".to_string(),
            empty: "No saved conversations yet.".to_string(),
            refresh: "Refresh list".to_string(),
        }
    }
}

/// What the user asked the picker to do.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SessionAction {
    /// Start a fresh conversation.
    New,
    /// Re-read the session store.
    Refresh,
    /// Load the conversation with this id.
    Load(String),
}

/// The conversation picker row. State (the entries, the current session) is
/// owned by the host.
pub struct SessionRow<'a> {
    entries: &'a [SessionEntry],
    current: Option<&'a str>,
    busy: bool,
    width: f32,
    labels: SessionLabels,
}

impl<'a> SessionRow<'a> {
    /// A picker over `entries` with `current` (a [`SessionEntry::id`]) active.
    pub fn new(entries: &'a [SessionEntry], current: Option<&'a str>) -> Self {
        Self {
            entries,
            current,
            busy: false,
            width: 150.0,
            labels: SessionLabels::default(),
        }
    }

    /// Set while the worker is busy: *New* is disabled, switching is not
    /// (the host may still queue the load).
    pub fn busy(mut self, busy: bool) -> Self {
        self.busy = busy;
        self
    }

    /// Override the (usually localized) labels.
    pub fn labels(mut self, labels: SessionLabels) -> Self {
        self.labels = labels;
        self
    }

    /// Width of the dropdown itself (the popup is wider, see
    /// [`POPUP_WIDTH`]).
    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    /// Render the row into `ui`.
    pub fn show(self, ui: &mut Ui) -> Option<SessionAction> {
        let mut action = None;
        ui.horizontal(|ui| {
            if ui
                .add_enabled(!self.busy, egui::Button::new(self.labels.new.clone()))
                .on_hover_text(&self.labels.new_hint)
                .clicked()
            {
                action = Some(SessionAction::New);
            }
            let selected = selected_text(self.entries, self.current, &self.labels);
            egui::ComboBox::from_id_salt("ragrig_egui::sessions::picker")
                .selected_text(selected)
                .width(self.width)
                .show_ui(ui, |ui| {
                    ui.set_min_width(POPUP_WIDTH);
                    if self.entries.is_empty() {
                        ui.weak(&self.labels.empty);
                    }
                    for entry in self.entries {
                        let is_current = self.current == Some(entry.id.as_str());
                        if ui.selectable_label(is_current, &entry.label).clicked() {
                            action = Some(SessionAction::Load(entry.id.clone()));
                        }
                    }
                    ui.separator();
                    if ui.button(&self.labels.refresh).clicked() {
                        action = Some(SessionAction::Refresh);
                    }
                });
        });
        action
    }
}

/// The dropdown is wider than the row's combo box: session labels carry
/// `turns · age · summary` and need the space.
pub const POPUP_WIDTH: f32 = 240.0;

/// The combo box caption: the current entry's label, or the *no conversation*
/// text while the host has not loaded one.
fn selected_text(
    entries: &[SessionEntry],
    current: Option<&str>,
    labels: &SessionLabels,
) -> String {
    current
        .and_then(|id| entries.iter().find(|entry| entry.id == id))
        .map_or_else(|| labels.current.clone(), |entry| entry.label.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::leaf_widgets;

    fn entries() -> Vec<SessionEntry> {
        vec![
            SessionEntry {
                id: "s1".into(),
                label: "12 turns · 3 h · methods".into(),
            },
            SessionEntry {
                id: "s2".into(),
                label: "4 turns · 2 d".into(),
            },
        ]
    }

    #[test]
    fn the_caption_follows_the_current_session() {
        let labels = SessionLabels::default();
        let entries = entries();
        assert_eq!(
            selected_text(&entries, Some("s1"), &labels),
            "12 turns · 3 h · methods"
        );
        assert_eq!(selected_text(&entries, None, &labels), labels.current);
        assert_eq!(
            selected_text(&entries, Some("gone"), &labels),
            labels.current,
            "an id the store no longer has falls back to the neutral caption"
        );
    }

    #[test]
    fn the_row_renders_all_states_without_panic() {
        let ctx = egui::Context::default();
        let entries = entries();
        for current in [Some("s1"), None] {
            for busy in [true, false] {
                let _ = ctx.run_ui(egui::RawInput::default(), |ui| {
                    let action = SessionRow::new(&entries, current).busy(busy).show(ui);
                    assert_eq!(action, None, "no input, no action");
                });
            }
        }
        let empty: Vec<SessionEntry> = Vec::new();
        let _ = ctx.run_ui(egui::RawInput::default(), |ui| {
            SessionRow::new(&empty, None).show(ui);
        });
    }

    #[test]
    fn the_new_button_reports_and_respects_busy() {
        let ctx = egui::Context::default();
        let entries = entries();
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(320.0, 60.0));
        let frame = |busy: bool, events: Vec<egui::Event>| {
            let input = egui::RawInput {
                screen_rect: Some(screen),
                events,
                ..egui::RawInput::default()
            };
            let mut action = None;
            let _ = ctx.run_ui(input, |ui| {
                action = SessionRow::new(&entries, None).busy(busy).show(ui);
            });
            action
        };
        let click_new = |busy: bool| {
            assert_eq!(frame(busy, vec![]), None, "no input, no action");
            // Frame 1 settled the layout: the New button is the leftmost
            // control of the row.
            let target = leaf_widgets(&ctx)[0].center();
            let press = |pressed| egui::Event::PointerButton {
                pos: target,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: egui::Modifiers::default(),
            };
            frame(
                busy,
                vec![egui::Event::PointerMoved(target), press(true), press(false)],
            )
        };
        assert_eq!(click_new(false), Some(SessionAction::New));
        assert_eq!(click_new(true), None, "busy disables the New button");
    }
}
