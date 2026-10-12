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
| `composer::Composer` | Multiline input with **Return = newline**, **Ctrl/⌘+Return = submit**, send/stop buttons |
| `agent_bar::AgentBar` | One icon per agent component with status dots (grey/green/pulsing/red) and a slide-up menu drawn by the host |

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
            ChatView::new(messages).empty_text("Ask your library").show(ui);
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

## Roadmap

- **0.1** (current) — dock, chat view, composer, agent bar; streaming and
  error message states; metadata footers; status dot semantics.
- **0.2** — rich text in assistant bubbles (light Markdown: emphasis, lists,
  inline/fenced code) plus message actions (copy feedback, retry, delete).
- **0.3** — draggable splitter between dock sections; menu builder helpers
  (setting rows: text field, slider, combo) to reduce host boilerplate.
- **0.4** — tabbed dock mode for hosts that need more than two panes; theme
  presets.

## Compatibility

- `egui` 0.34, Rust 1.92 (edition 2024).

## License

MIT — see `LICENSE`.
