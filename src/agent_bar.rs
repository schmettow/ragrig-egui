//! Component status bar for agent panels.
//!
//! [`AgentBar`] renders one icon per agent component (chat model, embedder,
//! vector store, parser, …) along a single row. Each icon carries a colored
//! status dot — grey *inactive*, green *ready*, pulsing *busy*, red *error* —
//! and a hover tooltip with the host-supplied detail line. Clicking an icon
//! opens a **slide-up menu** anchored above the bar whose content is drawn
//! by the host (activate/deactivate, update, current settings, …).
//!
//! ```no_run
//! # use egui_chat::agent_bar::{AgentBar, Component, ComponentStatus};
//! # fn ui(ui: &mut egui::Ui, open: &mut Option<String>) {
//! let components = [
//!     Component::new("chat", "Chat model", "\u{1F4AC}").status(ComponentStatus::Ready),
//!     Component::new("embed", "Embedder", "E").detail("nomic-embed-text · top_k 300"),
//! ];
//! AgentBar::new(&components, open).show(ui, |ui, id| {
//!     ui.label(format!("settings for {id}"));
//!     false // return true to close the menu after an action
//! });
//! # }
//! ```

use egui::{Color32, Response, RichText, Ui, Vec2};

/// Status of one agent component, driving its dot color.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ComponentStatus {
    /// Not configured or switched off (grey).
    Inactive,
    /// Configured and available (green).
    Ready,
    /// Currently working (pulsing).
    Busy,
    /// Failed; see the component's detail line (red).
    Error,
}

impl ComponentStatus {
    /// Short human-readable label used in tooltips.
    pub fn label(self) -> &'static str {
        match self {
            Self::Inactive => "inactive",
            Self::Ready => "ready",
            Self::Busy => "busy",
            Self::Error => "error",
        }
    }
}

/// One component shown in the bar.
#[derive(Clone, Debug, PartialEq)]
pub struct Component<'a> {
    /// Stable identifier, reported back when the menu opens.
    pub id: &'a str,
    /// Human-readable name (tooltip title).
    pub label: &'a str,
    /// Icon glyph rendered as the button face (any short string).
    pub glyph: &'a str,
    /// Current status.
    pub status: ComponentStatus,
    /// Optional detail line, e.g. backend, model, or last error.
    pub detail: Option<String>,
}

impl<'a> Component<'a> {
    /// A new, inactive component.
    pub fn new(id: &'a str, label: &'a str, glyph: &'a str) -> Self {
        Self {
            id,
            label,
            glyph,
            status: ComponentStatus::Inactive,
            detail: None,
        }
    }

    /// Set the status.
    pub fn status(mut self, status: ComponentStatus) -> Self {
        self.status = status;
        self
    }

    /// Set the detail line.
    pub fn detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }
}

/// Colors and sizing of the bar.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AgentBarStyle {
    /// Height of the icon buttons in points.
    pub height: f32,
    /// Dot color for [`ComponentStatus::Inactive`].
    pub inactive: Color32,
    /// Dot color for [`ComponentStatus::Ready`].
    pub ready: Color32,
    /// Dot color for [`ComponentStatus::Busy`] (pulses when rendered).
    pub busy: Color32,
    /// Dot color for [`ComponentStatus::Error`].
    pub error: Color32,
}

impl Default for AgentBarStyle {
    fn default() -> Self {
        Self {
            height: 24.0,
            inactive: Color32::from_gray(130),
            ready: Color32::from_rgb(76, 175, 80),
            busy: Color32::from_rgb(66, 165, 245),
            error: Color32::from_rgb(229, 57, 53),
        }
    }
}

/// What the user did in the bar this frame.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AgentBarOutput {
    /// Component whose menu was toggled this frame (opened or closed).
    pub toggled: Option<String>,
}

/// The component status bar. State (`open`) is owned by the host.
pub struct AgentBar<'a> {
    components: &'a [Component<'a>],
    open: &'a mut Option<String>,
    style: AgentBarStyle,
}

impl<'a> AgentBar<'a> {
    /// A bar over `components`; `open` remembers which component's menu is
    /// currently shown (host-owned so it can be reset externally).
    pub fn new(components: &'a [Component<'a>], open: &'a mut Option<String>) -> Self {
        Self {
            components,
            open,
            style: AgentBarStyle::default(),
        }
    }

    /// Override colors/size.
    pub fn style(mut self, style: AgentBarStyle) -> Self {
        self.style = style;
        self
    }

    /// Render the bar. `menu` draws the slide-up menu for the open
    /// component; return `true` from it to close the menu (e.g. after an
    /// action was chosen).
    pub fn show(self, ui: &mut Ui, mut menu: impl FnMut(&mut Ui, &str) -> bool) -> AgentBarOutput {
        let mut output = AgentBarOutput::default();
        ui.separator();
        let mut active: Option<Response> = None;
        let size = Vec2::splat(self.style.height);

        ui.horizontal(|ui| {
            for component in self.components {
                let button = ui.add(
                    egui::Button::new(
                        RichText::new(component.glyph).size(self.style.height * 0.58),
                    )
                    .frame(false)
                    .min_size(size),
                );
                let response = button.on_hover_ui(|ui| {
                    ui.strong(component.label);
                    if let Some(detail) = &component.detail {
                        ui.label(detail.clone());
                    }
                    ui.weak(component.status.label());
                });

                let dot = egui::pos2(response.rect.right() - 5.0, response.rect.bottom() - 5.0);
                ui.painter().circle_filled(
                    dot,
                    3.0,
                    status_color(&self.style, component.status, ui),
                );
                if component.status == ComponentStatus::Busy {
                    ui.ctx().request_repaint();
                }

                if response.clicked() {
                    toggle(self.open, component.id);
                    output.toggled = Some(component.id.to_string());
                }
                if self.open.as_deref() == Some(component.id) {
                    active = Some(response);
                }
            }
        });

        if let Some(response) = active {
            let open_id = self.open.clone().unwrap_or_default();
            let mut is_open = true;
            let mut close_requested = false;
            egui::Popup::from_response(&response)
                .id(ui.make_persistent_id(("egui_chat::agent_bar", open_id.as_str())))
                .open_bool(&mut is_open)
                .align(egui::RectAlign::TOP_START)
                .gap(6.0)
                .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
                .show(|ui| {
                    ui.set_min_width(240.0);
                    close_requested = menu(ui, open_id.as_str());
                });
            if !is_open || close_requested {
                *self.open = None;
            }
        }

        output
    }
}

/// Toggle `open` to `id`, or to `None` when `id` was already open.
fn toggle(open: &mut Option<String>, id: &str) {
    if open.as_deref() == Some(id) {
        *open = None;
    } else {
        *open = Some(id.to_string());
    }
}

/// Dot color for a status; busy pulses between 55 % and 100 % opacity.
fn status_color(style: &AgentBarStyle, status: ComponentStatus, ui: &Ui) -> Color32 {
    match status {
        ComponentStatus::Inactive => style.inactive,
        ComponentStatus::Ready => style.ready,
        ComponentStatus::Busy => {
            let time = ui.input(|i| i.time) as f32;
            let pulse = 0.55 + 0.45 * (time * 4.0).sin().abs();
            style.busy.gamma_multiply(pulse)
        }
        ComponentStatus::Error => style.error,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toggle_opens_switches_and_closes() {
        let mut open = None;
        toggle(&mut open, "chat");
        assert_eq!(open.as_deref(), Some("chat"));
        toggle(&mut open, "embed");
        assert_eq!(open.as_deref(), Some("embed"));
        toggle(&mut open, "embed");
        assert_eq!(open, None);
    }

    #[test]
    fn status_colors_map_to_style() {
        let ctx = egui::Context::default();
        let style = AgentBarStyle::default();
        let _ = ctx.run_ui(egui::RawInput::default(), |ui| {
            assert_eq!(
                status_color(&style, ComponentStatus::Inactive, ui),
                style.inactive
            );
            assert_eq!(
                status_color(&style, ComponentStatus::Ready, ui),
                style.ready
            );
            assert_eq!(
                status_color(&style, ComponentStatus::Error, ui),
                style.error
            );
        });
    }

    #[test]
    fn bar_renders_without_panic_and_reports_no_interaction() {
        let ctx = egui::Context::default();
        let components = [
            Component::new("chat", "Chat model", "C")
                .status(ComponentStatus::Ready)
                .detail("deepseek / deepseek-v4-pro"),
            Component::new("embed", "Embedder", "E"),
        ];
        let mut open = None;
        let _ = ctx.run_ui(egui::RawInput::default(), |ui| {
            let output = AgentBar::new(&components, &mut open).show(ui, |_, _| false);
            assert_eq!(output, AgentBarOutput::default());
        });
        assert_eq!(open, None);
    }
}
