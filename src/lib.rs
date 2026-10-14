//! The egui agent interface for [ragrig] — transcript, composer, component bar
//! and the panes that host them.
//!
//! `ragrig_egui` provides the UI building blocks for the *agent side* of a RAG
//! workspace: what an agent says, what it is doing, which components are up,
//! and where the user types back. The vocabulary is ragrig's — components with
//! health, retrieval metadata on answers, streaming output — while the crate
//! stays independent of the framework: the host owns the state and wires its
//! own worker.
//!
//! | Widget | Module | Purpose |
//! |---|---|---|
//! | [`dock::SectionDock`] | [`dock`] | Side panel with stacked sections, a persisted width, and a horizontal collapse to a strip |
//! | [`chat_view::ChatView`] | [`chat_view`] | Scrollable transcript of chat messages |
//! | [`markdown`] (feature `markdown`) | [`markdown`] | Light Markdown rendering for assistant bubbles |
//! | [`composer::Composer`] | [`composer`] | Multiline input: Return = newline, Ctrl+Return = submit |
//! | [`attachments::show`] | [`attachments`] | Pending-context chips above the composer (toggle, pending, offer) |
//! | [`sessions::SessionRow`] | [`sessions`] | Conversation picker: new, switch, refresh |
//! | [`agent_bar::AgentBar`] | [`agent_bar`] | One icon per agent component, with status dots and slide-up menus |
//! | [`form`] | [`form`] | Setting rows and note/warning lines for dense menus |
//!
//! # Design
//!
//! - **Host-owned state.** The widgets borrow their model (messages, input
//!   buffer, open menu) instead of owning it, so the host decides where
//!   state lives and how it is persisted. The widgets are stateless between
//!   frames.
//! - **No runtime, no `ragrig` dependency.** Messages, tokens and statuses
//!   travel through the host's own channels; the widgets render plain data.
//! - **[`egui`]-only dependency.** Icons, strings and colors are supplied
//!   by the host (defaults are provided), so the crate composes with any
//!   icon font and localization scheme.
//!
//! # Example
//!
//! ```no_run
//! # use ragrig_egui::{chat_view::{ChatMessage, ChatView}, composer::Composer};
//! # fn ui(ui: &mut egui::Ui, messages: &[ChatMessage], input: &mut String) {
//! ChatView::new(messages).empty_text("Ask your library a question").show(ui);
//! let out = Composer::new(input).show(ui);
//! if out.submitted {
//!     // forward `input` to the host's worker, then clear it
//! }
//! # }
//! ```
//!
//! [egui]: https://github.com/emilk/egui
//! [ragrig]: https://crates.io/crates/ragrig

#![warn(missing_docs)]
#![forbid(unsafe_code)]

pub mod agent_bar;
pub mod attachments;
pub mod chat_view;
pub mod composer;
pub mod dock;
pub mod form;
#[cfg(feature = "markdown")]
pub mod markdown;
pub mod sessions;

/// Helpers shared by the module tests (widget discovery for click tests).
#[cfg(test)]
mod test_support {
    /// The *leaf* widgets of the previous pass, top to bottom and left to
    /// right — how a test finds a control's rect without knowing the layout.
    /// Layout uis register rects of their own; every rect that contains
    /// another one is a container, not a control.
    pub fn leaf_widgets(ctx: &egui::Context) -> Vec<egui::Rect> {
        let all: Vec<egui::Rect> = ctx.viewport(|viewport| {
            viewport
                .prev_pass
                .widgets
                .layers()
                .filter(|(layer, _)| **layer == egui::LayerId::background())
                .flat_map(|(_, widgets)| widgets.iter().map(|widget| widget.rect))
                .collect()
        });
        let mut rects: Vec<egui::Rect> = all
            .iter()
            .copied()
            .filter(|rect| !all.iter().any(|inner| contains(rect, inner)))
            .collect();
        rects.sort_by(|a, b| {
            (a.min.y, a.min.x, a.max.y, a.max.x)
                .partial_cmp(&(b.min.y, b.min.x, b.max.y, b.max.x))
                .unwrap()
        });
        rects
    }

    /// Strict containment: equal rects (nested uis of one size) do not count.
    fn contains(outer: &egui::Rect, inner: &egui::Rect) -> bool {
        outer != inner
            && outer.min.x <= inner.min.x
            && outer.min.y <= inner.min.y
            && outer.max.x >= inner.max.x
            && outer.max.y >= inner.max.y
    }
}
