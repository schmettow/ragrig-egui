//! A dockable side panel hosting stacked, collapsible sections.
//!
//! [`SectionDock`] sits on the left or right edge of the window and divides
//! its height between the sections the host registers. Each section has a
//! clickable header (collapse/expand) and an optional close button; the
//! section bodies are host-drawn closures.
//!
//! The dock intentionally ships with **one layout mode** (stacked sections).
//! A tabbed mode — several panes sharing the same dock well — is a planned
//! extension once more than two consumers exist; the API presents sections
//! as independent units so that change stays additive.

use egui::{Id, Ui};

/// Which window edge a [`SectionDock`] occupies.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DockSide {
    /// Left edge; the panel grows to the right.
    Left,
    /// Right edge; the panel grows to the left.
    Right,
}

/// Per-section state owned by the host, so it can persist it.
#[derive(Clone, Debug, PartialEq)]
pub struct SectionState {
    /// Section title shown in the header.
    pub title: String,
    /// Whether the section body is hidden (only the header shows).
    pub collapsed: bool,
}

impl SectionState {
    /// A new, expanded section with the given title.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            collapsed: false,
        }
    }
}

impl Default for SectionState {
    fn default() -> Self {
        Self::new("Section")
    }
}

/// What the dock observed while rendering this frame.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct DockResponse {
    /// New panel width after the user resized it; the host persists it.
    /// `None` when the width did not change this frame.
    pub width: Option<f32>,
    /// The user pressed the dock's close button.
    pub close_requested: bool,
}

/// A side panel of stacked, collapsible sections.
///
/// ```no_run
/// # use egui_chat::dock::{DockSide, SectionDock, SectionState};
/// # fn ui(ui: &mut egui::Ui, agent: &mut SectionState, library: &mut SectionState, width: &mut f32) {
/// let response = SectionDock::new("my_dock", DockSide::Right, *width)
///     .section(agent, |ui| { ui.label("transcript + composer"); })
///     .section(library, |ui| { ui.label("source list"); })
///     .show(ui);
/// if let Some(new_width) = response.width {
///     *width = new_width;
/// }
/// # }
/// ```
pub struct SectionDock<'a> {
    id: Id,
    side: DockSide,
    width: f32,
    min_width: f32,
    max_width: f32,
    resizable: bool,
    closable: bool,
    sections: Vec<SectionEntry<'a>>,
}

struct SectionEntry<'a> {
    state: &'a mut SectionState,
    body: Box<dyn FnOnce(&mut Ui) + 'a>,
}

impl<'a> SectionDock<'a> {
    /// A right- or left-docked panel with the given persistence id and
    /// initial width.
    pub fn new(id_salt: impl std::hash::Hash, side: DockSide, width: f32) -> Self {
        Self {
            id: Id::new(("egui_chat::dock", id_salt)),
            side,
            width,
            min_width: 200.0,
            max_width: 640.0,
            resizable: true,
            closable: false,
            sections: Vec::new(),
        }
    }

    /// Bounds for user resizing (defaults: 200–640 px).
    pub fn width_range(mut self, min: f32, max: f32) -> Self {
        self.min_width = min;
        self.max_width = max;
        self
    }

    /// Whether the user may drag the panel edge (default `true`).
    pub fn resizable(mut self, resizable: bool) -> Self {
        self.resizable = resizable;
        self
    }

    /// Show a close button in a header row (default: no close button).
    pub fn closable(mut self, closable: bool) -> Self {
        self.closable = closable;
        self
    }

    /// Register a section. Sections render top to bottom in call order.
    pub fn section(mut self, state: &'a mut SectionState, body: impl FnOnce(&mut Ui) + 'a) -> Self {
        self.sections.push(SectionEntry {
            state,
            body: Box::new(body),
        });
        self
    }

    /// Render the dock inside `ui`.
    pub fn show(self, ui: &mut Ui) -> DockResponse {
        let mut response = DockResponse::default();
        let width_before = self.width;
        let side = self.side;
        let resizable = self.resizable;
        let closable = self.closable;
        let mut sections = self.sections;

        let panel = match side {
            DockSide::Left => egui::Panel::left(self.id),
            DockSide::Right => egui::Panel::right(self.id),
        };

        let inner = panel
            .default_size(self.width)
            .min_size(self.min_width)
            .max_size(self.max_width)
            .resizable(resizable)
            .show_inside(ui, |ui| {
                // Panels store the rendered *content* min-rect as their width;
                // pin the content to the panel width so a narrow section
                // cannot shrink the dock on the next frame.
                ui.set_min_width(ui.available_width());
                for entry in sections.drain(..) {
                    if header(ui, entry.state, closable) {
                        response.close_requested = true;
                    }
                    if !entry.state.collapsed {
                        (entry.body)(ui);
                    }
                }
            });

        let width_after = inner.response.rect.width();
        if (width_after - width_before).abs() > 0.5 {
            response.width = Some(width_after);
        }
        response
    }
}

/// Draw a section header; returns `true` when its close button was clicked.
fn header(ui: &mut Ui, state: &mut SectionState, closable: bool) -> bool {
    let mut close_clicked = false;
    ui.horizontal(|ui| {
        let arrow = if state.collapsed {
            '\u{25B6}'
        } else {
            '\u{25BC}'
        };
        let title = format!("{arrow}  {}", state.title);
        if ui
            .add(egui::Button::new(egui::RichText::new(title).strong()).frame(false))
            .clicked()
        {
            state.collapsed = !state.collapsed;
        }
        if closable {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .add(egui::Button::new("\u{2715}").frame(false))
                    .on_hover_text("Close panel")
                    .clicked()
                {
                    close_clicked = true;
                }
            });
        }
    });
    close_clicked
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn section_state_starts_expanded() {
        let state = SectionState::new("Agent");
        assert_eq!(state.title, "Agent");
        assert!(!state.collapsed);
    }

    #[test]
    fn dock_renders_with_sections_and_returns_no_change() {
        let ctx = egui::Context::default();
        let mut state = SectionState::new("Agent");
        let _ = ctx.run_ui(egui::RawInput::default(), |ui| {
            let response = SectionDock::new("test_dock", DockSide::Right, 320.0)
                .section(&mut state, |ui| {
                    ui.label("body");
                })
                .show(ui);
            assert_eq!(response, DockResponse::default());
        });
    }

    #[test]
    fn collapsed_section_skips_its_body() {
        let ctx = egui::Context::default();
        let mut state = SectionState::new("Agent");
        state.collapsed = true;
        let mut body_ran = false;
        let _ = ctx.run_ui(egui::RawInput::default(), |ui| {
            let _ = SectionDock::new("collapsed_test", DockSide::Right, 320.0)
                .section(&mut state, |ui| {
                    body_ran = true;
                    ui.label("body");
                })
                .show(ui);
        });
        assert!(!body_ran, "collapsed section must not run its body");
    }
}
