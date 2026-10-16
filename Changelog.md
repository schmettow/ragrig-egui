# Changelog

All notable changes to ragrig-egui are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## 0.5.0 — 2026-10-16 — first crates.io release

First publication. The 0.1–0.5 line was developed in-tree with the sciscraper
agent pane; 0.5.0 is the API as first published.

### Added

- `dock::SectionDock` — side panel (left/right) with stacked sections, a
  persisted width, a horizontal collapse to a strip at the window edge, and an
  optional close button.
- `chat_view::ChatView` — scrollable transcript of `ChatMessage`s:
  user/assistant/system bubbles, streaming and error states, metadata footer
  (`12 chunks · 3 sources · 8.2 s`).
- `composer::Composer` — multiline input with **Return = newline**,
  **Ctrl/⌘+Return = submit**, send/stop buttons and capped growth;
  `composer::append` routes picked text into the prompt (keeps focus, caret
  after the text).
- `attachments` — pending-context chip rows above the composer: toggles, a
  pending context with a ✕, and an offer with an action button.
- `sessions::SessionRow` — conversation picker: *New*, a dropdown of saved
  sessions, and a refresh action.
- `agent_bar::AgentBar` — one icon per agent component with status dots
  (grey/green/pulsing/red) and a slide-up menu drawn by the host.
- `form` — setting rows (`setting_row`) and note/warning lines for dense
  component menus.
- `markdown` (feature, default) — light Markdown rendering for assistant
  bubbles via pulldown-cmark.

### Quality

- `cargo check --all-targets --all-features` clean; 57 tests green
  (46 unit, 2 + 9 integration) on Rust 1.92.0.
- The crate links neither `ragrig` nor an async runtime — `egui` is the only
  dependency; the host owns state and worker, the widgets render what it hands
  them.
