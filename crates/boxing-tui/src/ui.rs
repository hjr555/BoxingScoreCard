use crate::app::{App, FinishField, FormField, Screen};
use boxing_core::{FighterSide, TimerPhase};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Gauge, List, ListItem, Paragraph, Row, Table},
    Frame,
};

pub fn draw(f: &mut Frame, app: &mut App) {
    match app.screen {
        Screen::Home => draw_home(f, app),
        Screen::NewMatch => {
            draw_home(f, app);
            draw_new_match_modal(f, app);
        }
        Screen::Scoring => draw_scoring(f, app),
        Screen::EditBoxRec => {
            if app.previous_screen == Screen::Scoring {
                draw_scoring(f, app);
            } else {
                draw_home(f, app);
            }
            draw_boxrec_modal(f, app);
        }
        Screen::FinishMatch => {
            if app.previous_screen == Screen::Scoring {
                draw_scoring(f, app);
            } else {
                draw_home(f, app);
            }
            draw_finish_match_modal(f, app);
        }
    }
}

fn draw_home(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(8),
            Constraint::Length(3),
            Constraint::Length(2),
        ])
        .split(f.area());

    let title = Paragraph::new(Line::from(vec![
        Span::styled(
            " 🥊 BOXING SCORECARD ",
            Style::default()
                .fg(Color::Black)
                .bg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("  Minimal-Input Round-by-Round Match Scorer"),
    ]))
    .block(Block::default().borders(Borders::ALL));
    f.render_widget(title, chunks[0]);

    let items: Vec<ListItem> = if app.matches.is_empty() {
        vec![ListItem::new(Line::from(Span::styled(
            "  No saved matches yet. Press [N] or [Enter] to create a new match!",
            Style::default().fg(Color::DarkGray),
        )))]
    } else {
        app.matches
            .iter()
            .enumerate()
            .map(|(idx, m)| {
                let (tot_a, tot_b) = m.total_scores();
                let status = if let Some(summary) = m.outcome_summary() {
                    format!("FINAL: {}", summary)
                } else {
                    match &m.active_round {
                        Some(r) if r.is_walkout() => "Walkout".to_string(),
                        Some(r) => format!("Round {}/{}", r.round_number, m.scheduled_rounds),
                        None => "FINAL".to_string(),
                    }
                };
                let walkout_info = match &m.walkout_score {
                    Some(w) => format!("Walkout: {}-{}", w.fighter_a_score, w.fighter_b_score),
                    None => "Walkout: —".to_string(),
                };
                let fa_display = match &m.fighter_a_boxrec_id {
                    Some(id) => format!("{} (#{})", m.fighter_a, id),
                    None => m.fighter_a.clone(),
                };
                let fb_display = match &m.fighter_b_boxrec_id {
                    Some(id) => format!("{} (#{})", m.fighter_b, id),
                    None => m.fighter_b.clone(),
                };
                let line = format!(
                    " {:<32} | {:<22} | {:<13} | Cards: {} {} - {} {}",
                    m.title(),
                    status,
                    walkout_info,
                    fa_display,
                    tot_a,
                    tot_b,
                    fb_display
                );
                let style = if idx == app.selected_match_idx {
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::Cyan)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };
                ListItem::new(line).style(style)
            })
            .collect()
    };

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Saved Matches (JSON Backend) — [↑/↓] or [J/K] Select "),
    );
    f.render_widget(list, chunks[1]);

    let btn = Paragraph::new(" [ + NEW MATCH (Press N) ] ")
        .alignment(Alignment::Center)
        .style(
            Style::default()
                .fg(Color::Black)
                .bg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )
        .block(Block::default().borders(Borders::NONE));
    f.render_widget(btn, chunks[2]);

    let footer = Paragraph::new(format!(
        " {}  |  [N] New  [Enter] Open  [F] Stoppage/Finish  [B] BoxRec IDs  [D] Delete  [Q] Quit",
        app.status_message
    ))
    .style(Style::default().fg(Color::Gray));
    f.render_widget(footer, chunks[3]);
}

fn draw_new_match_modal(f: &mut Frame, app: &mut App) {
    let area = centered_rect(65, 65, f.area());
    f.render_widget(Clear, area);

    let block = Block::default()
        .title(" Create New Boxing Match ")
        .borders(Borders::ALL)
        .style(Style::default().bg(Color::Black).fg(Color::White));
    f.render_widget(block, area);

    let inner = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Min(2),
        ])
        .split(area);

    let field_style = |field: FormField| {
        if app.form_field == field {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        }
    };

    let fa = Paragraph::new(app.input_fighter_a.as_str()).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Fighter 1 (Red Corner) ")
            .border_style(field_style(FormField::FighterA)),
    );
    f.render_widget(fa, inner[0]);

    let fb = Paragraph::new(app.input_fighter_b.as_str()).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Fighter 2 (Blue Corner) ")
            .border_style(field_style(FormField::FighterB)),
    );
    f.render_widget(fb, inner[1]);

    let dt = Paragraph::new(app.input_date.as_str()).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Match Date (e.g. 01 Jan 2026) ")
            .border_style(field_style(FormField::Date)),
    );
    f.render_widget(dt, inner[2]);

    let rds = Paragraph::new(format!(
        "{} rounds (Use [←/→] or [-/+] to adjust)",
        app.input_rounds
    ))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Scheduled Rounds ")
            .border_style(field_style(FormField::Rounds)),
    );
    f.render_widget(rds, inner[3]);

    let preview = Paragraph::new(Line::from(vec![
        Span::styled("Formatted Title: ", Style::default().fg(Color::Gray)),
        Span::styled(
            app.preview_title(),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
    ]))
    .block(Block::default().borders(Borders::ALL).title(" Preview "));
    f.render_widget(preview, inner[4]);

    let help = Paragraph::new(" [Tab / ↑ / ↓] Switch Field   [Enter] Start Match   [Esc] Cancel")
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::Green));
    f.render_widget(help, inner[5]);
}

fn draw_boxrec_modal(f: &mut Frame, app: &mut App) {
    let area = centered_rect(60, 48, f.area());
    f.render_widget(Clear, area);

    let block = Block::default()
        .title(format!(
            " Optional BoxRec IDs — {} ",
            app.boxrec_target_title
        ))
        .borders(Borders::ALL)
        .style(Style::default().bg(Color::Black).fg(Color::White));
    f.render_widget(block, area);

    let inner = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Min(2),
        ])
        .split(area);

    let style_a = if app.boxrec_side == FighterSide::FighterA {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::White)
    };
    let style_b = if app.boxrec_side == FighterSide::FighterB {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::White)
    };

    let fa_boxrec = Paragraph::new(app.input_boxrec_a.as_str()).block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!(
                " {} — BoxRec ID (e.g. 659461, leave blank to clear) ",
                app.boxrec_fighter_a_name
            ))
            .border_style(style_a),
    );
    f.render_widget(fa_boxrec, inner[0]);

    let fb_boxrec = Paragraph::new(app.input_boxrec_b.as_str()).block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!(
                " {} — BoxRec ID (e.g. 479205, leave blank to clear) ",
                app.boxrec_fighter_b_name
            ))
            .border_style(style_b),
    );
    f.render_widget(fb_boxrec, inner[1]);

    let help = Paragraph::new(" [Tab / ↑ / ↓] Switch Fighter   [Enter] Save IDs   [Esc] Cancel")
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::Green));
    f.render_widget(help, inner[2]);
}

fn draw_finish_match_modal(f: &mut Frame, app: &mut App) {
    let area = centered_rect(65, 58, f.area());
    f.render_widget(Clear, area);

    let block = Block::default()
        .title(format!(
            " Stoppage / Complete Fight — {} ",
            app.finish_target_title
        ))
        .borders(Borders::ALL)
        .style(Style::default().bg(Color::Black).fg(Color::White));
    f.render_widget(block, area);

    let inner = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Min(2),
        ])
        .split(area);

    let field_style = |field: FinishField| {
        if app.finish_field == field {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        }
    };

    let method_text = format!(
        "◄  {}  ►   (Use [←/→] to change)",
        app.finish_method.description()
    );
    let method_box = Paragraph::new(method_text).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" 1. How Did the Fight End? (TKO / Towel RTD / KO / DQ / Decision) ")
            .border_style(field_style(FinishField::Method)),
    );
    f.render_widget(method_box, inner[0]);

    let winner_name = match app.finish_winner {
        Some(FighterSide::FighterA) => format!("{} (Red Corner)", app.finish_fighter_a_name),
        Some(FighterSide::FighterB) => format!("{} (Blue Corner)", app.finish_fighter_b_name),
        None => "Draw / Scorecard Only".to_string(),
    };
    let winner_text = format!("◄  {}  ►   (Use [←/→] to change)", winner_name);
    let winner_box = Paragraph::new(winner_text).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" 2. Winner ")
            .border_style(field_style(FinishField::Winner)),
    );
    f.render_widget(winner_box, inner[1]);

    let round_text = format!(
        "◄  Round {} of {}  ►   (Use [←/→] to change)",
        app.finish_round, app.finish_max_rounds
    );
    let round_box = Paragraph::new(round_text).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" 3. Ended in Round ")
            .border_style(field_style(FinishField::Round)),
    );
    f.render_widget(round_box, inner[2]);

    let help = Paragraph::new(
        " [Tab / ↑ / ↓] Switch Field   [← / →] Change Value   [Enter] Mark Fight Complete   [Esc] Cancel",
    )
    .alignment(Alignment::Center)
    .style(Style::default().fg(Color::Green));
    f.render_widget(help, inner[3]);
}

fn draw_scoring(f: &mut Frame, app: &mut App) {
    let Some(ref m) = app.current_match else {
        return;
    };

    let root = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3), // Match Title Header
            Constraint::Length(4), // Loose Timer & Alert Banner
            Constraint::Min(10),   // Fighter Cards + Scorecard Table
            Constraint::Length(3), // Bottom LOCK ROUND IN Button
            Constraint::Length(2), // Status & Controls Bar
        ])
        .split(f.area());

    // 1. Header: Match Title `Joshua v Fury (01 Jan 2026)` & Running Total
    let (tot_a, tot_b) = m.total_scores();
    let is_walkout_stage = m
        .active_round
        .as_ref()
        .map(|r| r.is_walkout())
        .unwrap_or(false);

    let (round_status, badge_bg) = if let Some(summary) = m.outcome_summary() {
        (format!("🏆 FINAL: {}", summary), Color::Green)
    } else {
        match &m.active_round {
            Some(active) if active.is_walkout() => (
                "🎵 PRE-FIGHT WALKOUT (FUN ROUND)".to_string(),
                Color::Magenta,
            ),
            Some(active) => (
                format!("ROUND {} OF {}", active.round_number, m.scheduled_rounds),
                Color::Blue,
            ),
            None => ("FINAL SCORECARD (COMPLETE)".to_string(), Color::DarkGray),
        }
    };

    let header = Paragraph::new(Line::from(vec![
        Span::styled(
            format!(" {} ", m.title()),
            Style::default()
                .fg(Color::Black)
                .bg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("   "),
        Span::styled(
            format!(" {} ", round_status),
            Style::default()
                .fg(if badge_bg == Color::Green {
                    Color::Black
                } else {
                    Color::White
                })
                .bg(badge_bg)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(format!(
            "   Official Fight Score: {} {} - {} {}",
            m.fighter_a, tot_a, tot_b, m.fighter_b
        )),
    ]))
    .block(Block::default().borders(Borders::ALL));
    f.render_widget(header, root[0]);

    // 2. Timer Banner (Loose prompts for round starts and breaks)
    if let Some(ref timer) = app.timer {
        let total_dur = match timer.phase {
            TimerPhase::InRound { .. } | TimerPhase::Ready { .. } => {
                timer.round_duration.as_secs_f64()
            }
            TimerPhase::InBreak { .. } => timer.break_duration.as_secs_f64(),
            TimerPhase::Completed => 1.0,
        };
        let rem = timer.remaining.as_secs_f64();
        let ratio = if total_dur > 0.0 {
            (rem / total_dur).clamp(0.0, 1.0)
        } else {
            0.0
        };

        let bar_color = if app.is_alert_active() {
            Color::LightRed
        } else {
            match timer.phase {
                TimerPhase::InRound { .. } => Color::Green,
                TimerPhase::InBreak { .. } => Color::Cyan,
                TimerPhase::Ready { .. } => Color::Magenta,
                TimerPhase::Completed => Color::DarkGray,
            }
        };

        let pause_tag = if timer.paused && timer.phase != TimerPhase::Completed {
            " [PAUSED]"
        } else {
            ""
        };
        let prompt_text = timer
            .last_prompt
            .as_ref()
            .map(|p| p.message())
            .unwrap_or_else(|| {
                if let Some(summary) = m.outcome_summary() {
                    format!("Fight Complete — {}", summary)
                } else if is_walkout_stage {
                    "Score the pre-fight walkout! Round 1 timer starts when locked in.".to_string()
                } else {
                    String::new()
                }
            });

        let label = format!(
            "{} — {}{}  |  {}",
            timer.phase_label(),
            timer.formatted_remaining(),
            pause_tag,
            prompt_text
        );

        let gauge = Gauge::default()
            .block(Block::default().borders(Borders::ALL).title(
                " Match Timer (Loose Prompt — [P] Pause/Resume, [T] Next Phase, [F] Stoppage/Finish) ",
            ))
            .gauge_style(
                Style::default()
                    .fg(bar_color)
                    .bg(Color::Black)
                    .add_modifier(Modifier::BOLD),
            )
            .ratio(ratio)
            .label(label);
        f.render_widget(gauge, root[1]);
    }

    // 3. Main Area: Left/Center Fighter Score Cards + Right Round-by-Round Scorecard
    let main_split = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(68), Constraint::Percentage(32)])
        .split(root[2]);

    let cards_split = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(main_split[0]);

    let (score_a, score_b) = m
        .active_round
        .as_ref()
        .map(|r| (r.fighter_a_score, r.fighter_b_score))
        .unwrap_or((0, 0));

    let points_caption = if is_walkout_stage {
        "🎵 WALKOUT POINTS (FUN)"
    } else if m.active_round.is_some() {
        "POINTS THIS ROUND"
    } else {
        "MATCH COMPLETE"
    };

    draw_fighter_card(
        f,
        cards_split[0],
        &m.fighter_a,
        m.fighter_a_boxrec_id.as_deref(),
        score_a,
        m.active_round.is_some(),
        points_caption,
        Color::LightRed,
        "Deduct (-1): [1] / [←] / [A]   |   Restore (+1): [!] / [W]",
    );

    draw_fighter_card(
        f,
        cards_split[1],
        &m.fighter_b,
        m.fighter_b_boxrec_id.as_deref(),
        score_b,
        m.active_round.is_some(),
        points_caption,
        Color::LightBlue,
        "Deduct (-1): [2] / [→] / [D]   |   Restore (+1): [@] / [E]",
    );

    // Scorecard Table on the right
    draw_scorecard_table(f, main_split[1], m);

    // 4. Bottom Button: Lock the Walkout or Round In
    let (btn_text, btn_style) = match &m.active_round {
        Some(active) if active.is_walkout() => (
            format!(
                " 🎵 LOCK WALKOUT SCORE IN ({} {} - {} {}) & START ROUND 1 — [Press Enter or Space] ",
                m.fighter_a,
                active.fighter_a_score,
                active.fighter_b_score,
                m.fighter_b
            ),
            Style::default()
                .fg(Color::White)
                .bg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
        ),
        Some(active) => (
            format!(
                " 🔒 LOCK ROUND {} IN ({} {} - {} {}) — [Enter/Space]   |   🛑 STOPPAGE / FINISH FIGHT — [Press F] ",
                active.round_number,
                m.fighter_a,
                active.fighter_a_score,
                active.fighter_b_score,
                m.fighter_b
            ),
            Style::default()
                .fg(Color::Black)
                .bg(Color::Green)
                .add_modifier(Modifier::BOLD),
        ),
        None => {
            let summary = m
                .outcome_summary()
                .unwrap_or_else(|| "ALL ROUNDS LOCKED".to_string());
            (
                format!(
                    " ✅ MATCH COMPLETE ({}) — [F] Edit Result  [U] Reopen Round  [Esc] Menu ",
                    summary
                ),
                Style::default()
                    .fg(Color::White)
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            )
        }
    };

    let lock_btn = Paragraph::new(btn_text)
        .alignment(Alignment::Center)
        .style(btn_style)
        .block(Block::default().borders(Borders::NONE));
    f.render_widget(lock_btn, root[3]);

    // 5. Status & Help Footer
    let footer = Paragraph::new(format!(
        " {}  |  [F] Stoppage/Finish  [0] Walkout  [B] BoxRec  [U] Undo  [Esc] Menu",
        app.status_message
    ))
    .style(Style::default().fg(Color::Gray));
    f.render_widget(footer, root[4]);
}

#[allow(clippy::too_many_arguments)]
fn draw_fighter_card(
    f: &mut Frame,
    area: Rect,
    name: &str,
    boxrec_id: Option<&str>,
    score: u8,
    is_active: bool,
    points_caption: &str,
    accent: Color,
    hint: &str,
) {
    let title_str = match boxrec_id {
        Some(id) => format!(" {} (BoxRec: #{}) ", name.to_uppercase(), id),
        None => format!(" {} ", name.to_uppercase()),
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(title_str)
        .border_style(Style::default().fg(accent));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Min(5),
            Constraint::Length(2),
        ])
        .split(inner);

    let subtitle = match boxrec_id {
        Some(id) => format!("{}  [BoxRec #{}]", name, id),
        None => name.to_string(),
    };

    let header = Paragraph::new(subtitle)
        .alignment(Alignment::Center)
        .style(Style::default().fg(accent).add_modifier(Modifier::BOLD));
    f.render_widget(header, rows[0]);

    let score_display = if is_active {
        format!("{}", score)
    } else {
        "—".to_string()
    };

    let big_score = Paragraph::new(vec![
        Line::from(""),
        Line::from(Span::styled(
            format!("  {}  ", score_display),
            Style::default()
                .fg(Color::Black)
                .bg(accent)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            points_caption,
            Style::default().fg(Color::White),
        )),
    ])
    .alignment(Alignment::Center);
    f.render_widget(big_score, rows[1]);

    let instructions = Paragraph::new(hint)
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::Gray));
    f.render_widget(instructions, rows[2]);
}

fn draw_scorecard_table(f: &mut Frame, area: Rect, m: &boxing_core::BoxingMatch) {
    let mut rows: Vec<Row> = Vec::new();

    // Pre-fight Walkout fun row at the very top before Round 1
    match &m.walkout_score {
        Some(w) => {
            rows.push(
                Row::new(vec![
                    "🎵 Walkout".to_string(),
                    format!("{}", w.fighter_a_score),
                    format!("{}", w.fighter_b_score),
                ])
                .style(
                    Style::default()
                        .fg(Color::LightMagenta)
                        .add_modifier(Modifier::ITALIC | Modifier::BOLD),
                ),
            );
        }
        None => {
            let (wa, wb) = m
                .active_round
                .as_ref()
                .filter(|r| r.is_walkout())
                .map(|r| {
                    (
                        format!("{}*", r.fighter_a_score),
                        format!("{}*", r.fighter_b_score),
                    )
                })
                .unwrap_or_else(|| ("—".to_string(), "—".to_string()));
            rows.push(
                Row::new(vec!["🎵 Walkout".to_string(), wa, wb]).style(
                    Style::default()
                        .fg(Color::Magenta)
                        .add_modifier(Modifier::ITALIC),
                ),
            );
        }
    }

    for r in &m.locked_rounds {
        let style = if r.fighter_a_score > r.fighter_b_score {
            Style::default().fg(Color::LightRed)
        } else if r.fighter_b_score > r.fighter_a_score {
            Style::default().fg(Color::LightBlue)
        } else {
            Style::default().fg(Color::Yellow)
        };
        rows.push(
            Row::new(vec![
                format!("R{}", r.round_number),
                format!("{}", r.fighter_a_score),
                format!("{}", r.fighter_b_score),
            ])
            .style(style),
        );
    }

    if let Some(out) = &m.outcome {
        let (col_a, col_b) = match out.winner {
            Some(FighterSide::FighterA) => (
                format!("WIN ({})", out.method.short_label()),
                format!("R{}", out.round),
            ),
            Some(FighterSide::FighterB) => (
                format!("R{}", out.round),
                format!("WIN ({})", out.method.short_label()),
            ),
            None => (
                out.method.short_label().to_string(),
                format!("R{}", out.round),
            ),
        };
        rows.push(
            Row::new(vec!["🛑 Result".to_string(), col_a, col_b]).style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
        );
    }

    let (tot_a, tot_b) = m.total_scores();
    rows.push(
        Row::new(vec![
            "TOTAL".to_string(),
            format!("{}", tot_a),
            format!("{}", tot_b),
        ])
        .style(
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        ),
    );

    let table = Table::new(
        rows,
        [
            Constraint::Percentage(36),
            Constraint::Percentage(32),
            Constraint::Percentage(32),
        ],
    )
    .header(
        Row::new(vec!["Stage", &m.fighter_a, &m.fighter_b]).style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
    )
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Scorecard (Walkout + Official) "),
    );

    f.render_widget(table, area);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
