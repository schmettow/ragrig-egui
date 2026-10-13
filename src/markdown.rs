//! Light Markdown rendering for chat messages (feature `markdown`).
//!
//! Parsing uses [`pulldown_cmark`] (CommonMark + strikethrough), rendering is
//! purpose-built for a chat bubble: paragraphs, headings, emphasis, inline
//! and fenced code, lists (nested, ordered), block quotes and rules. Links
//! render as underlined text with their target appended in small print —
//! clickable links are a later polish item.
//!
//! The parser deliberately keeps a *flat* block model ([`Block`]) so the
//! output can be asserted in unit tests without a UI, and so streaming text
//! can be re-parsed cheaply on every frame.
//!
//! ```no_run
//! # use egui_chat::markdown;
//! # fn ui(ui: &mut egui::Ui, text: &str) {
//! markdown::render(ui, &markdown::parse(text));
//! # }
//! ```

use egui::text::LayoutJob;
use egui::{Color32, FontId, RichText, Stroke, TextFormat, Ui};

use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd};

/// Marker in front of a list item.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ListMarker {
    /// An unordered item (`-`).
    Bullet,
    /// An ordered item with its number.
    Ordered(u64),
}

/// The kind of a rendered block.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BlockKind {
    /// A regular paragraph.
    Paragraph,
    /// A heading; `level` is 1–6.
    Heading {
        /// Heading level (1 = largest).
        level: u8,
    },
    /// A block quote (blocks inside quotes are marked as a whole).
    Quote,
    /// A list item at `depth` (0 = outermost) with its marker.
    ListItem {
        /// Nesting depth, starting at 0.
        depth: usize,
        /// Bullet or ordered number.
        marker: ListMarker,
    },
    /// A code block, with its info-string language when given.
    Code {
        /// Fenced-code language, e.g. `rust`.
        language: Option<String>,
    },
    /// A horizontal rule.
    Rule,
}

/// One inline run of text with its formatting.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Span {
    /// The text.
    pub text: String,
    /// Rendered with the theme's strong color.
    pub strong: bool,
    /// Italic.
    pub italic: bool,
    /// Inline code (monospace, tinted background).
    pub code: bool,
    /// Link target, when the span is part of a link or image alt text.
    pub link: Option<String>,
}

/// A block of the parsed document.
#[derive(Clone, Debug, PartialEq)]
pub struct Block {
    /// Block kind.
    pub kind: BlockKind,
    /// Inline content (empty for [`BlockKind::Rule`]).
    pub spans: Vec<Span>,
}

impl Block {
    /// Concatenated plain text of the block.
    pub fn text(&self) -> String {
        self.spans.iter().map(|span| span.text.as_str()).collect()
    }
}

/// Parse `text` into the flat block model.
pub fn parse(text: &str) -> Vec<Block> {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_STRIKETHROUGH);
    let parser = Parser::new_ext(text, options);

    let mut blocks: Vec<Block> = Vec::new();
    let mut current: Option<Block> = None;
    let mut style = Style::default();
    let mut lists: Vec<ListLevel> = Vec::new();
    let mut quote_depth: usize = 0;

    for event in parser {
        match event {
            Event::Start(Tag::Paragraph) => {
                // Paragraphs inside list items append to the open item block.
                if current.is_none() {
                    open_block(&mut current, BlockKind::Paragraph, quote_depth > 0);
                }
            }
            Event::End(TagEnd::Paragraph) => flush(&mut blocks, &mut current),
            Event::Start(Tag::Heading { level, .. }) => {
                let level = match level {
                    HeadingLevel::H1 => 1,
                    HeadingLevel::H2 => 2,
                    HeadingLevel::H3 => 3,
                    HeadingLevel::H4 => 4,
                    HeadingLevel::H5 => 5,
                    HeadingLevel::H6 => 6,
                };
                open_block(&mut current, BlockKind::Heading { level }, quote_depth > 0);
            }
            Event::End(TagEnd::Heading(_)) => flush(&mut blocks, &mut current),
            Event::Start(Tag::BlockQuote(_)) => quote_depth += 1,
            Event::End(TagEnd::BlockQuote(_)) => quote_depth = quote_depth.saturating_sub(1),
            Event::Start(Tag::CodeBlock(kind)) => {
                let language = match kind {
                    CodeBlockKind::Fenced(info) => {
                        let language = info.split_whitespace().next().unwrap_or_default();
                        (!language.is_empty()).then(|| language.to_string())
                    }
                    CodeBlockKind::Indented => None,
                };
                open_block(&mut current, BlockKind::Code { language }, false);
            }
            Event::End(TagEnd::CodeBlock) => {
                if let Some(block) = current.as_mut()
                    && let Some(span) = block.spans.last_mut()
                {
                    span.text = span.text.trim_end_matches('\n').to_string();
                }
                flush(&mut blocks, &mut current);
            }
            Event::Start(Tag::List(start)) => lists.push(ListLevel { next: start }),
            Event::End(TagEnd::List(_)) => {
                lists.pop();
            }
            Event::Start(Tag::Item) => {
                // A nested item (or a block that follows item text) closes
                // the parent item's inline content first.
                if matches!(
                    current.as_ref().map(|block| &block.kind),
                    Some(BlockKind::ListItem { .. })
                ) {
                    flush(&mut blocks, &mut current);
                }
                let depth = lists.len().saturating_sub(1);
                let marker = match lists.last_mut() {
                    Some(level) if level.next.is_some() => {
                        let next = level.next.as_mut().expect("checked by is_some");
                        let number = *next;
                        *next += 1;
                        ListMarker::Ordered(number)
                    }
                    _ => ListMarker::Bullet,
                };
                open_block(
                    &mut current,
                    BlockKind::ListItem { depth, marker },
                    quote_depth > 0,
                );
            }
            Event::End(TagEnd::Item) => flush(&mut blocks, &mut current),
            Event::Start(Tag::Emphasis) => style.italic = true,
            Event::End(TagEnd::Emphasis) => style.italic = false,
            Event::Start(Tag::Strong) => style.strong = true,
            Event::End(TagEnd::Strong) => style.strong = false,
            Event::Start(Tag::Link { dest_url, .. }) => style.link = Some(dest_url.to_string()),
            Event::End(TagEnd::Link) => style.link = None,
            Event::Start(Tag::Image { dest_url, .. }) => {
                style.link = Some(dest_url.to_string());
                style.italic = true;
            }
            Event::End(TagEnd::Image) => {
                style.link = None;
                style.italic = false;
            }
            Event::Text(text) => push_span(&mut current, &style, &text),
            Event::Code(code) => {
                let mut code_style = style.clone();
                code_style.code = true;
                push_span(&mut current, &code_style, &code);
            }
            Event::SoftBreak => push_span(&mut current, &style, " "),
            Event::HardBreak => push_span(&mut current, &style, "\n"),
            Event::Rule => {
                flush(&mut blocks, &mut current);
                blocks.push(Block {
                    kind: BlockKind::Rule,
                    spans: Vec::new(),
                });
            }
            Event::TaskListMarker(checked) => {
                push_span(&mut current, &style, if checked { "☒ " } else { "☐ " });
            }
            _ => {}
        }
    }
    flush(&mut blocks, &mut current);
    blocks
}

/// Render parsed `blocks` into `ui`.
pub fn render(ui: &mut Ui, blocks: &[Block]) {
    for (index, block) in blocks.iter().enumerate() {
        if index > 0 {
            ui.add_space(spacing_after(&blocks[index - 1]));
        }
        match &block.kind {
            BlockKind::Rule => {
                ui.separator();
            }
            BlockKind::Code { language } => {
                let text = block.text();
                let visuals = ui.visuals();
                egui::Frame::new()
                    .fill(visuals.extreme_bg_color)
                    .corner_radius(4.0)
                    .inner_margin(egui::Margin::symmetric(6, 4))
                    .show(ui, |ui| {
                        if let Some(language) = language {
                            ui.weak(RichText::new(language).size(10.0));
                        }
                        let mut job = LayoutJob::default();
                        job.wrap.max_width = ui.available_width().max(24.0);
                        job.append(
                            &text,
                            0.0,
                            TextFormat {
                                font_id: FontId::monospace(12.0),
                                color: ui.visuals().text_color(),
                                ..Default::default()
                            },
                        );
                        ui.add(egui::Label::new(job).selectable(true));
                    });
            }
            BlockKind::Heading { level } => {
                let size = heading_size(*level);
                let color = ui.visuals().strong_text_color();
                ui.add(egui::Label::new(job(ui, &block.spans, size, color)).selectable(true));
            }
            BlockKind::Quote => {
                let color = ui.visuals().weak_text_color();
                ui.horizontal_top(|ui| {
                    ui.add_space(8.0);
                    // A label in a *horizontal* layout extends instead of
                    // wrapping unless told otherwise — long quotes would run
                    // out of the bubble (and get clipped by the host).
                    ui.add(
                        egui::Label::new(job(ui, &block.spans, 13.0, color))
                            .selectable(true)
                            .wrap(),
                    );
                });
            }
            BlockKind::ListItem { depth, marker } => {
                let color = ui.visuals().text_color();
                ui.horizontal_top(|ui| {
                    ui.add_space(4.0 + *depth as f32 * 12.0);
                    let marker = match marker {
                        ListMarker::Bullet => "•".to_string(),
                        ListMarker::Ordered(number) => format!("{number}."),
                    };
                    ui.add(egui::Label::new(
                        RichText::new(marker)
                            .color(ui.visuals().weak_text_color())
                            .size(13.0),
                    ));
                    // See the quote arm: list items live in a horizontal
                    // layout, so wrapping must be requested explicitly.
                    ui.add(
                        egui::Label::new(job(ui, &block.spans, 13.0, color))
                            .selectable(true)
                            .wrap(),
                    );
                });
            }
            BlockKind::Paragraph => {
                let color = ui.visuals().text_color();
                ui.add(egui::Label::new(job(ui, &block.spans, 13.0, color)).selectable(true));
            }
        }
    }
}

fn spacing_after(block: &Block) -> f32 {
    match block.kind {
        BlockKind::Heading { .. } => 6.0,
        BlockKind::Code { .. } => 6.0,
        _ => 4.0,
    }
}

fn heading_size(level: u8) -> f32 {
    match level {
        1 => 17.0,
        2 => 15.5,
        3 => 14.5,
        _ => 13.5,
    }
}

/// Build a wrapping [`LayoutJob`] from the block's spans.
fn job(ui: &Ui, spans: &[Span], base_size: f32, default_color: Color32) -> LayoutJob {
    let visuals = ui.visuals();
    let mut job = LayoutJob::default();
    job.wrap.max_width = ui.available_width().max(24.0);
    for span in spans {
        let is_link = span.link.is_some();
        let color = if is_link {
            visuals.hyperlink_color
        } else if span.strong {
            visuals.strong_text_color()
        } else {
            default_color
        };
        let font_id = if span.code {
            FontId::monospace(base_size * 0.95)
        } else {
            FontId::proportional(base_size)
        };
        job.append(
            &span.text,
            0.0,
            TextFormat {
                font_id,
                color,
                background: if span.code {
                    visuals.code_bg_color
                } else {
                    Color32::TRANSPARENT
                },
                italics: span.italic,
                underline: if is_link {
                    Stroke::new(1.0_f32, visuals.hyperlink_color)
                } else {
                    Stroke::NONE
                },
                ..Default::default()
            },
        );
        // Show the target of a link whose text is not already the URL.
        if let Some(target) = &span.link
            && !target.is_empty()
            && span.text.trim() != target
        {
            job.append(
                &format!(" ({target})"),
                0.0,
                TextFormat {
                    font_id: FontId::proportional(base_size * 0.85),
                    color: visuals.weak_text_color(),
                    ..Default::default()
                },
            );
        }
    }
    job
}

/// Block-open helper that maps quoted blocks to [`BlockKind::Quote`].
fn open_block(current: &mut Option<Block>, kind: BlockKind, quoted: bool) {
    let kind = if quoted && !matches!(kind, BlockKind::Code { .. } | BlockKind::Rule) {
        BlockKind::Quote
    } else {
        kind
    };
    *current = Some(Block {
        kind,
        spans: Vec::new(),
    });
}

fn flush(blocks: &mut Vec<Block>, current: &mut Option<Block>) {
    if let Some(block) = current.take()
        && (!block.spans.is_empty() || matches!(block.kind, BlockKind::Rule))
    {
        blocks.push(block);
    }
}

#[derive(Clone, Debug, Default)]
struct Style {
    strong: bool,
    italic: bool,
    code: bool,
    link: Option<String>,
}

struct ListLevel {
    /// `None` = unordered; `Some(n)` = ordered, next number is `n`.
    next: Option<u64>,
}

fn push_span(current: &mut Option<Block>, style: &Style, text: &str) {
    let Some(block) = current.as_mut() else {
        return;
    };
    if text.is_empty() {
        return;
    }
    if let Some(last) = block.spans.last_mut()
        && last.strong == style.strong
        && last.italic == style.italic
        && last.code == style.code
        && last.link == style.link
    {
        last.text.push_str(text);
        return;
    }
    block.spans.push(Span {
        text: text.to_string(),
        strong: style.strong,
        italic: style.italic,
        code: style.code,
        link: style.link.clone(),
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_inline_styles() {
        let blocks = parse("**bold** and `code` and *italic*");
        assert_eq!(blocks.len(), 1);
        let spans = &blocks[0].spans;
        assert!(spans[0].strong && spans[0].text == "bold");
        assert!(spans[1].text == " and " && !spans[1].strong && !spans[1].code);
        assert!(spans[2].code && spans[2].text == "code");
        assert!(spans[3].text == " and ");
        assert!(spans[4].italic && spans[4].text == "italic");
    }

    #[test]
    fn parses_headings_with_levels() {
        let blocks = parse("# one\n\n## two\n\n#### four");
        let kinds: Vec<_> = blocks.iter().map(|b| b.kind.clone()).collect();
        assert_eq!(
            kinds,
            vec![
                BlockKind::Heading { level: 1 },
                BlockKind::Heading { level: 2 },
                BlockKind::Heading { level: 4 },
            ]
        );
        assert_eq!(blocks[0].text(), "one");
    }

    #[test]
    fn parses_nested_and_ordered_lists() {
        let blocks = parse("- a\n  - b\n1. c\n2. d");
        let items: Vec<_> = blocks
            .iter()
            .filter_map(|b| match &b.kind {
                BlockKind::ListItem { depth, marker } => Some((*depth, *marker, b.text())),
                _ => None,
            })
            .collect();
        assert_eq!(
            items,
            vec![
                (0, ListMarker::Bullet, "a".to_string()),
                (1, ListMarker::Bullet, "b".to_string()),
                (0, ListMarker::Ordered(1), "c".to_string()),
                (0, ListMarker::Ordered(2), "d".to_string()),
            ]
        );
    }

    #[test]
    fn parses_fenced_code_with_language() {
        let blocks = parse("```rust\nfn main() {}\n```");
        assert_eq!(blocks.len(), 1);
        assert_eq!(
            blocks[0].kind,
            BlockKind::Code {
                language: Some("rust".to_string())
            }
        );
        assert_eq!(blocks[0].text(), "fn main() {}");
    }

    #[test]
    fn soft_break_becomes_space_hard_break_newline() {
        let blocks = parse("line one\nline two");
        assert_eq!(blocks[0].text(), "line one line two");

        let blocks = parse("line one  \nline two");
        assert_eq!(blocks[0].text(), "line one\nline two");
    }

    #[test]
    fn parses_rule_between_paragraphs() {
        let blocks = parse("a\n\n---\n\nb");
        let kinds: Vec<_> = blocks.iter().map(|b| b.kind.clone()).collect();
        assert_eq!(
            kinds,
            vec![BlockKind::Paragraph, BlockKind::Rule, BlockKind::Paragraph]
        );
    }

    #[test]
    fn parses_block_quote() {
        let blocks = parse("> quoted text");
        assert_eq!(blocks[0].kind, BlockKind::Quote);
        assert_eq!(blocks[0].text(), "quoted text");
    }

    #[test]
    fn parses_link_with_target() {
        let blocks = parse("[docs](https://example.com)");
        let span = &blocks[0].spans[0];
        assert_eq!(span.text, "docs");
        assert_eq!(span.link.as_deref(), Some("https://example.com"));
    }

    #[test]
    fn empty_input_yields_no_blocks() {
        assert!(parse("").is_empty());
        assert!(parse("   \n").is_empty());
    }

    #[test]
    fn renders_rich_text_without_panic() {
        let source = "# Title\n\nA **bold** paragraph with `code`, a [link](https://x.dev) \
                      and\nmore text.\n\n- item\n  - nested\n1. ordered\n\n> quote\n\n\
                      ```rust\nlet x = 1;\n```\n\n---\n\n☒ done\n";
        let blocks = parse(source);
        let ctx = egui::Context::default();
        let _ = ctx.run_ui(egui::RawInput::default(), |ui| {
            render(ui, &blocks);
        });
    }

    #[test]
    fn rendered_blocks_stay_within_the_available_width() {
        // Regression: list items and quotes draw inside *horizontal* layouts,
        // where labels extend instead of wrapping unless asked to wrap. A
        // long bullet used to demand thousands of pixels in a 300 px column
        // (and got clipped by the host panel).
        let ctx = egui::Context::default();
        let long = "word ".repeat(60);
        let source = format!(
            "Intro paragraph with {long}\n\n- bullet with {long}\n- second bullet\n\n> quote with {long}\n\n# Heading with {long}\n"
        );
        let mut inner = egui::Rect::NOTHING;
        let _ = ctx.run_ui(egui::RawInput::default(), |ui| {
            ui.allocate_ui_with_layout(
                egui::vec2(300.0, 600.0),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    render(ui, &parse(&source));
                    inner = ui.min_rect();
                },
            );
        });
        assert!(
            inner.width() <= 301.0,
            "content overflowed its column: {} px wide in 300 px",
            inner.width()
        );
        assert!(
            inner.height() > 300.0,
            "the text must wrap into many lines, got {} px",
            inner.height()
        );
    }

    #[test]
    fn renders_partial_streaming_markdown() {
        // Unterminated fence and trailing emphasis must not panic.
        let ctx = egui::Context::default();
        let _ = ctx.run_ui(egui::RawInput::default(), |ui| {
            render(ui, &parse("Answer so far **bo"));
            render(ui, &parse("```rust\nfn par"));
        });
    }
}
