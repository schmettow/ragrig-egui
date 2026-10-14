# ragrig-egui

The egui agent interface for [ragrig](https://crates.io/crates/ragrig).

`ragrig-egui` is the front-end half of a ragrig workspace. Where `ragrig` runs
the agents, the retrieval and the indexing, this crate gives them a face in an
[egui](https://github.com/emilk/egui) application: what an agent says, what it
is doing, which components are up, and where the user types back.

It is used by [sciscraper](https://github.com/schmettow/sciscraper-rs), a scientific
Markdown editor whose agent pane is driven by `ragrig` — and it is shaped
by that job rather than by chat in general:

- the **transcript** understands retrieval: metadata footers
  (`12 chunks · 3 sources · 8.2 s`), streaming answers, error states, and a
  Markdown mode for answers that carry structure;
- the **agent bar** speaks ragrig's component vocabulary — chat model, memory,
  embedder, vector store, parsers, chunker: one icon per component with a
  health dot, and a slide-up menu the host fills with that component's
  settings;
- the **composer** follows the convention a terminal or an agent pane trains:
  **Return = newline**, **Ctrl/⌘+Return = submit**, with send/stop
  affordances;
- the **dock** hosts the panes these widgets live in, with the persistence a
  long-running workspace expects: width, a horizontal collapse to a strip, and
  an optional close button.

Adopting it stays frictionless: the crate does not link `ragrig` and brings no
async runtime — `egui` is the only dependency. The host owns the state and its
worker; the widgets render what it hands them.

## Widgets

| Widget | What it does |
|---|---|
| `dock::SectionDock` | Side panel (left/right) with stacked sections, a persisted width, and a **horizontal** collapse to a narrow strip |
| `chat_view::ChatView` | Scrollable transcript of `ChatMessage`s: user/assistant/system bubbles, streaming state, error state, metadata footer (`12 chunks · 3 sources · 8.2 s`) |
| `composer::Composer` | Multiline input with **Return = newline**, **Ctrl/⌘+Return = submit**, send/stop buttons; grows to six rows, then scrolls inside |
| `attachments` | Pending-context chips above the composer: a **toggle** (the document buffer, the bibliography), a **pending** context with a ✕, and an **offer** with an action button |
| `sessions::SessionRow` | Conversation picker: *New*, a dropdown of saved sessions, and a refresh action |
| `agent_bar::AgentBar` | One icon per agent component with status dots (grey/green/pulsing/red) and a slide-up menu drawn by the host |
| `markdown` (feature, default) | Light Markdown rendering for assistant bubbles: emphasis, headings, lists, inline/fenced code, quotes, links |
| `form` | Setting rows (`setting_row`) and note/warning lines for dense component menus |

## Quick start

```rust,no_run
use ragrig_egui::agent_bar::{AgentBar, Component, ComponentStatus};
use ragrig_egui::chat_view::{ChatMessage, ChatView};
use ragrig_egui::composer::Composer;
use ragrig_egui::dock::{DockSide, SectionDock, SectionState};

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
- **No runtime.** Nothing here blocks a frame, and nothing links `ragrig` or
  an async runtime: messages, tokens and component statuses travel through the
  host's own channels, and the widgets render plain data.
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
- **0.3.1** — `form` helpers for dense menus (`setting_row`, `note`,
  `warning`); bounded widgets: the composer grows to six rows then scrolls, and
  Markdown list items/quotes wrap inside the available width (they used to
  extend past the bubble and get clipped).
- **0.4.0** — **renamed from `egui_chat` to `ragrig-egui`**: the name
  now says what the crate is for. Also: the dock collapses **horizontally** — a
  collapsed pane becomes a narrow strip at its window edge with one control to
  expand it again (host-owned flag, reported via `DockResponse::collapsed`).
  The old per-section fold — headers stacked, bodies hidden — is gone with
  `SectionState::collapsed` (breaking).
- **0.5.0** (current) — the **workflow widgets** a ragrig panel needs around
  the composer, adopted from the sciscraper host where they were prototyped:
  `attachments` (pending-context chips: toggle/pending/offer, disabled chips
  explain themselves in their tooltip) and `sessions` (the conversation picker
  with host-localized labels and a busy state). The host keeps the state and
  the labels; the crate renders and reports.
- **0.6** — message actions (copy feedback, retry, delete), clickable links,
  draggable splitter between dock sections; index housekeeping controls
  (*Update*/*Rebuild* + index state note) and a source-list widget with
  hover row actions — the remaining sciscraper prototypes once their API has
  settled.
- **0.7** — tabbed dock mode for hosts that need more than two panes; theme
  presets; publish to crates.io.

## Compatibility

- `egui` 0.34, Rust 1.92 (edition 2024).
- 0.4.0 renamed the package (`egui_chat` → `ragrig-egui`); the module paths are
  unchanged apart from the crate name.
- 0.5.0 is additive: the new modules `attachments` and `sessions` sit next to
  the existing widgets, nothing was removed.

## License

MIT — see `LICENSE`.
