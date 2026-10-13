# egui_chat

Reusable chat and agent-panel widgets for [egui](https://github.com/emilk/egui).

`egui_chat` is a small, dependency-light widget crate for building
assistant-style side panels: a chat transcript, a composer with conventional
submit keys, a component status bar, and a dockable panel that hosts them.
It was extracted from the [sciscraper](https://github.com/schmettow) RAG
editor, where it drives the "agent pane" for a local retrieval-augmented
writing assistant — but it knows nothing about LLMs or RAG.

## Widgets

| Widget | What it does |
|---|---|
| `dock::SectionDock` | Side panel (left/right) with stacked, collapsible sections and a persisted width |
| `chat_view::ChatView` | Scrollable transcript of `ChatMessage`s: user/assistant/system bubbles, streaming state, error state, metadata footer (`12 chunks · 3 sources · 8.2 s`) |
| `composer::Composer` | Multiline input with **Return = newline**, **Ctrl/⌘+Return = submit**, send/stop buttons; grows to six rows, then scrolls inside |
| `agent_bar::AgentBar` | One icon per agent component with status dots (grey/green/pulsing/red) and a slide-up menu drawn by the host |
| `markdown` (feature, default) | Light Markdown rendering for assistant bubbles: emphasis, headings, lists, inline/fenced code, quotes, links |
| `form` | Setting rows (`setting_row`) and note/warning lines for dense component menus |

## Quick start

```rust,no_run
use egui_chat::agent_bar::{AgentBar, Component, ComponentStatus};
use egui_chat::chat_view::{ChatMessage, ChatView};
use egui_chat::composer::Composer;
use egui_chat::dock::{DockSide, SectionDock, SectionState};

fn agent_pane(
    ui: &mut egui::Ui,
    messages: &[ChatMessage],
    input: &mut String,
    open_menu: &mut Option<String>,
    agent_section: &mut SectionState,
    library_section: &mut SectionState,
) {
    let components = [
        Component::new("chat", "Chat model", "C").status(ComponentStatus::Ready),
        Component::new("embed", "Embedder", "E").detail("nomic-embed-text"),
    ];
    SectionDock::new("agent_dock", DockSide::Right, 360.0)
        .section(agent_section, |ui| {
            ChatView::new(messages)
                .empty_text("Ask your library")
                .markdown(true)
                .show(ui);
            let out = Composer::new(input).show(ui);
            if out.submitted {
                // forward `input` to your worker, then clear it
            }
            AgentBar::new(&components, open_menu).show(ui, |ui, id| {
                ui.label(format!("settings for {id}"));
                false
            });
        })
        .section(library_section, |ui| {
            ui.label("source list");
        })
        .show(ui);
}
```

## Design principles

- **Host-owned state.** Widgets borrow their model (messages, input buffer,
  open menu, section state) rather than owning it; the host decides where
  state lives and how it is persisted.
- **No async, no domain knowledge.** Statuses, messages and tokens travel
  through the host's own channels; nothing here blocks a frame.
- **egui-only dependency.** Icons, labels and colors are host-supplied
  (sensible defaults included), so the crate composes with any icon font and
  localization scheme.
- **Bounded widgets.** Nothing here grows without a limit: bubbles wrap inside
  the available width, and the composer stops at six rows. Together with the
  bottom-anchored footer recipe (see `composer::max_composer_height`) that is
  what keeps a host's layout from being pushed out of its panel by a long
  message or a multi-line prompt.

## Roadmap

- **0.1** — dock, chat view, composer, agent bar; streaming and error message
  states; metadata footers; status dot semantics.
- **0.2** — light Markdown in assistant bubbles (emphasis, headings,
  lists, inline/fenced code, quotes, links with targets) via `pulldown-cmark`,
  behind the default `markdown` feature; streaming-safe (partial Markdown
  renders without panicking).
- **0.3.1** (current) — `form` helpers for dense menus (`setting_row`, `note`,
  `warning`); bounded widgets: the composer grows to six rows then scrolls, and
  Markdown list items/quotes wrap inside the available width (they used to
  extend past the bubble and get clipped).
- **0.4** — message actions (copy feedback, retry, delete), clickable links,
  draggable splitter between dock sections.
- **0.5** — tabbed dock mode for hosts that need more than two panes; theme
  presets; publish to crates.io.

## Compatibility

- `egui` 0.34, Rust 1.92 (edition 2024).

## License

MIT — see `LICENSE`.
