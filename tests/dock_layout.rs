//! Layout contract for hosts that stack "transcript above a footer".
//!
//! Immediate mode knows a widget's height only after drawing it, so the
//! recipe is *rect-based* rather than measurement-based:
//!
//! 1. reserve the library's share at the bottom of the section,
//! 2. draw the footer (agent bar + composer) bottom-up in a rect ending above
//!    that share — the composer grows upward as the prompt does,
//! 3. put the transcript in the space between the session row and the footer,
//! 4. advance the cursor to the footer's bottom so the next section follows.
//!
//! These tests pin the promise: a multi-line prompt neither pushes the agent
//! bar out of the panel nor hides the section below it, and the transcript
//! never overlaps the footer.

use std::cell::RefCell;

use egui::{Pos2, Rect, Vec2};
use ragrig_egui::agent_bar::{AgentBar, AgentBarStyle, Component, ComponentStatus};
use ragrig_egui::chat_view::{ChatMessage, ChatView};
use ragrig_egui::composer::{Composer, max_composer_height};
use ragrig_egui::dock::{DockSide, SectionDock, SectionState};

const MIN_TRANSCRIPT: f32 = 120.0;

#[derive(Debug, Default, Clone, Copy)]
struct Geometry {
    panel_bottom: f32,
    transcript_bottom: f32,
    footer_top: f32,
    footer_bottom: f32,
    library_top: f32,
}

fn run_frame(
    ctx: &egui::Context,
    state: &mut SectionState,
    library_state: &mut SectionState,
    input: &mut String,
    open_menu: &mut Option<String>,
) -> Geometry {
    let geometry = RefCell::new(Geometry::default());
    let input_raw = egui::RawInput {
        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1280.0, 800.0))),
        ..egui::RawInput::default()
    };
    let _ = ctx.run_ui(input_raw, |ui| {
        let components = [Component::new("chat", "Chat", "C").status(ComponentStatus::Ready)];
        SectionDock::new("footer_contract_dock", DockSide::Right, 380.0)
            .section(state, |ui| {
                let panel = ui.max_rect();
                geometry.borrow_mut().panel_bottom = panel.bottom();
                ui.label("session row");
                let top = ui.cursor().min.y;
                let left = ui.available_rect_before_wrap().left();
                let right = ui.available_rect_before_wrap().right();

                // The library below keeps a usable share.
                let library_reserve = 40.0 + 3.0 * 20.0;
                let footer_min = max_composer_height(ui)
                    + AgentBarStyle::default().height
                    + 4.0 * ui.spacing().item_spacing.y;
                let bottom =
                    (panel.bottom() - library_reserve).max(top + MIN_TRANSCRIPT + footer_min);

                // 1. Footer, anchored to the bottom of its rect.
                let footer_rect = Rect::from_min_max(
                    Pos2::new(left, top + MIN_TRANSCRIPT),
                    Pos2::new(right, bottom),
                );
                let mut footer_ui = ui.new_child(
                    egui::UiBuilder::new()
                        .max_rect(footer_rect)
                        .layout(egui::Layout::bottom_up(egui::Align::Min)),
                );
                AgentBar::new(&components, open_menu).show(&mut footer_ui, |_, _| false);
                let after_bar = footer_ui.min_rect();
                let _ = Composer::new(input).show(&mut footer_ui);
                println!(
                    "footer_rect={:?} after_bar={:?} min_rect={:?}",
                    footer_rect,
                    after_bar,
                    footer_ui.min_rect()
                );
                let (footer_top, footer_bottom) =
                    (footer_ui.min_rect().top(), footer_ui.min_rect().bottom());
                {
                    let mut g = geometry.borrow_mut();
                    g.footer_top = footer_top;
                    g.footer_bottom = footer_bottom;
                }

                // 2. Transcript above the footer.
                let transcript_bottom =
                    (footer_top - ui.spacing().item_spacing.y).max(top + MIN_TRANSCRIPT);
                let transcript_rect =
                    Rect::from_min_max(Pos2::new(left, top), Pos2::new(right, transcript_bottom));
                ui.scope_builder(egui::UiBuilder::new().max_rect(transcript_rect), |ui| {
                    ChatView::new(&messages()).markdown(true).show(ui);
                });
                geometry.borrow_mut().transcript_bottom = transcript_rect.bottom();

                // 3. The section ends where the footer ends.
                ui.allocate_rect(
                    Rect::from_min_max(Pos2::new(left, top), Pos2::new(right, bottom)),
                    egui::Sense::hover(),
                );
            })
            .section(library_state, |ui| {
                geometry.borrow_mut().library_top = ui.cursor().min.y;
                ui.label("library.bib");
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show_rows(ui, 26.0, 3, |ui, range| {
                        for i in range {
                            ui.label(format!("row {i}"));
                        }
                    });
            })
            .show(ui);
    });
    geometry.into_inner()
}

fn messages() -> Vec<ChatMessage> {
    vec![
        ChatMessage::user("short question"),
        ChatMessage::assistant(
            "An answer with a paragraph that wraps, and a list:\n\n\
             - a bullet long enough to wrap several times inside a narrow dock\n\
             - another bullet",
        ),
    ]
}

fn check(rows: usize) {
    let ctx = egui::Context::default();
    let mut state = SectionState::new("Agent");
    let mut library_state = SectionState::new("Library");
    let mut input = "line\n".repeat(rows);
    let mut open_menu = None;
    let _ = run_frame(
        &ctx,
        &mut state,
        &mut library_state,
        &mut input,
        &mut open_menu,
    );
    let geometry = run_frame(
        &ctx,
        &mut state,
        &mut library_state,
        &mut input,
        &mut open_menu,
    );

    assert!(
        geometry.transcript_bottom <= geometry.footer_top + 1.0,
        "{rows} rows: transcript ({}) overlaps the footer ({})",
        geometry.transcript_bottom,
        geometry.footer_top
    );
    assert!(
        geometry.footer_bottom <= geometry.panel_bottom + 1.0,
        "{rows} rows: footer ends at {} but the panel ends at {}",
        geometry.footer_bottom,
        geometry.panel_bottom
    );
    assert!(
        geometry.library_top <= geometry.panel_bottom,
        "{rows} rows: the section below was pushed out (library at {}, panel ends at {})",
        geometry.library_top,
        geometry.panel_bottom
    );
}

#[test]
fn a_short_prompt_keeps_everything_visible() {
    check(1);
}

#[test]
fn a_multi_line_prompt_keeps_everything_visible() {
    // Regression: an unbounded composer used to push the agent bar and the
    // section below out of the panel.
    check(10);
}
