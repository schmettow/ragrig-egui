//! Multiline chat composer with conventional submit keys.
//!
//! The composer follows the convention common in assistant panels:
//!
//! - **Return** inserts a newline (chat prompts span paragraphs),
//! - **Ctrl+Return** (⌘+Return on macOS) submits,
//! - a Send button submits, a Stop button cancels a running request.
//!
//! The [`Composer`] borrows the host's input buffer and reports what
//! happened in [`ComposerOutput`]; it never clears the buffer itself, so the
//! host stays in control (e.g. keep the text when sending fails).

use egui::{Key, Modifiers, RichText, Ui};

/// The input grows with its content up to this many text rows, then scrolls
/// internally.
///
/// A bounded composer is what keeps a multi-line prompt from pushing the
/// host's layout around: the field can never demand more than
/// [`Composer::max_input_height`], so a host can reserve that much and keep
/// everything below it visible.
pub const MAX_INPUT_ROWS: usize = 6;

/// What the user did in the composer this frame.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ComposerOutput {
    /// The user submitted (Ctrl+Return or the send button).
    pub submitted: bool,
    /// The user pressed Stop while a request was running.
    pub cancelled: bool,
}

/// Multiline input for chat messages.
///
/// ```no_run
/// # use egui_chat::composer::Composer;
/// # fn ui(ui: &mut egui::Ui, input: &mut String, busy: bool) {
/// let output = Composer::new(input).busy(busy).show(ui);
/// if output.submitted {
///     // hand `input` to the worker, then clear it
/// }
/// if output.cancelled {
///     // signal the worker to stop
/// }
/// # }
/// ```
pub struct Composer<'a> {
    text: &'a mut String,
    busy: bool,
    enabled: bool,
    placeholder: &'a str,
    send_label: &'a str,
    stop_label: &'a str,
    hint: Option<&'a str>,
    rows: usize,
}

impl<'a> Composer<'a> {
    /// A composer editing `text`.
    pub fn new(text: &'a mut String) -> Self {
        Self {
            text,
            busy: false,
            enabled: true,
            placeholder: "Type your question…",
            send_label: "Send",
            stop_label: "Stop",
            hint: None,
            rows: 2,
        }
    }

    /// The stable id of the composer's text field (useful for tests and for
    /// programmatically focusing the input).
    pub fn input_id() -> egui::Id {
        egui::Id::new("egui_chat::composer::input")
    }

    /// Set while a request is running: the stop button shows and submission
    /// is disabled.
    pub fn busy(mut self, busy: bool) -> Self {
        self.busy = busy;
        self
    }

    /// Disable the whole composer (e.g. no backend configured).
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Hint text shown while the buffer is empty.
    pub fn placeholder(mut self, placeholder: &'a str) -> Self {
        self.placeholder = placeholder;
        self
    }

    /// Override the button labels.
    pub fn labels(mut self, send: &'a str, stop: &'a str) -> Self {
        self.send_label = send;
        self.stop_label = stop;
        self
    }

    /// Show a hint line under the buttons (e.g. the submit-key reminder).
    pub fn hint(mut self, hint: &'a str) -> Self {
        self.hint = Some(hint);
        self
    }

    /// Number of visible text rows (default 2).
    pub fn rows(mut self, rows: usize) -> Self {
        self.rows = rows;
        self
    }

    /// The height the input area never exceeds (see [`MAX_INPUT_ROWS`]).
    ///
    /// Hosts lay out “transcript above, composer below” in immediate mode,
    /// where a widget's height is only known after it is drawn; reserving
    /// this bound (or the previous frame's measured height) keeps a tall
    /// prompt from pushing the rest of the panel off-screen.
    pub fn max_input_height(ui: &Ui) -> f32 {
        input_height(ui, MAX_INPUT_ROWS)
    }

    /// Render the composer into `ui`.
    pub fn show(self, ui: &mut Ui) -> ComposerOutput {
        // Consume Ctrl/Cmd+Return *before* the text field sees it, so it
        // submits instead of inserting a newline.
        let submit_shortcut = ui.input_mut(|i| i.consume_key(Modifiers::COMMAND, Key::Enter));

        let mut output = ComposerOutput::default();
        ui.add_enabled_ui(self.enabled, |ui| {
            // The field keeps an *exact* height derived from the line count
            // (capped at [`MAX_INPUT_ROWS`]; longer text scrolls inside): a
            // content-sized editor would make the host's layout depend on the
            // previous frame, and a multi-line prompt would push everything
            // below it out of the panel.
            let rows = self.text.lines().count().clamp(1, MAX_INPUT_ROWS);
            ui.allocate_ui_with_layout(
                egui::vec2(ui.available_width(), input_height(ui, rows)),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    egui::ScrollArea::vertical()
                        .id_salt("egui_chat::composer::input_scroll")
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            ui.add(
                                egui::TextEdit::multiline(self.text)
                                    .id(Self::input_id())
                                    .desired_rows(rows)
                                    .desired_width(f32::INFINITY)
                                    .hint_text(self.placeholder),
                            );
                        });
                },
            );
            ui.horizontal(|ui| {
                if self.busy {
                    if ui.button(self.stop_label).clicked() {
                        output.cancelled = true;
                    }
                } else {
                    let can_send = !self.text.trim().is_empty();
                    if ui
                        .add_enabled(can_send, egui::Button::new(self.send_label))
                        .clicked()
                    {
                        output.submitted = true;
                    }
                }
                if let Some(hint) = self.hint {
                    ui.weak(RichText::new(hint).size(11.0));
                }
            });
        });

        if submit_shortcut && self.enabled && !self.busy && !self.text.trim().is_empty() {
            output.submitted = true;
        }
        output
    }
}

/// Height the input field needs for `rows` text rows (bounded by
/// [`MAX_INPUT_ROWS`]).
///
/// This is exact: [`Composer::show`] sizes the field with it, so hosts can
/// reserve the space before drawing anything.
pub fn input_height(ui: &Ui, rows: usize) -> f32 {
    const FIELD_MARGIN: f32 = 8.0;
    ui.text_style_height(&egui::TextStyle::Body) * rows.clamp(1, MAX_INPUT_ROWS) as f32
        + FIELD_MARGIN
}

/// Height a composer needs at most: the capped input plus its button row.
///
/// Hosts that anchor the composer at the bottom of a panel use this to size
/// the block the composer can never exceed.
pub fn max_composer_height(ui: &Ui) -> f32 {
    input_height(ui, MAX_INPUT_ROWS) + composer_chrome(ui)
}

/// The Send/Stop row plus the spacing around it.
fn composer_chrome(ui: &Ui) -> f32 {
    ui.spacing().interact_size.y + 2.0 * ui.spacing().item_spacing.y
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{Event, RawInput};

    fn key_event(key: Key, modifiers: Modifiers) -> Event {
        Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers,
        }
    }

    /// Run one frame with the composer focused and the given events.
    fn run_frame(text: &mut String, events: Vec<Event>) -> ComposerOutput {
        let ctx = egui::Context::default();
        // Focus the composer's text field (its id is stable by design).
        ctx.memory_mut(|m| m.request_focus(Composer::input_id()));
        let input = RawInput {
            events,
            ..RawInput::default()
        };
        let mut output = ComposerOutput::default();
        let _ = ctx.run_ui(input, |ui| {
            output = Composer::new(text).show(ui);
        });
        output
    }

    #[test]
    fn return_inserts_newline_and_does_not_submit() {
        let mut text = String::from("hello");
        let output = run_frame(&mut text, vec![key_event(Key::Enter, Modifiers::NONE)]);
        assert!(!output.submitted, "plain Return must not submit");
        assert_eq!(text, "hello\n");
    }

    #[test]
    fn ctrl_return_submits_without_inserting_newline() {
        let mut text = String::from("hello");
        let output = run_frame(&mut text, vec![key_event(Key::Enter, Modifiers::COMMAND)]);
        assert!(output.submitted, "Ctrl+Return must submit");
        assert_eq!(text, "hello", "Ctrl+Return must not insert a newline");
    }

    #[test]
    fn ctrl_return_on_empty_input_does_not_submit() {
        let mut text = String::new();
        let output = run_frame(&mut text, vec![key_event(Key::Enter, Modifiers::COMMAND)]);
        assert!(!output.submitted);
        assert!(text.is_empty());
    }

    #[test]
    fn ctrl_return_while_busy_does_not_submit() {
        let mut text = String::from("hello");
        let ctx = egui::Context::default();
        ctx.memory_mut(|m| m.request_focus(Composer::input_id()));
        let input = RawInput {
            events: vec![key_event(Key::Enter, Modifiers::COMMAND)],
            ..RawInput::default()
        };
        let mut output = ComposerOutput::default();
        let _ = ctx.run_ui(input, |ui| {
            output = Composer::new(&mut text).busy(true).show(ui);
        });
        assert!(!output.submitted);
        assert!(!output.cancelled);
    }

    #[test]
    fn a_long_prompt_does_not_grow_the_composer_past_its_cap() {
        // Regression: an unbounded input used to push the host's agent bar and
        // whatever sits below it out of the panel.
        let ctx = egui::Context::default();
        let mut text = "a long prompt line\n".repeat(40);
        let mut height = 0.0f32;
        let mut cap = 0.0f32;
        let _ = ctx.run_ui(RawInput::default(), |ui| {
            cap = max_composer_height(ui) + 3.0 * ui.spacing().item_spacing.y;
            let before = ui.min_rect().bottom();
            let _ = Composer::new(&mut text).show(ui);
            height = ui.min_rect().bottom() - before;
        });
        assert!(
            height <= cap + 1.0,
            "composer grew to {height} px (cap {cap})"
        );
        assert!(height > 0.0);
    }

    #[test]
    fn the_height_estimate_matches_the_capped_composer() {
        let ctx = egui::Context::default();
        for lines in [1usize, 2, MAX_INPUT_ROWS, 40] {
            let mut text = "x\n".repeat(lines);
            let mut height = 0.0f32;
            let mut cap = 0.0f32;
            let _ = ctx.run_ui(RawInput::default(), |ui| {
                cap = max_composer_height(ui);
                let before = ui.min_rect().bottom();
                let _ = Composer::new(&mut text).show(ui);
                height = ui.min_rect().bottom() - before;
            });
            assert!(
                height <= cap + 0.5,
                "{lines} lines needed {height} px, cap is {cap}"
            );
        }
    }
}
