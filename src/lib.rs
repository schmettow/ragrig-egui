//! Reusable chat and agent-panel widgets for [egui].
//!
//! `egui_chat` provides the UI building blocks for assistant-style panels —
//! a chat transcript, a composer with conventional submit keys, a component
//! status bar, and a dockable side panel that hosts them:
//!
//! | Widget | Module | Purpose |
//! |---|---|---|
//! | [`dock::SectionDock`] | [`dock`] | Side panel with stacked, collapsible sections |
//! | [`chat_view::ChatView`] | [`chat_view`] | Scrollable transcript of chat messages |
//! | [`markdown`] (feature `markdown`) | [`markdown`] | Light Markdown rendering for assistant bubbles |
//! | [`composer::Composer`] | [`composer`] | Multiline input: Return = newline, Ctrl+Return = submit |
//! | [`agent_bar::AgentBar`] | [`agent_bar`] | One icon per agent component, with status dots and slide-up menus |
//! | [`form`] | [`form`] | Setting rows and note/warning lines for dense menus |
//!
//! # Design
//!
//! - **Host-owned state.** The widgets borrow their model (messages, input
//!   buffer, open menu) instead of owning it, so the host decides where
//!   state lives and how it is persisted. The widgets are stateless between
//!   frames.
//! - **No async, no domain knowledge.** Nothing here knows about LLMs,
//!   RAG pipelines, or workers; send messages, stream tokens, and track
//!   statuses through the host's own channels.
//! - **[`egui`]-only dependency.** Icons, strings and colors are supplied
//!   by the host (defaults are provided), so the crate composes with any
//!   icon font and localization scheme.
//!
//! # Example
//!
//! ```no_run
//! # use egui_chat::{chat_view::{ChatMessage, ChatView}, composer::Composer};
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

#![warn(missing_docs)]
#![forbid(unsafe_code)]

pub mod agent_bar;
pub mod chat_view;
pub mod composer;
pub mod dock;
pub mod form;
#[cfg(feature = "markdown")]
pub mod markdown;
