//! Pending-context chips shown above the composer.
//!
//! An agent panel collects context for the next question: the active document,
//! the bibliography, a marked passage, a regenerated block. [`show`] renders
//! that collection as rows of chips — the visible half of the attachment
//! contract (ragrig's `AttachedDocument` is the other one).
//!
//! Three kinds cover the workflow:
//!
//! - [`Attachment::toggle`] — something that rides along while it is on (the
//!   document buffer, the library): a selected button, a click flips it,
//! - [`Attachment::pending`] — context that waits for the next question (a
//!   marked passage): a ✕ drops it,
//! - [`Attachment::offer`] — a result the user can accept or drop (the
//!   replacement of a failed co-edit): an action button plus the ✕.
//!
//! The host owns the state: it builds the chips from its own model each frame
//! and maps the returned [`AttachmentEvent`] back to it. A chip with
//! `enabled == false` renders dimmed and explains itself in its tooltip (e.g.
//! "this document has no text to attach") — a chip never promises an
//! attachment it cannot make.
//!
//! ```no_run
//! # use ragrig_egui::attachments::{show, Attachment, AttachmentEvent};
//! # fn ui(ui: &mut egui::Ui, attached: &mut bool, library: &mut bool) {
//! let document = Attachment::toggle("buffer", "P", "draft.md")
//!     .on(*attached)
//!     .hint("The active document rides along with every question");
//! let bibliography = Attachment::toggle("library", "L", "Library")
//!     .on(*library)
//!     .hint("The bibliography rides along with every question");
//! // The two toggles share a row; a pending context gets its own below.
//! let rows: [&[Attachment]; 2] = [
//!     &[document, bibliography],
//!     &[Attachment::pending("selection", "S", "Marked region (312 chars)")
//!         .drop("\u{2715}", "Drop the marked region")],
//! ];
//! if let Some(event) = show(ui, &rows) {
//!     match event {
//!         AttachmentEvent::Toggled(id) if id == "buffer" => *attached = !*attached,
//!         AttachmentEvent::Toggled(_) => *library = !*library,
//!         AttachmentEvent::Dropped(_) => {} // forget the pending context
//!         AttachmentEvent::Accepted(_) => {} // e.g. insert the offered text
//!     }
//! }
//! # }
//! ```

use egui::Ui;

/// How a chip behaves.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AttachmentKind {
    /// A toggle: rendered selected while `on`; a click flips it.
    Toggle {
        /// The state the host rendered the chip in.
        on: bool,
    },
    /// Context that waits for the next question; a ✕ drops it.
    Pending {
        /// Glyph of the drop button (host icon font).
        drop_glyph: String,
        /// Tooltip of the drop button.
        drop_hint: String,
    },
    /// A ready artefact: an action button (`Insert`) plus the ✕.
    Offer {
        /// Label of the action button.
        action: String,
        /// Tooltip of the action button (empty for none).
        action_hint: String,
        /// Glyph of the drop button (host icon font).
        drop_glyph: String,
        /// Tooltip of the drop button.
        drop_hint: String,
    },
}

/// One chip above the composer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Attachment {
    /// Stable identifier reported back on interaction.
    pub id: String,
    /// Glyph in front of the label (host icon font).
    pub glyph: String,
    /// Chip text, e.g. `draft.md` or `Marked region (312 chars)`.
    pub label: String,
    /// Hover hint of the label.
    pub hint: String,
    /// Whether the chip can be interacted with at all.
    pub enabled: bool,
    /// What the chip offers.
    pub kind: AttachmentKind,
}

impl Attachment {
    /// A toggle chip — the document buffer, the bibliography: context that
    /// rides along while it is switched on.
    pub fn toggle(
        id: impl Into<String>,
        glyph: impl Into<String>,
        label: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            glyph: glyph.into(),
            label: label.into(),
            hint: String::new(),
            enabled: true,
            kind: AttachmentKind::Toggle { on: false },
        }
    }

    /// A pending context that waits for the next question until it is dropped.
    pub fn pending(
        id: impl Into<String>,
        glyph: impl Into<String>,
        label: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            glyph: glyph.into(),
            label: label.into(),
            hint: String::new(),
            enabled: true,
            kind: AttachmentKind::Pending {
                drop_glyph: DEFAULT_DROP_GLYPH.to_string(),
                drop_hint: DEFAULT_DROP_HINT.to_string(),
            },
        }
    }

    /// A ready artefact with an action button (`action`, e.g. `Insert`) and a
    /// drop button.
    pub fn offer(
        id: impl Into<String>,
        glyph: impl Into<String>,
        label: impl Into<String>,
        action: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            glyph: glyph.into(),
            label: label.into(),
            hint: String::new(),
            enabled: true,
            kind: AttachmentKind::Offer {
                action: action.into(),
                action_hint: String::new(),
                drop_glyph: DEFAULT_DROP_GLYPH.to_string(),
                drop_hint: DEFAULT_DROP_HINT.to_string(),
            },
        }
    }

    /// Set the toggle state (meaningful for [`Attachment::toggle`]).
    pub fn on(mut self, on: bool) -> Self {
        if let AttachmentKind::Toggle { on: state } = &mut self.kind {
            *state = on;
        }
        self
    }

    /// Set the hover hint of the label.
    pub fn hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = hint.into();
        self
    }

    /// Set the action button's tooltip (meaningful for [`Attachment::offer`]).
    pub fn action_hint(mut self, hint: impl Into<String>) -> Self {
        if let AttachmentKind::Offer { action_hint, .. } = &mut self.kind {
            *action_hint = hint.into();
        }
        self
    }

    /// Enable or disable the whole chip; a disabled chip can only be hovered.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Override the drop button's glyph and tooltip (pending and offer chips).
    pub fn drop(mut self, glyph: impl Into<String>, hint: impl Into<String>) -> Self {
        let glyph = glyph.into();
        let hint = hint.into();
        match &mut self.kind {
            AttachmentKind::Toggle { .. } => {}
            AttachmentKind::Pending {
                drop_glyph,
                drop_hint,
            }
            | AttachmentKind::Offer {
                drop_glyph,
                drop_hint,
                ..
            } => {
                *drop_glyph = glyph;
                *drop_hint = hint;
            }
        }
        self
    }
}

/// Fallback drop glyph; hosts with an icon font override it via
/// [`Attachment::drop`].
const DEFAULT_DROP_GLYPH: &str = "\u{2715}";
/// Fallback drop tooltip.
const DEFAULT_DROP_HINT: &str = "Drop";

/// What the user did with a chip this frame.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AttachmentEvent {
    /// A toggle was clicked — the host flips its state (the chip rendered the
    /// state of the previous frame).
    Toggled(String),
    /// The chip's ✕ was clicked.
    Dropped(String),
    /// An offer's action button was clicked.
    Accepted(String),
}

/// Render the chips grouped into **rows**: the outer slice is top to bottom,
/// each inner slice is one row, left to right (the document toggle and the
/// library toggle share a row).
///
/// Returns the interaction of this frame — a single pointer cannot work two
/// chips at once. Empty rows render nothing; a row does not wrap (a very long
/// label runs into the pane's edge, it does not push chips onto a new line).
pub fn show(ui: &mut Ui, rows: &[&[Attachment]]) -> Option<AttachmentEvent> {
    let mut event = None;
    for row in rows {
        if row.is_empty() {
            continue;
        }
        ui.horizontal(|ui| {
            for chip in *row {
                // Each chip gets its own id scope, so a chip appearing or
                // disappearing (the buffer chip without a document) cannot
                // renumber the others.
                ui.push_id(chip.id.as_str(), |ui| {
                    if let Some(interaction) = show_chip(ui, chip) {
                        event = Some(interaction);
                    }
                });
            }
        });
    }
    event
}

/// Draw one chip — a control of the current row — and return what the user
/// did with it, if anything.
fn show_chip(ui: &mut Ui, chip: &Attachment) -> Option<AttachmentEvent> {
    let mut event = None;
    let text = format!("{} {}", chip.glyph, chip.label);
    match &chip.kind {
        AttachmentKind::Toggle { on } => {
            let response = ui
                .add_enabled(chip.enabled, egui::Button::new(text).selected(*on))
                .hover_text_if(chip.enabled, &chip.hint);
            if response.clicked() {
                event = Some(AttachmentEvent::Toggled(chip.id.clone()));
            }
        }
        AttachmentKind::Pending {
            drop_glyph,
            drop_hint,
        } => {
            label(ui, text, chip).hover_text_if(chip.enabled, &chip.hint);
            if ui
                .add_enabled(chip.enabled, egui::Button::new(drop_glyph).small())
                .hover_text_if(chip.enabled, drop_hint)
                .clicked()
            {
                event = Some(AttachmentEvent::Dropped(chip.id.clone()));
            }
        }
        AttachmentKind::Offer {
            action,
            action_hint,
            drop_glyph,
            drop_hint,
        } => {
            label(ui, text, chip).hover_text_if(chip.enabled, &chip.hint);
            if ui
                .add_enabled(chip.enabled, egui::Button::new(action).small())
                .hover_text_if(chip.enabled, action_hint)
                .clicked()
            {
                event = Some(AttachmentEvent::Accepted(chip.id.clone()));
            }
            if ui
                .add_enabled(chip.enabled, egui::Button::new(drop_glyph).small())
                .hover_text_if(chip.enabled, drop_hint)
                .clicked()
            {
                event = Some(AttachmentEvent::Dropped(chip.id.clone()));
            }
        }
    }
    event
}

/// The chip text: not selectable — a chip is a control, not copyable content.
fn label(ui: &mut Ui, text: String, chip: &Attachment) -> egui::Response {
    ui.add_enabled(chip.enabled, egui::Label::new(text).selectable(false))
}

/// Hover text on live widgets, a dimmed explanation on disabled ones — a dead
/// control still says why it is dead.
trait HoverText {
    /// Attach `text` as hover text, choosing the disabled variant when needed.
    fn hover_text_if(self, enabled: bool, text: &str) -> Self;
}

impl HoverText for egui::Response {
    fn hover_text_if(self, enabled: bool, text: &str) -> Self {
        if text.is_empty() {
            return self;
        }
        if enabled {
            self.on_hover_text(text)
        } else {
            self.on_disabled_hover_text(text)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::leaf_widgets;

    /// Render `rows` and click the `index`-th control (top-left to
    /// bottom-right).
    fn click(ctx: &egui::Context, rows: &[&[Attachment]], index: usize) -> Option<AttachmentEvent> {
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(320.0, 200.0));
        let frame = |events: Vec<egui::Event>| {
            let input = egui::RawInput {
                screen_rect: Some(screen),
                events,
                ..egui::RawInput::default()
            };
            let mut event = None;
            let _ = ctx.run_ui(input, |ui| {
                event = show(ui, rows);
            });
            event
        };
        // Frame 1 settles the layout.
        assert!(frame(vec![]).is_none(), "no input, no interaction");
        let target = leaf_widgets(ctx)[index].center();
        let press = |pressed| egui::Event::PointerButton {
            pos: target,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::default(),
        };
        frame(vec![
            egui::Event::PointerMoved(target),
            press(true),
            press(false),
        ])
    }

    #[test]
    fn toggles_share_a_row_and_each_reports_its_id() {
        // The document and library toggles sit next to each other; a pending
        // context gets its own row below.
        let ctx = egui::Context::default();
        let document = Attachment::toggle("buffer", "P", "draft.md").on(true);
        let library = Attachment::toggle("library", "L", "Library");
        let selection = Attachment::pending("selection", "S", "Marked region (312 chars)");
        let rows: [&[Attachment]; 2] = [&[document, library], &[selection]];

        assert_eq!(
            click(&ctx, &rows, 0),
            Some(AttachmentEvent::Toggled("buffer".into()))
        );
        assert_eq!(
            click(&ctx, &rows, 1),
            Some(AttachmentEvent::Toggled("library".into())),
            "the second chip of the row is its own control"
        );
        // Row two: the label, then the ✕.
        assert_eq!(
            click(&ctx, &rows, 3),
            Some(AttachmentEvent::Dropped("selection".into()))
        );
    }

    #[test]
    fn a_pending_chip_drops() {
        let ctx = egui::Context::default();
        let pending = Attachment::pending("selection", "S", "Marked region (312 chars)")
            .drop("X", "Drop the marked region");
        let rows: [&[Attachment]; 1] = [&[pending]];
        // The second control of the row is the ✕ (after the label).
        assert_eq!(
            click(&ctx, &rows, 1),
            Some(AttachmentEvent::Dropped("selection".into()))
        );
    }

    #[test]
    fn an_offer_chip_accepts_and_drops() {
        let offer = Attachment::offer("insert", "E", "Replacement ready (508 chars)", "Insert");
        let rows: [&[Attachment]; 1] = [&[offer]];
        assert_eq!(
            click(&egui::Context::default(), &rows, 1),
            Some(AttachmentEvent::Accepted("insert".into()))
        );
        assert_eq!(
            click(&egui::Context::default(), &rows, 2),
            Some(AttachmentEvent::Dropped("insert".into()))
        );
    }

    #[test]
    fn a_disabled_chip_does_not_fire() {
        let ctx = egui::Context::default();
        let chip = Attachment::toggle("buffer", "P", "untitled")
            .enabled(false)
            .hint("This document has no text to attach");
        let rows: [&[Attachment]; 1] = [&[chip]];
        assert_eq!(click(&ctx, &rows, 0), None);
    }

    #[test]
    fn chips_render_without_panic() {
        let ctx = egui::Context::default();
        let document = Attachment::toggle("buffer", "P", "draft.md");
        let library = Attachment::toggle("library", "L", "Library");
        let selection = Attachment::pending("selection", "S", "Marked region (312 chars)");
        let offer = Attachment::offer("insert", "E", "Replacement ready (508 chars)", "Insert");
        let rows: [&[Attachment]; 3] = [&[document, library], &[selection, offer], &[]];
        let _ = ctx.run_ui(egui::RawInput::default(), |ui| {
            assert!(show(ui, &rows).is_none());
        });
        let none: [&[Attachment]; 0] = [];
        let _ = ctx.run_ui(egui::RawInput::default(), |ui| {
            assert!(show(ui, &none).is_none());
        });
    }
}
