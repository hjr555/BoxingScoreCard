use crate::app::{App, FinishField, FormField, Screen};
use boxing_core::{FighterSide, TimerPhase};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, Borders, Clear, Gauge, List, ListItem, ListState, Paragraph, Row, Table, TableState,
    },
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

    let title = Paragraph::new(Line::from(vec![Span::styled(
        " 🥊 BOXING SCORECARD ",
        Style::default()
            .fg(Color::Black)
            .bg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    )]))
    .block(Block::default().borders(Borders::ALL));
    f.render_widget(title, chunks[0]);

    let items: Vec<ListItem> = if app.matches.is_empty() {
        vec![ListItem::new(Line::from(Span::styled(
            "  No matches yet — [N]ew match",
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
                    " {:<32} | {:<22} | {:<13} | {} {} - {} {}",
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
            .title(" Matches "),
    );
    let mut list_state = ListState::default();
    if !app.matches.is_empty() {
        list_state.select(Some(app.selected_match_idx));
    }
    f.render_stateful_widget(list, chunks[1], &mut list_state);

    let btn = Paragraph::new(" + [N]ew Match ")
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
        " {}  |  [N]ew  [Enter] Open  [F]inish  [B]oxRec  [D]elete  [Q]uit",
        app.status_message
    ))
    .style(Style::default().fg(Color::Gray));
    f.render_widget(footer, chunks[3]);
}

fn draw_new_match_modal(f: &mut Frame, app: &mut App) {
    let area = centered_rect(60, 62, f.area());
    f.render_widget(Clear, area);

    let block = Block::default()
        .title(" New Match ")
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
            .title(" Date ")
            .border_style(field_style(FormField::Date)),
    );
    f.render_widget(dt, inner[2]);

    let rds = Paragraph::new(format!("{}  [←/→]", app.input_rounds)).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Rounds ")
            .border_style(field_style(FormField::Rounds)),
    );
    f.render_widget(rds, inner[3]);

    let preview = Paragraph::new(Line::from(vec![Span::styled(
        app.preview_title(),
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )]))
    .block(Block::default().borders(Borders::ALL).title(" Match "));
    f.render_widget(preview, inner[4]);

    let help = Paragraph::new("[Tab/↑/↓] Field   [Enter] Start   [Esc] Cancel")
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::Green));
    f.render_widget(help, inner[5]);
}

fn draw_boxrec_modal(f: &mut Frame, app: &mut App) {
    let area = centered_rect(56, 45, f.area());
    f.render_widget(Clear, area);

    let block = Block::default()
        .title(format!(" BoxRec IDs — {} ", app.boxrec_target_title))
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
            .title(format!(" {} ", app.boxrec_fighter_a_name))
            .border_style(style_a),
    );
    f.render_widget(fa_boxrec, inner[0]);

    let fb_boxrec = Paragraph::new(app.input_boxrec_b.as_str()).block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!(" {} ", app.boxrec_fighter_b_name))
            .border_style(style_b),
    );
    f.render_widget(fb_boxrec, inner[1]);

    let help = Paragraph::new("[Tab/↑/↓] Switch   [Enter] Save   [Esc] Cancel")
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::Green));
    f.render_widget(help, inner[2]);
}

fn draw_finish_match_modal(f: &mut Frame, app: &mut App) {
    let area = centered_rect(68, 65, f.area());
    f.render_widget(Clear, area);

    let block = Block::default()
        .title(format!(" Finish Match — {} ", app.finish_target_title))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::LightRed))
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
        "◄  {}  ►   |  [T]KO  [K]O  [R]TD  [Q] DQ  [D]ec/TD",
        app.finish_method.description()
    );
    let method_box = Paragraph::new(method_text).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Method [←/→] ")
            .border_style(field_style(FinishField::Method)),
    );
    f.render_widget(method_box, inner[0]);

    let winner_name = match app.finish_winner {
        Some(FighterSide::FighterA) => format!("{} (Red)", app.finish_fighter_a_name),
        Some(FighterSide::FighterB) => format!("{} (Blue)", app.finish_fighter_b_name),
        None => "Draw".to_string(),
    };
    let winner_text = format!(
        "◄  {}  ►   |  [1] {}  [2] {}  [0] Draw",
        winner_name, app.finish_fighter_a_name, app.finish_fighter_b_name
    );
    let winner_box = Paragraph::new(winner_text).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Winner [←/→] ")
            .border_style(field_style(FinishField::Winner)),
    );
    f.render_widget(winner_box, inner[1]);

    let round_text = format!(
        "◄  Round {} of {}  ►   |  [-/+] Round",
        app.finish_round, app.finish_max_rounds
    );
    let round_box = Paragraph::new(round_text).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Round [←/→] ")
            .border_style(field_style(FinishField::Round)),
    );
    f.render_widget(round_box, inner[2]);

    let preview = Paragraph::new(Line::from(vec![
        Span::styled(
            format!(" 🏆 {} ", app.preview_finish_outcome()),
            Style::default()
                .fg(Color::Black)
                .bg(Color::Green)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(format!("   |   Cards: {}", app.finish_cards_summary)),
    ]))
    .block(Block::default().borders(Borders::ALL).title(" Result "));
    f.render_widget(preview, inner[3]);

    let help = Paragraph::new("[Enter] Confirm   [Tab/↑/↓] Field   [Esc] Cancel")
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::Green).add_modifier(Modifier::BOLD));
    f.render_widget(help, inner[4]);
}

fn draw_scoring(f: &mut Frame, app: &mut App) {
    let Some(ref m) = app.current_match else {
        return;
    };

    let root = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3), // Match Header
            Constraint::Length(4), // Timer Banner
            Constraint::Min(10),   // Fighter Cards + Scorecard Table
            Constraint::Length(3), // Action Bar
            Constraint::Length(2), // Status & Controls Bar
        ])
        .split(f.area());

    // 1. Header
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
            Some(active) if active.is_walkout() => ("🎵 WALKOUT".to_string(), Color::Magenta),
            Some(active) => (
                format!("ROUND {} / {}", active.round_number, m.scheduled_rounds),
                Color::Blue,
            ),
            None => ("FINAL".to_string(), Color::DarkGray),
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
            "   Score: {} {} - {} {}",
            m.fighter_a, tot_a, tot_b, m.fighter_b
        )),
    ]))
    .block(Block::default().borders(Borders::ALL));
    f.render_widget(header, root[0]);

    // 2. Timer Banner
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
                    summary
                } else if is_walkout_stage {
                    "Walkout".to_string()
                } else {
                    String::new()
                }
            });

        let label = if prompt_text.is_empty() {
            format!(
                "{} — {}{}",
                timer.phase_label(),
                timer.formatted_remaining(),
                pause_tag
            )
        } else {
            format!(
                "{} — {}{}  |  {}",
                timer.phase_label(),
                timer.formatted_remaining(),
                pause_tag,
                prompt_text
            )
        };

        let gauge = Gauge::default()
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Timer — [P]ause  [T] Next Phase "),
            )
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

    // 3. Main Area: Fighter Score Cards + Scorecard Table
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
        "WALKOUT"
    } else if m.active_round.is_some() {
        "ROUND SCORE"
    } else {
        "FINAL"
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
        "[-1] 1 / ← / A   [+1] ! / W",
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
        "[-1] 2 / → / D   [+1] @ / E",
    );

    draw_scorecard_table(f, main_split[1], m);

    // 4. Bottom Action Bar
    match &m.active_round {
        Some(active) => {
            let action_split = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Percentage(54),
                    Constraint::Length(1),
                    Constraint::Percentage(45),
                ])
                .split(root[3]);

            let (lock_text, lock_style) = if active.is_walkout() {
                (
                    format!(
                        " [Enter] Lock Walkout ({}-{}) ",
                        active.fighter_a_score, active.fighter_b_score
                    ),
                    Style::default()
                        .fg(Color::White)
                        .bg(Color::Magenta)
                        .add_modifier(Modifier::BOLD),
                )
            } else {
                (
                    format!(
                        " [Enter] Lock Round {} ({}-{}) ",
                        active.round_number, active.fighter_a_score, active.fighter_b_score
                    ),
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                )
            };

            let lock_btn = Paragraph::new(lock_text)
                .alignment(Alignment::Center)
                .style(lock_style);
            f.render_widget(lock_btn, action_split[0]);

            let stop_btn = Paragraph::new(" [F]inish Fight ([K]O / [R]TD) ")
                .alignment(Alignment::Center)
                .style(
                    Style::default()
                        .fg(Color::White)
                        .bg(Color::Red)
                        .add_modifier(Modifier::BOLD),
                );
            f.render_widget(stop_btn, action_split[2]);
        }
        None => {
            let summary = m
                .outcome_summary()
                .unwrap_or_else(|| "COMPLETE".to_string());
            let done_btn = Paragraph::new(format!(
                " ✅ {}  —  [F]inish  [U]ndo  [Esc] Matches ",
                summary
            ))
            .alignment(Alignment::Center)
            .style(
                Style::default()
                    .fg(Color::White)
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            );
            f.render_widget(done_btn, root[3]);
        }
    }

    // 5. Status & Help Footer
    let footer = Paragraph::new(format!(
        " {}  |  [F]inish  [0] Walkout  [B]oxRec  [U]ndo  [P]ause  [Esc] Matches",
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
        Some(id) => format!(" {} (#{}) ", name.to_uppercase(), id),
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
            Constraint::Length(1),
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

    let instructions = Paragraph::new(Line::from(Span::styled(
        hint,
        Style::default().fg(Color::Gray),
    )))
    .alignment(Alignment::Center);
    f.render_widget(instructions, rows[2]);
}

fn draw_scorecard_table(f: &mut Frame, area: Rect, m: &boxing_core::BoxingMatch) {
    let mut rows: Vec<Row> = Vec::new();

    // Walkout row before Round 1
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

    if let Some(active) = &m.active_round {
        if !active.is_walkout() {
            rows.push(
                Row::new(vec![
                    format!("▶ R{}", active.round_number),
                    format!("{}*", active.fighter_a_score),
                    format!("{}*", active.fighter_b_score),
                ])
                .style(
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
            );
        }
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

    let row_count = rows.len();
    let table = Table::new(
        rows,
        [
            Constraint::Percentage(36),
            Constraint::Percentage(32),
            Constraint::Percentage(32),
        ],
    )
    .header(
        Row::new(vec!["Round", &m.fighter_a, &m.fighter_b]).style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
    )
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Scorecard "),
    );

    // Keep the bottom of the scorecard (latest rounds, Result, and TOTAL) visible on small terminals
    let mut table_state = TableState::default();
    if row_count > 0 {
        table_state.select(Some(row_count - 1));
    }
    f.render_stateful_widget(table, area, &mut table_state);
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
