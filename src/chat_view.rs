//! Scrollable transcript of chat messages.
//!
//! [`ChatView`] renders a slice of [`ChatMessage`]s as left/right aligned
//! bubbles: user messages on the right, assistant messages on the left,
//! system notes centered. Assistant messages can render a metadata footer
//! (retrieved chunks, documents, token counts, elapsed time) and an error
//! state. The view sticks to the bottom while the user has not scrolled up.
//!
//! Rendering is intentionally lightweight (selectable text, no Markdown
//! layout yet); rich-text rendering is a planned addition that will not
//! change this API.

use std::time::Duration;

use egui::{Color32, RichText, Ui};

/// Who authored a message.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChatRole {
    /// The human user.
    User,
    /// The assistant / model.
    Assistant,
    /// System notice (state changes, hints, errors without a message).
    System,
}

/// Lifecycle state of a message.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum MessageState {
    /// Fully received.
    #[default]
    Complete,
    /// Still streaming; a caret is appended to the rendered text.
    Streaming,
    /// The message body describes an error; the bubble is tinted.
    Error,
}

/// Pipeline metadata shown under an assistant message.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MessageMeta {
    /// Number of retrieved chunks that entered the prompt.
    pub chunks: Option<usize>,
    /// Distinct source documents among the retrieved chunks.
    pub documents: Vec<String>,
    /// Wall-clock generation time.
    pub elapsed: Option<Duration>,
    /// Number of streamed tokens, when counted by the host.
    pub tokens: Option<usize>,
}

impl MessageMeta {
    /// One-line summary, e.g. `12 chunks · 3 sources · 421 tokens · 8.2 s`.
    /// Empty when the metadata carries nothing to show.
    pub fn summary(&self) -> String {
        let mut parts = Vec::new();
        if let Some(chunks) = self.chunks {
            parts.push(format!("{chunks} chunks"));
        }
        if !self.documents.is_empty() {
            let shown = self.documents.iter().take(3).cloned().collect::<Vec<_>>();
            let more = self.documents.len().saturating_sub(shown.len());
            let mut names = shown.join(", ");
            if more > 0 {
                names.push_str(&format!(" +{more}"));
            }
            parts.push(format!("sources: {names}"));
        }
        if let Some(tokens) = self.tokens {
            parts.push(format!("{tokens} tokens"));
        }
        if let Some(elapsed) = self.elapsed {
            parts.push(format!("{:.1} s", elapsed.as_secs_f32()));
        }
        parts.join(" · ")
    }
}

/// One chat message as displayed in the transcript.
#[derive(Clone, Debug, PartialEq)]
pub struct ChatMessage {
    /// Author of the message.
    pub role: ChatRole,
    /// Message body (plain text for now, Markdown rendering is planned).
    pub text: String,
    /// Lifecycle state of the message.
    pub state: MessageState,
    /// Optional metadata footer (assistant messages typically have one).
    pub meta: Option<MessageMeta>,
}

impl ChatMessage {
    /// A complete user message.
    pub fn user(text: impl Into<String>) -> Self {
        Self {
            role: ChatRole::User,
            text: text.into(),
            state: MessageState::Complete,
            meta: None,
        }
    }

    /// A complete assistant message.
    pub fn assistant(text: impl Into<String>) -> Self {
        Self {
            role: ChatRole::Assistant,
            text: text.into(),
            state: MessageState::Complete,
            meta: None,
        }
    }

    /// A system notice.
    pub fn system(text: impl Into<String>) -> Self {
        Self {
            role: ChatRole::System,
            text: text.into(),
            state: MessageState::Complete,
            meta: None,
        }
    }

    /// Mark the message as still streaming.
    pub fn streaming(mut self) -> Self {
        self.state = MessageState::Streaming;
        self
    }
}

/// What the user did inside the transcript.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ChatViewOutput {
    /// Index of the message whose "Copy" context action was used.
    pub copied: Option<usize>,
}

/// Renders a transcript of [`ChatMessage`]s.
pub struct ChatView<'a> {
    messages: &'a [ChatMessage],
    empty_text: Option<&'a str>,
    markdown: bool,
}

impl<'a> ChatView<'a> {
    /// A view over `messages`.
    pub fn new(messages: &'a [ChatMessage]) -> Self {
        Self {
            messages,
            empty_text: None,
            markdown: false,
        }
    }

    /// Text shown while the transcript is empty.
    pub fn empty_text(mut self, text: &'a str) -> Self {
        self.empty_text = Some(text);
        self
    }

    /// Render assistant messages as light Markdown (feature `markdown`).
    pub fn markdown(mut self, markdown: bool) -> Self {
        self.markdown = markdown;
        self
    }

    /// Render the transcript into `ui`.
    pub fn show(self, ui: &mut Ui) -> ChatViewOutput {
        let mut output = ChatViewOutput::default();
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .stick_to_bottom(true)
            .show(ui, |ui| {
                if self.messages.is_empty() {
                    if let Some(text) = self.empty_text {
                        ui.add_space(8.0);
                        ui.weak(text);
                    }
                    return;
                }
                for (index, message) in self.messages.iter().enumerate() {
                    if message_bubble(ui, message, self.markdown) {
                        output.copied = Some(index);
                    }
                    ui.add_space(4.0);
                }
            });
        output
    }
}

/// Draw one message bubble; returns `true` when the user copied it.
fn message_bubble(ui: &mut Ui, message: &ChatMessage, rich_markdown: bool) -> bool {
    let mut copied = false;
    let align = match message.role {
        ChatRole::User => egui::Align::Max,
        ChatRole::Assistant => egui::Align::Min,
        ChatRole::System => egui::Align::Center,
    };

    ui.with_layout(egui::Layout::top_down(align), |ui| {
        ui.set_max_width(ui.available_width() * 0.9);
        let fill = bubble_fill(ui, message);
        let stroke = if message.state == MessageState::Error {
            egui::Stroke::new(1.0_f32, ui.visuals().error_fg_color)
        } else {
            egui::Stroke::NONE
        };
        let text_color = if message.state == MessageState::Error {
            ui.visuals().error_fg_color
        } else {
            ui.visuals().text_color()
        };

        let frame = egui::Frame::new()
            .fill(fill)
            .stroke(stroke)
            .corner_radius(6.0)
            .inner_margin(egui::Margin::symmetric(8, 5));

        let inner = frame.show(ui, |ui| {
            let mut text = message.text.clone();
            if message.state == MessageState::Streaming {
                text.push('\u{258C}');
            }
            let rich = rich_markdown && message.role == ChatRole::Assistant;
            render_message_text(ui, &text, text_color, rich);
            if let Some(meta) = &message.meta {
                let summary = meta.summary();
                if !summary.is_empty() {
                    ui.add_space(2.0);
                    ui.weak(summary);
                }
            }
        });

        inner.response.context_menu(|ui| {
            if ui.button("Copy message").clicked() {
                ui.ctx().copy_text(message.text.clone());
                copied = true;
                ui.close();
            }
        });
    });
    copied
}

fn bubble_fill(ui: &Ui, message: &ChatMessage) -> Color32 {
    let visuals = ui.visuals();
    match message.role {
        ChatRole::User => visuals.selection.bg_fill.gamma_multiply(0.35),
        ChatRole::Assistant => visuals.faint_bg_color,
        ChatRole::System => visuals.faint_bg_color,
    }
}

/// Plain text by default; parsed Markdown for assistant bubbles when the
/// `markdown` feature is enabled and requested.
fn render_message_text(ui: &mut Ui, text: &str, color: Color32, rich: bool) {
    #[cfg(feature = "markdown")]
    if rich {
        crate::markdown::render(ui, &crate::markdown::parse(text));
        return;
    }
    #[cfg(not(feature = "markdown"))]
    let _ = rich;
    ui.add(egui::Label::new(RichText::new(text).color(color)).selectable(true));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn meta_summary_formats_all_fields() {
        let meta = MessageMeta {
            chunks: Some(12),
            documents: vec!["paper-a.pdf".into(), "paper-b.pdf".into()],
            elapsed: Some(Duration::from_secs_f32(8.24)),
            tokens: Some(421),
        };
        assert_eq!(
            meta.summary(),
            "12 chunks · sources: paper-a.pdf, paper-b.pdf · 421 tokens · 8.2 s"
        );
    }

    #[test]
    fn meta_summary_truncates_many_documents() {
        let meta = MessageMeta {
            documents: vec!["a".into(), "b".into(), "c".into(), "d".into(), "e".into()],
            ..MessageMeta::default()
        };
        assert_eq!(meta.summary(), "sources: a, b, c +2");
    }

    #[test]
    fn meta_summary_empty_when_no_data() {
        assert_eq!(MessageMeta::default().summary(), "");
    }

    #[test]
    fn chat_message_constructors_set_roles() {
        assert_eq!(ChatMessage::user("q").role, ChatRole::User);
        assert_eq!(ChatMessage::assistant("a").role, ChatRole::Assistant);
        let streaming = ChatMessage::assistant("a").streaming();
        assert_eq!(streaming.state, MessageState::Streaming);
    }

    #[test]
    fn chat_view_renders_all_states_without_panic() {
        let messages = vec![
            ChatMessage::user("question"),
            ChatMessage::assistant("answer").streaming(),
            ChatMessage::system("note"),
            ChatMessage {
                role: ChatRole::Assistant,
                text: "boom".into(),
                state: MessageState::Error,
                meta: Some(MessageMeta {
                    chunks: Some(3),
                    ..MessageMeta::default()
                }),
            },
        ];
        let ctx = egui::Context::default();
        let _ = ctx.run_ui(egui::RawInput::default(), |ui| {
            let output = ChatView::new(&messages).show(ui);
            assert_eq!(output.copied, None);
        });
    }

    #[test]
    fn chat_view_renders_with_markdown_enabled() {
        let messages = vec![
            ChatMessage::user("**raw user text**"),
            ChatMessage::assistant("# Title\n\n- item\n\n```rust\nlet x = 1;\n```"),
        ];
        let ctx = egui::Context::default();
        let _ = ctx.run_ui(egui::RawInput::default(), |ui| {
            ChatView::new(&messages).markdown(true).show(ui);
        });
    }

    #[test]
    fn chat_view_renders_empty_transcript() {
        let ctx = egui::Context::default();
        let empty: Vec<ChatMessage> = Vec::new();
        let _ = ctx.run_ui(egui::RawInput::default(), |ui| {
            ChatView::new(&empty).empty_text("nothing yet").show(ui);
        });
    }
}
