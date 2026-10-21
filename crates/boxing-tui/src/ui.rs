use crate::app::{App, FinishField, FormField, Screen};
use boxing_core::{fighter_last_name, FighterSide, TimerPhase};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Gauge, Paragraph, Row, Table, TableState},
    Frame,
};

const BOXREC_URL_PREFIX: &str = "https://boxrec.com/en/box-pro/";

fn boxrec_url(id: &str) -> String {
    format!("{}{}", BOXREC_URL_PREFIX, id.trim())
}

pub fn draw(f: &mut Frame, app: &mut App) {
    app.clickable_links.clear();
    match app.screen {
        Screen::Home => draw_home(f, app),
        Screen::NewMatch => {
            draw_home(f, app);
            app.clickable_links.clear();
            draw_new_match_modal(f, app);
        }
        Screen::Scoring => draw_scoring(f, app),
        Screen::EditBoxRec => {
            if app.previous_screen == Screen::Scoring {
                draw_scoring(f, app);
            } else {
                draw_home(f, app);
            }
            app.clickable_links.clear();
            draw_boxrec_modal(f, app);
        }
        Screen::FinishMatch => {
            if app.previous_screen == Screen::Scoring {
                draw_scoring(f, app);
            } else {
                draw_home(f, app);
            }
            app.clickable_links.clear();
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
            Constraint::Length(1),
            Constraint::Length(1),
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

    if app.matches.is_empty() {
        let empty = Paragraph::new(Line::from(Span::styled(
            "  No matches yet — [N]ew match",
            Style::default().fg(Color::DarkGray),
        )))
        .block(Block::default().borders(Borders::ALL).title(" Matches "));
        f.render_widget(empty, chunks[1]);
    } else {
        let rows: Vec<Row> = app
            .matches
            .iter()
            .enumerate()
            .map(|(idx, m)| {
                let (tot_a, tot_b) = m.total_scores();
                let status = if let Some(summary) = m.outcome_summary() {
                    summary
                } else {
                    match &m.active_round {
                        Some(r) if r.is_walkout() => "Walkout".to_string(),
                        Some(r) => format!("Round {}/{}", r.round_number, m.scheduled_rounds),
                        None => "Complete".to_string(),
                    }
                };
                let fa_last = fighter_last_name(&m.fighter_a).to_string();
                let fb_last = fighter_last_name(&m.fighter_b).to_string();
                let link_mod = Style::default().add_modifier(Modifier::UNDERLINED);

                let match_line = Line::from(vec![
                    if m.fighter_a_boxrec_id.is_some() {
                        Span::styled(m.fighter_a.clone(), link_mod)
                    } else {
                        Span::raw(m.fighter_a.clone())
                    },
                    Span::raw(" v "),
                    if m.fighter_b_boxrec_id.is_some() {
                        Span::styled(m.fighter_b.clone(), link_mod)
                    } else {
                        Span::raw(m.fighter_b.clone())
                    },
                    Span::raw(format!(" ({})", m.date.format("%d %b %Y"))),
                ]);

                let score_line = Line::from(vec![
                    if m.fighter_a_boxrec_id.is_some() {
                        Span::styled(fa_last, link_mod)
                    } else {
                        Span::raw(fa_last)
                    },
                    Span::raw(format!(" {} - {} ", tot_a, tot_b)),
                    if m.fighter_b_boxrec_id.is_some() {
                        Span::styled(fb_last, link_mod)
                    } else {
                        Span::raw(fb_last)
                    },
                ]);

                let style = if idx == app.selected_match_idx {
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::Cyan)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };
                Row::new(vec![match_line, Line::from(status), score_line]).style(style)
            })
            .collect();

        let col_widths = [
            Constraint::Percentage(48),
            Constraint::Percentage(26),
            Constraint::Percentage(26),
        ];
        let table_block = Block::default().borders(Borders::ALL).title(" Matches ");
        let table_inner = table_block.inner(chunks[1]);

        let table = Table::new(rows, col_widths)
            .column_spacing(2)
            .header(
                Row::new(vec!["Match", "Status", "Scorecard"]).style(
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
            )
            .block(table_block);

        let mut table_state = TableState::default();
        table_state.select(Some(app.selected_match_idx));
        f.render_stateful_widget(table, chunks[1], &mut table_state);

        // Register clickable BoxRec fighter name regions in the Match and Scorecard columns
        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .spacing(2)
            .constraints(col_widths)
            .split(table_inner);
        let first_idx = table_state.offset();
        let max_visible_rows = table_inner.height.saturating_sub(1) as usize;
        for (vis_row, m) in app
            .matches
            .iter()
            .skip(first_idx)
            .take(max_visible_rows)
            .enumerate()
        {
            let row_y = table_inner.y + 1 + vis_row as u16;
            if row_y >= table_inner.y + table_inner.height {
                break;
            }
            let (tot_a, tot_b) = m.total_scores();
            let fa_last = fighter_last_name(&m.fighter_a);
            let fb_last = fighter_last_name(&m.fighter_b);
            let fa_full_w = m.fighter_a.chars().count() as u16;
            let fb_full_w = m.fighter_b.chars().count() as u16;
            let fa_last_w = fa_last.chars().count() as u16;
            let fb_last_w = fb_last.chars().count() as u16;

            if let Some(ref id_a) = m.fighter_a_boxrec_id {
                let url_a = boxrec_url(id_a);
                let w_match = fa_full_w.min(cols[0].width);
                if w_match > 0 {
                    app.clickable_links
                        .push((cols[0].x, row_y, w_match, url_a.clone()));
                }
                let w_score = fa_last_w.min(cols[2].width);
                if w_score > 0 {
                    app.clickable_links
                        .push((cols[2].x, row_y, w_score, url_a));
                }
            }

            if let Some(ref id_b) = m.fighter_b_boxrec_id {
                let url_b = boxrec_url(id_b);
                let match_b_offset = fa_full_w.saturating_add(3); // " v "
                if match_b_offset < cols[0].width {
                    let w_match = fb_full_w.min(cols[0].width - match_b_offset);
                    if w_match > 0 {
                        app.clickable_links.push((
                            cols[0].x + match_b_offset,
                            row_y,
                            w_match,
                            url_b.clone(),
                        ));
                    }
                }
                let score_prefix = format!("{} {} - {} ", fa_last, tot_a, tot_b);
                let score_b_offset = score_prefix.chars().count() as u16;
                if score_b_offset < cols[2].width {
                    let w_score = fb_last_w.min(cols[2].width - score_b_offset);
                    if w_score > 0 {
                        app.clickable_links.push((
                            cols[2].x + score_b_offset,
                            row_y,
                            w_score,
                            url_b,
                        ));
                    }
                }
            }
        }
    }

    let btn = Paragraph::new(
        " [N]ew   [O]pen   [F]inish   [B]oxRec   [D]elete   [Q]uit ",
    )
    .alignment(Alignment::Center)
    .style(
        Style::default()
            .fg(Color::Black)
            .bg(Color::Green)
            .add_modifier(Modifier::BOLD),
    );
    f.render_widget(btn, chunks[2]);

    let footer = Paragraph::new(format!(" {}", app.status_message))
        .style(Style::default().fg(Color::Gray));
    f.render_widget(footer, chunks[3]);
}

fn draw_new_match_modal(f: &mut Frame, app: &mut App) {
    let area = centered_modal(64, 21, f.area());
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
            Constraint::Min(1),
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
    let area = centered_modal(56, 12, f.area());
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
            Constraint::Min(1),
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
    let area = centered_modal(72, 18, f.area());
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
            Constraint::Min(1),
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
            Constraint::Length(1), // Action Bar
            Constraint::Length(1), // Status & Controls Bar
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
        (format!("🏆 {}", summary), Color::Green)
    } else {
        match &m.active_round {
            Some(active) if active.is_walkout() => ("🎵 WALKOUT".to_string(), Color::Magenta),
            Some(active) => (
                format!("ROUND {} / {}", active.round_number, m.scheduled_rounds),
                Color::Blue,
            ),
            None => ("Complete".to_string(), Color::DarkGray),
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

    let fa_name = m.fighter_a.clone();
    let fb_name = m.fighter_b.clone();
    let fa_boxrec = m.fighter_a_boxrec_id.clone();
    let fb_boxrec = m.fighter_b_boxrec_id.clone();
    let is_active = m.active_round.is_some();

    draw_fighter_card(
        f,
        cards_split[0],
        &fa_name,
        fa_boxrec.as_deref(),
        score_a,
        is_active,
        points_caption,
        Color::LightRed,
        "[-1] 1 / ← / A   [+1] ! / W",
        &mut app.clickable_links,
    );

    draw_fighter_card(
        f,
        cards_split[1],
        &fb_name,
        fb_boxrec.as_deref(),
        score_b,
        is_active,
        points_caption,
        Color::LightBlue,
        "[-1] 2 / → / D   [+1] @ / E",
        &mut app.clickable_links,
    );

    if let Some(ref m_ref) = app.current_match {
        draw_scorecard_table(f, main_split[1], m_ref, &mut app.clickable_links);
    }

    let Some(ref m) = app.current_match else {
        return;
    };

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
            let done_btn = Paragraph::new(format!(" ✅ {} ", summary))
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
        " {}  |  [0] Walkout  [B]oxRec  [U]ndo  [Esc] Matches",
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
    clickable_links: &mut Vec<(u16, u16, u16, String)>,
) {
    let title_str = format!(" {} ", name.to_uppercase());

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

    let header_style = if boxrec_id.is_some() {
        Style::default()
            .fg(accent)
            .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
    } else {
        Style::default().fg(accent).add_modifier(Modifier::BOLD)
    };

    let header = Paragraph::new(name)
        .alignment(Alignment::Center)
        .style(header_style);
    f.render_widget(header, rows[0]);

    if let Some(id) = boxrec_id {
        let name_w = (name.chars().count() as u16).min(rows[0].width);
        if name_w > 0 {
            let name_x = rows[0].x + rows[0].width.saturating_sub(name_w) / 2;
            clickable_links.push((name_x, rows[0].y, name_w, boxrec_url(id)));
        }
    }

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

fn draw_scorecard_table(
    f: &mut Frame,
    area: Rect,
    m: &boxing_core::BoxingMatch,
    clickable_links: &mut Vec<(u16, u16, u16, String)>,
) {
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
    let col_widths = [
        Constraint::Percentage(36),
        Constraint::Percentage(32),
        Constraint::Percentage(32),
    ];
    let table_block = Block::default()
        .borders(Borders::ALL)
        .title(" Scorecard ");
    let table_inner = table_block.inner(area);
    let fa_last = fighter_last_name(&m.fighter_a);
    let fb_last = fighter_last_name(&m.fighter_b);
    let link_mod = Style::default().add_modifier(Modifier::UNDERLINED);

    let table = Table::new(rows, col_widths)
        .header(
            Row::new(vec![
                Span::raw("Round"),
                if m.fighter_a_boxrec_id.is_some() {
                    Span::styled(fa_last, link_mod)
                } else {
                    Span::raw(fa_last)
                },
                if m.fighter_b_boxrec_id.is_some() {
                    Span::styled(fb_last, link_mod)
                } else {
                    Span::raw(fb_last)
                },
            ])
            .style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
        )
        .block(table_block);

    // Keep the bottom of the scorecard (latest rounds, Result, and TOTAL) visible on small terminals
    let mut table_state = TableState::default();
    if row_count > 0 {
        table_state.select(Some(row_count - 1));
    }
    f.render_stateful_widget(table, area, &mut table_state);

    if table_inner.height > 0 {
        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .spacing(1)
            .constraints(col_widths)
            .split(table_inner);
        if let Some(ref id_a) = m.fighter_a_boxrec_id {
            let w = (fa_last.chars().count() as u16).min(cols[1].width);
            if w > 0 {
                clickable_links.push((cols[1].x, table_inner.y, w, boxrec_url(id_a)));
            }
        }
        if let Some(ref id_b) = m.fighter_b_boxrec_id {
            let w = (fb_last.chars().count() as u16).min(cols[2].width);
            if w > 0 {
                clickable_links.push((cols[2].x, table_inner.y, w, boxrec_url(id_b)));
            }
        }
    }
}

fn centered_modal(min_width: u16, min_height: u16, r: Rect) -> Rect {
    let width = min_width.max((r.width * 60) / 100).min(r.width.saturating_sub(2));
    let height = min_height.min(r.height.saturating_sub(2));
    let x = r.x + r.width.saturating_sub(width) / 2;
    let y = r.y + r.height.saturating_sub(height) / 2;
    Rect::new(x, y, width, height)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::App;
    use boxing_core::{
        BoxingMatch, FighterSide, FinishMethod, JsonMatchRepository, MatchOutcome, RoundScore,
        ScoringService,
    };
    use chrono::NaiveDate;
    use ratatui::{backend::TestBackend, Terminal};
    use std::path::PathBuf;

    #[test]
    fn test_home_screen_rendering_has_no_artifacts() {
        let repo = JsonMatchRepository::new(PathBuf::from("nonexistent_test_matches.json"));
        let service = ScoringService::new(repo);
        let mut app = App::new(service);

        let mut m = BoxingMatch::new(
            "Daniel Dubois",
            "Fabio Wardley",
            NaiveDate::from_ymd_opt(2026, 10, 17).unwrap(),
        );
        m.set_boxrec_id(FighterSide::FighterA, "787430");
        m.set_boxrec_id(FighterSide::FighterB, "785893");
        m.locked_rounds.push(RoundScore {
            round_number: 1,
            fighter_a_score: 10,
            fighter_b_score: 9,
        });
        m.finish_match(MatchOutcome {
            method: FinishMethod::Tko,
            winner: Some(FighterSide::FighterA),
            round: 2,
        });
        app.matches = vec![m];

        let backend = TestBackend::new(110, 20);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal.draw(|f| draw(f, &mut app)).unwrap();
        let buf = terminal.backend().buffer();

        let mut lines = Vec::new();
        for y in 0..buf.area.height {
            let mut row = String::new();
            for x in 0..buf.area.width {
                let sym = buf[(x, y)].symbol();
                assert_eq!(
                    sym.chars().count(),
                    1,
                    "Every cell symbol must be 1 char at ({}, {}): {:?}",
                    x,
                    y,
                    sym
                );
                row.push_str(sym);
            }
            lines.push(row);
        }
        let full_screen = lines.join("\n");
        assert!(full_screen.contains("Daniel Dubois v Fabio Wardley (17 Oct 2026)"));
        assert!(full_screen.contains("Dubois by TKO (R2)"));
        assert!(full_screen.contains("Dubois 10 - 9 Wardley"));
        assert!(full_screen.contains("[N]ew   [O]pen   [F]inish   [B]oxRec   [D]elete   [Q]uit"));
        assert!(!full_screen.contains("[Enter] Open"));
        assert!(!full_screen.contains("FINAL:"));
        assert!(!full_screen.contains("Walkout"));
        assert_eq!(app.clickable_links.len(), 4);

        // Verify the green action bar is exactly 1 row high (only 1 row has Color::Green background)
        let green_rows: Vec<u16> = (0..buf.area.height)
            .filter(|&y| (0..buf.area.width).any(|x| buf[(x, y)].bg == Color::Green))
            .collect();
        assert_eq!(green_rows.len(), 1);
    }
}
