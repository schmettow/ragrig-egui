//! A dockable side panel hosting stacked sections.
//!
//! [`SectionDock`] sits on the left or right edge of the window and divides
//! its height between the sections the host registers. Each section has a
//! titled header and an optional close button; the section bodies are
//! host-drawn closures.
//!
//! The **whole dock collapses horizontally**: it shrinks to a narrow strip at
//! its window edge that holds one control to expand it again. (0.4 — the
//! earlier per-section fold, which stacked the headers and hid only the
//! bodies, was a relict of hosting several panes inside one dock.) The host
//! owns the collapsed flag and persists it; the dock reports the user's
//! toggle in [`DockResponse::collapsed`].
//!
//! The dock intentionally ships with **one layout mode** (stacked sections).
//! A tabbed mode — several panes sharing the same dock well — is a planned
//! extension once more than two consumers exist; the API presents sections
//! as independent units so that change stays additive.

use egui::{Id, Ui};

/// Width of a collapsed dock: the strip that holds the expand control.
pub const COLLAPSED_WIDTH: f32 = 28.0;

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
}

impl SectionState {
    /// A new section with the given title.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
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
    /// `None` when the width did not change this frame — and while collapsed,
    /// where the strip is not the panel's width.
    pub width: Option<f32>,
    /// The user pressed the dock's close button.
    pub close_requested: bool,
    /// The user collapsed or expanded the dock; the host persists the value.
    /// `None` when the control was not used this frame.
    pub collapsed: Option<bool>,
}

/// A side panel of stacked sections.
///
/// ```no_run
/// # use egui_chat::dock::{DockSide, SectionDock, SectionState};
/// # fn ui(ui: &mut egui::Ui, agent: &mut SectionState, library: &mut SectionState, width: &mut f32, collapsed: &mut bool) {
/// let response = SectionDock::new("my_dock", DockSide::Right, *width)
///     .collapsed(*collapsed)
///     .section(agent, |ui| { ui.label("transcript + composer"); })
///     .section(library, |ui| { ui.label("source list"); })
///     .show(ui);
/// if let Some(new_width) = response.width {
///     *width = new_width;
/// }
/// if let Some(new_state) = response.collapsed {
///     *collapsed = new_state;
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
    collapsed: bool,
    collapse_hint: String,
    expand_hint: String,
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
            collapsed: false,
            collapse_hint: "Collapse panel".to_string(),
            expand_hint: "Expand panel".to_string(),
            sections: Vec::new(),
        }
    }

    /// Bounds for user resizing (defaults: 200–640 px).
    pub fn width_range(mut self, min: f32, max: f32) -> Self {
        self.min_width = min;
        self.max_width = max;
        self
    }

    /// Whether the user may drag the panel edge (default `true`). A collapsed
    /// dock never resizes — it is a strip.
    pub fn resizable(mut self, resizable: bool) -> Self {
        self.resizable = resizable;
        self
    }

    /// Show a close button in a header row (default: no close button; a
    /// collapsed dock shows it on the strip).
    pub fn closable(mut self, closable: bool) -> Self {
        self.closable = closable;
        self
    }

    /// Whether the dock is collapsed to its strip (host-owned; the user's
    /// toggle comes back in [`DockResponse::collapsed`]).
    pub fn collapsed(mut self, collapsed: bool) -> Self {
        self.collapsed = collapsed;
        self
    }

    /// Tooltip of the collapse control in the first header (default
    /// "Collapse panel"), so hosts can localize it.
    pub fn collapse_hint(mut self, hint: impl Into<String>) -> Self {
        self.collapse_hint = hint.into();
        self
    }

    /// Tooltip of the expand control on the strip (default "Expand panel").
    pub fn expand_hint(mut self, hint: impl Into<String>) -> Self {
        self.expand_hint = hint.into();
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
        let collapsed = self.collapsed;
        let closable = self.closable;
        let mut sections = self.sections;

        let panel = match side {
            DockSide::Left => egui::Panel::left(self.id),
            DockSide::Right => egui::Panel::right(self.id),
        };
        // The strip needs a tighter frame than a full pane: the default side
        // panel margin would leave no room for the control.
        let panel = if collapsed {
            panel.frame(strip_frame(ui.style()))
        } else {
            panel
        };

        let (size, min, max) = if collapsed {
            (COLLAPSED_WIDTH, COLLAPSED_WIDTH, COLLAPSED_WIDTH)
        } else {
            (self.width, self.min_width, self.max_width)
        };
        let collapse_hint = self.collapse_hint;
        let expand_hint = self.expand_hint;
        let inner = panel
            .default_size(size)
            .min_size(min)
            .max_size(max)
            .resizable(self.resizable && !collapsed)
            .show_inside(ui, |ui| {
                // Panels store the rendered *content* min-rect as their width;
                // pin the content to the panel width so a narrow section
                // cannot shrink the dock on the next frame.
                ui.set_min_width(ui.available_width());

                if collapsed {
                    let control = HeaderControl {
                        arrow: collapse_arrow(side, true),
                        hint: &expand_hint,
                    };
                    let outcome = strip(ui, &control, closable);
                    response.close_requested |= outcome.close;
                    if outcome.collapse {
                        response.collapsed = Some(false);
                    }
                    return;
                }

                let mut entries = sections.drain(..);
                // The first header carries the dock's collapse control.
                if let Some(entry) = entries.next() {
                    let control = HeaderControl {
                        arrow: collapse_arrow(side, false),
                        hint: &collapse_hint,
                    };
                    let outcome = header(ui, entry.state, closable, Some(&control));
                    response.close_requested |= outcome.close;
                    if outcome.collapse {
                        response.collapsed = Some(true);
                    }
                    (entry.body)(ui);
                }
                for entry in entries {
                    let outcome = header(ui, entry.state, closable, None);
                    response.close_requested |= outcome.close;
                    (entry.body)(ui);
                }
            });

        // Only an expanded dock has a width to report: the strip must not
        // overwrite the host's remembered width.
        if !collapsed {
            let width_after = inner.response.rect.width();
            if (width_after - width_before).abs() > 0.5 {
                response.width = Some(width_after);
            }
        }
        response
    }
}

/// The frame of a collapsed dock's strip: just enough margin for the
/// control to fit the narrow panel.
fn strip_frame(style: &egui::Style) -> egui::Frame {
    egui::Frame::side_top_panel(style).inner_margin(egui::Margin::symmetric(2, 6))
}

/// What a header's (or the strip's) controls did this frame.
#[derive(Default)]
struct HeaderOutcome {
    /// The close button was pressed.
    close: bool,
    /// The collapse/expand control was pressed.
    collapse: bool,
}

/// The dock-level collapse control, drawn in the first section's header (and
/// on the strip while collapsed).
struct HeaderControl<'a> {
    arrow: &'a str,
    hint: &'a str,
}

/// The arrow of the collapse control: it points the way the pane will move —
/// toward its window edge while expanded, back into the window while
/// collapsed.
fn collapse_arrow(side: DockSide, collapsed: bool) -> &'static str {
    match (side, collapsed) {
        (DockSide::Right, false) | (DockSide::Left, true) => "\u{25B6}", // ▶
        (DockSide::Left, false) | (DockSide::Right, true) => "\u{25C0}", // ◀
    }
}

/// Draw a section header; `control` marks the header that carries the dock's
/// collapse control (the first one).
fn header(
    ui: &mut Ui,
    state: &mut SectionState,
    closable: bool,
    control: Option<&HeaderControl<'_>>,
) -> HeaderOutcome {
    let mut outcome = HeaderOutcome::default();
    ui.horizontal(|ui| {
        if let Some(control) = control
            && ui
                .add(egui::Button::new(control.arrow).frame(false))
                .on_hover_text(control.hint)
                .clicked()
        {
            outcome.collapse = true;
        }
        ui.add(egui::Label::new(egui::RichText::new(&state.title).strong()));
        if closable {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .add(egui::Button::new("\u{2715}").frame(false))
                    .on_hover_text("Close panel")
                    .clicked()
                {
                    outcome.close = true;
                }
            });
        }
    });
    outcome
}

/// The collapsed dock: a narrow strip whose only control expands it again.
fn strip(ui: &mut Ui, control: &HeaderControl<'_>, closable: bool) -> HeaderOutcome {
    let mut outcome = HeaderOutcome::default();
    ui.vertical_centered(|ui| {
        if ui
            .add(egui::Button::new(control.arrow).frame(false))
            .on_hover_text(control.hint)
            .clicked()
        {
            outcome.collapse = true;
        }
        if closable
            && ui
                .add(egui::Button::new("\u{2715}").frame(false))
                .on_hover_text("Close panel")
                .clicked()
        {
            outcome.close = true;
        }
    });
    outcome
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{Pos2, Rect, Vec2};

    /// The screen the tests click in — small enough to keep the arithmetic
    /// obvious.
    fn screen() -> Rect {
        Rect::from_min_size(Pos2::ZERO, Vec2::new(900.0, 600.0))
    }

    fn input(events: Vec<egui::Event>) -> egui::RawInput {
        egui::RawInput {
            screen_rect: Some(screen()),
            events,
            ..egui::RawInput::default()
        }
    }

    /// A press-and-release at `pos` (the two events egui turns into a click).
    fn click(pos: Pos2) -> Vec<egui::Event> {
        let press = |pressed| egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::default(),
        };
        vec![press(true), press(false)]
    }

    /// Run the same dock once more and return what it reported.
    fn second_frame(
        ctx: &egui::Context,
        state: &mut SectionState,
        events: Vec<egui::Event>,
    ) -> DockResponse {
        let mut response = DockResponse::default();
        let _ = ctx.run_ui(input(events), |ui| {
            response = SectionDock::new("click_dock", DockSide::Right, 320.0)
                .section(state, |ui| {
                    ui.label("body");
                })
                .show(ui);
        });
        response
    }

    #[test]
    fn section_state_keeps_its_title() {
        let state = SectionState::new("Agent");
        assert_eq!(state.title, "Agent");
    }

    #[test]
    fn the_collapse_arrow_points_at_the_window_edge() {
        // Expanded, the arrow points the way the pane will move (toward its
        // edge); collapsed, it points back into the window.
        assert_eq!(collapse_arrow(DockSide::Right, false), "\u{25B6}");
        assert_eq!(collapse_arrow(DockSide::Right, true), "\u{25C0}");
        assert_eq!(collapse_arrow(DockSide::Left, false), "\u{25C0}");
        assert_eq!(collapse_arrow(DockSide::Left, true), "\u{25B6}");
    }

    #[test]
    fn dock_renders_with_sections_and_returns_no_change() {
        let ctx = egui::Context::default();
        let mut state = SectionState::new("Agent");
        let _ = ctx.run_ui(input(Vec::new()), |ui| {
            let response = SectionDock::new("test_dock", DockSide::Right, 320.0)
                .section(&mut state, |ui| {
                    ui.label("body");
                })
                .show(ui);
            assert_eq!(response, DockResponse::default());
        });
    }

    #[test]
    fn a_collapsed_dock_shows_the_strip_and_skips_the_bodies() {
        let ctx = egui::Context::default();
        let mut state = SectionState::new("Agent");
        let mut body_ran = false;
        let mut response = DockResponse::default();
        let _ = ctx.run_ui(input(Vec::new()), |ui| {
            response = SectionDock::new("collapsed_test", DockSide::Right, 320.0)
                .collapsed(true)
                .section(&mut state, |ui| {
                    body_ran = true;
                    ui.label("body");
                })
                .show(ui);
        });
        assert!(
            !body_ran,
            "a collapsed dock must not run its section bodies"
        );
        assert_eq!(response.width, None, "the strip is not the panel's width");
        assert_eq!(response.collapsed, None, "no click, no toggle");
    }

    #[test]
    fn the_header_control_collapses_the_dock() {
        // The chevron sits right after the panel's inner margin, at the left
        // of the first header row.
        let ctx = egui::Context::default();
        let margin = egui::Frame::side_top_panel(&ctx.global_style()).inner_margin;
        let target = Pos2::new(
            screen().right() - 320.0 + f32::from(margin.left) + 9.0,
            screen().top() + f32::from(margin.top) + 9.0,
        );

        // The first frame registers the widgets; a click lands on the next.
        let _ = second_frame(&ctx, &mut SectionState::new("Agent"), Vec::new());
        let response = second_frame(&ctx, &mut SectionState::new("Agent"), click(target));
        assert_eq!(
            response.collapsed,
            Some(true),
            "the chevron collapses the dock"
        );
    }

    #[test]
    fn the_strip_control_expands_the_dock() {
        let ctx = egui::Context::default();
        let margin = strip_frame(&ctx.global_style()).inner_margin;
        let target = Pos2::new(
            screen().right() - COLLAPSED_WIDTH / 2.0,
            screen().top() + f32::from(margin.top) + 9.0,
        );

        let collapsed_frame = |events: Vec<egui::Event>| {
            let mut response = DockResponse::default();
            let mut state = SectionState::new("Agent");
            let _ = ctx.run_ui(input(events), |ui| {
                response = SectionDock::new("strip_click", DockSide::Right, 320.0)
                    .collapsed(true)
                    .section(&mut state, |ui| {
                        ui.label("body");
                    })
                    .show(ui);
            });
            response
        };
        let _ = collapsed_frame(Vec::new());
        let response = collapsed_frame(click(target));
        assert_eq!(
            response.collapsed,
            Some(false),
            "the strip expands the dock"
        );
    }
}
