mod app;
mod ui;

use app::{App, FinishField, FormField, Screen};
use boxing_core::{FighterSide, FinishMethod, JsonMatchRepository, ScoringService};
use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, KeyModifiers,
        MouseButton, MouseEventKind,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{
    env, io, panic,
    path::PathBuf,
    process::{Command, Stdio},
    time::{Duration, Instant},
};

fn restore_terminal() {
    let _ = disable_raw_mode();
    let _ = execute!(io::stdout(), DisableMouseCapture, LeaveAlternateScreen);
}

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();
    let mut json_path = PathBuf::from("matches.json");

    let mut idx = 1;
    while idx < args.len() {
        let arg = args[idx].as_str();
        match arg {
            "--file" | "-f" | "--db" if idx + 1 < args.len() => {
                json_path = PathBuf::from(&args[idx + 1]);
                idx += 2;
            }
            "--test" | "-t" => {
                json_path = PathBuf::from("matches.test.json");
                idx += 1;
            }
            "--version" | "-V" => {
                println!("boxing {}", env!("CARGO_PKG_VERSION"));
                return Ok(());
            }
            "--help" | "-h" => {
                println!("Boxing Scorecard v{}", env!("CARGO_PKG_VERSION"));
                println!("Usage: boxing [matches.json] [--file <path>] [--test] [--version]");
                return Ok(());
            }
            _ if !arg.starts_with('-') => {
                json_path = PathBuf::from(arg);
                idx += 1;
            }
            _ => {
                idx += 1;
            }
        }
    }

    let file_label = json_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("matches.json")
        .to_string();
    let repo = JsonMatchRepository::new(json_path);
    let service = ScoringService::new(repo);
    let mut app = App::new(service);
    app.spawn_release_check();
    if app.status_message == "Ready" {
        app.status_message = format!("Loaded {} ({} matches)", file_label, app.matches.len());
    }

    // Install panic hook so raw mode and alternate screen are always cleaned up on panic
    let default_hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        restore_terminal();
        default_hook(panic_info);
    }));

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let res = run_event_loop(&mut terminal, &mut app);

    // Restore terminal
    restore_terminal();
    terminal.show_cursor()?;

    if let Err(err) = res {
        eprintln!("Error: {:?}", err);
    }

    Ok(())
}

fn open_url_in_browser(url: &str) {
    #[cfg(target_os = "windows")]
    let _ = Command::new("cmd")
        .args(["/C", "start", "", url])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();

    #[cfg(target_os = "macos")]
    let _ = Command::new("open")
        .arg(url)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();

    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    let _ = Command::new("xdg-open")
        .arg(url)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();
}

fn run_event_loop<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
) -> io::Result<()> {
    let tick_rate = Duration::from_millis(100);
    let mut last_tick = Instant::now();

    loop {
        terminal.draw(|f| ui::draw(f, app))?;

        let timeout = tick_rate.saturating_sub(last_tick.elapsed());
        if event::poll(timeout)? {
            match event::read()? {
                Event::Key(key) => {
                    if key.kind == KeyEventKind::Press {
                        handle_key_event(app, key.code, key.modifiers);
                    }
                }
                Event::Mouse(mouse) => {
                    if mouse.kind == MouseEventKind::Down(MouseButton::Left) {
                        for (lx, ly, lw, url) in &app.clickable_links {
                            if mouse.row == *ly
                                && mouse.column >= *lx
                                && mouse.column < lx.saturating_add(*lw)
                            {
                                let target = url.clone();
                                open_url_in_browser(&target);
                                app.status_message = format!("Opened {}", target);
                                break;
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        if last_tick.elapsed() >= tick_rate {
            let elapsed = last_tick.elapsed();
            last_tick = Instant::now();
            app.on_tick(elapsed);
        }

        if app.should_quit {
            return Ok(());
        }
    }
}

fn handle_key_event(app: &mut App, code: KeyCode, modifiers: KeyModifiers) {
    if modifiers.contains(KeyModifiers::CONTROL) && code == KeyCode::Char('c') {
        app.should_quit = true;
        return;
    }

    match app.screen {
        Screen::Home => match code {
            KeyCode::Char('q') | KeyCode::Char('Q') => app.should_quit = true,
            KeyCode::Char('n') | KeyCode::Char('N') => app.open_new_match_form(),
            KeyCode::Char('b') | KeyCode::Char('B') => app.open_boxrec_editor(),
            KeyCode::Up | KeyCode::Char('k') => {
                app.confirm_delete_id = None;
                if app.selected_match_idx > 0 {
                    app.selected_match_idx -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                app.confirm_delete_id = None;
                if app.selected_match_idx + 1 < app.matches.len() {
                    app.selected_match_idx += 1;
                }
            }
            KeyCode::PageUp => {
                app.confirm_delete_id = None;
                app.selected_match_idx = app.selected_match_idx.saturating_sub(10);
            }
            KeyCode::PageDown => {
                app.confirm_delete_id = None;
                if !app.matches.is_empty() {
                    app.selected_match_idx =
                        (app.selected_match_idx + 10).min(app.matches.len() - 1);
                }
            }
            KeyCode::Home => {
                app.confirm_delete_id = None;
                app.selected_match_idx = 0;
            }
            KeyCode::End => {
                app.confirm_delete_id = None;
                if !app.matches.is_empty() {
                    app.selected_match_idx = app.matches.len() - 1;
                }
            }
            KeyCode::Enter | KeyCode::Char('o') | KeyCode::Char('O') => {
                app.confirm_delete_id = None;
                if let Some(m) = app.matches.get(app.selected_match_idx).cloned() {
                    app.start_scoring_match(m);
                } else {
                    app.open_new_match_form();
                }
            }
            KeyCode::Char('d') | KeyCode::Char('D') | KeyCode::Delete => {
                app.request_delete_selected_match();
            }
            KeyCode::Char('y') | KeyCode::Char('Y') if app.confirm_delete_id.is_some() => {
                app.request_delete_selected_match();
            }
            KeyCode::Esc if app.confirm_delete_id.is_some() => {
                app.confirm_delete_id = None;
                app.status_message = "Cancelled match deletion.".to_string();
            }
            _ => {
                if app.confirm_delete_id.is_some() {
                    app.confirm_delete_id = None;
                    app.status_message = "Cancelled match deletion.".to_string();
                }
            }
        },
        Screen::NewMatch => match code {
            KeyCode::Esc => {
                app.screen = Screen::Home;
                app.status_message = "Cancelled new match creation.".to_string();
            }
            KeyCode::Tab | KeyCode::Down => {
                app.form_field = app.form_field.next();
            }
            KeyCode::BackTab | KeyCode::Up => {
                app.form_field = app.form_field.prev();
            }
            KeyCode::Enter => {
                app.submit_new_match();
            }
            KeyCode::Left if app.form_field == FormField::Rounds => {
                if app.input_rounds > 1 {
                    app.input_rounds -= 1;
                }
            }
            KeyCode::Right if app.form_field == FormField::Rounds => {
                if app.input_rounds < 15 {
                    app.input_rounds += 1;
                }
            }
            KeyCode::Backspace => match app.form_field {
                FormField::FighterA => {
                    app.input_fighter_a.pop();
                }
                FormField::FighterB => {
                    app.input_fighter_b.pop();
                }
                FormField::Date => {
                    app.input_date.pop();
                }
                FormField::Rounds => {}
            },
            KeyCode::Char(c) => match app.form_field {
                FormField::FighterA => app.input_fighter_a.push(c),
                FormField::FighterB => app.input_fighter_b.push(c),
                FormField::Date => app.input_date.push(c),
                FormField::Rounds => {
                    if c == '+' && app.input_rounds < 15 {
                        app.input_rounds += 1;
                    } else if c == '-' && app.input_rounds > 1 {
                        app.input_rounds -= 1;
                    }
                }
            },
            _ => {}
        },
        Screen::EditBoxRec => match code {
            KeyCode::Esc => {
                app.screen = app.previous_screen;
                app.status_message = "Cancelled BoxRec ID update.".to_string();
            }
            KeyCode::Tab | KeyCode::BackTab | KeyCode::Up | KeyCode::Down => {
                app.boxrec_side = match app.boxrec_side {
                    FighterSide::FighterA => FighterSide::FighterB,
                    FighterSide::FighterB => FighterSide::FighterA,
                };
            }
            KeyCode::Enter => {
                app.submit_boxrec_ids();
            }
            KeyCode::Backspace => match app.boxrec_side {
                FighterSide::FighterA => {
                    app.input_boxrec_a.pop();
                }
                FighterSide::FighterB => {
                    app.input_boxrec_b.pop();
                }
            },
            KeyCode::Char(c) => match app.boxrec_side {
                FighterSide::FighterA => app.input_boxrec_a.push(c),
                FighterSide::FighterB => app.input_boxrec_b.push(c),
            },
            _ => {}
        },
        Screen::FinishMatch => match code {
            KeyCode::Esc => {
                app.screen = app.previous_screen;
                app.status_message = "Cancelled stoppage / finish.".to_string();
            }
            KeyCode::Tab | KeyCode::Down | KeyCode::Char('j') => {
                app.finish_field = app.finish_field.next();
            }
            KeyCode::BackTab | KeyCode::Up => {
                app.finish_field = app.finish_field.prev();
            }
            KeyCode::Left | KeyCode::Char('h') => match app.finish_field {
                FinishField::Method => app.finish_method = app.finish_method.prev(),
                FinishField::Winner => app.cycle_finish_winner_prev(),
                FinishField::Round => {
                    if app.finish_round > 1 {
                        app.finish_round -= 1;
                    }
                }
            },
            KeyCode::Right | KeyCode::Char('l') => match app.finish_field {
                FinishField::Method => app.finish_method = app.finish_method.next(),
                FinishField::Winner => app.cycle_finish_winner_next(),
                FinishField::Round => {
                    if app.finish_round < app.finish_max_rounds {
                        app.finish_round += 1;
                    }
                }
            },
            // Instant Winner hotkeys (1 = Fighter 1, 2 = Fighter 2, 0 = Draw)
            KeyCode::Char('1') => {
                app.finish_winner = Some(FighterSide::FighterA);
                app.finish_field = FinishField::Winner;
            }
            KeyCode::Char('2') => {
                app.finish_winner = Some(FighterSide::FighterB);
                app.finish_field = FinishField::Winner;
            }
            KeyCode::Char('0') => {
                app.finish_winner = None;
                app.finish_field = FinishField::Winner;
            }
            // Instant Method hotkeys (T = TKO, K = KO, R = RTD/Towel, Q = DQ, D = Decision/TD)
            KeyCode::Char('t') | KeyCode::Char('T') => {
                app.finish_method = FinishMethod::Tko;
                app.finish_field = FinishField::Method;
            }
            KeyCode::Char('k') | KeyCode::Char('K') => {
                app.finish_method = FinishMethod::Ko;
                app.finish_field = FinishField::Method;
            }
            KeyCode::Char('r') | KeyCode::Char('R') => {
                app.finish_method = FinishMethod::Rtd;
                app.finish_field = FinishField::Method;
            }
            KeyCode::Char('q') | KeyCode::Char('Q') => {
                app.finish_method = FinishMethod::Dq;
                app.finish_field = FinishField::Method;
            }
            KeyCode::Char('d') | KeyCode::Char('D') => {
                app.finish_method = if app.finish_method == FinishMethod::Decision {
                    FinishMethod::Td
                } else {
                    FinishMethod::Decision
                };
                app.finish_field = FinishField::Method;
            }
            // Instant Round hotkeys (- / +)
            KeyCode::Char('-') | KeyCode::Char('[') => {
                if app.finish_round > 1 {
                    app.finish_round -= 1;
                }
                app.finish_field = FinishField::Round;
            }
            KeyCode::Char('+') | KeyCode::Char('=') | KeyCode::Char(']') => {
                if app.finish_round < app.finish_max_rounds {
                    app.finish_round += 1;
                }
                app.finish_field = FinishField::Round;
            }
            KeyCode::Enter | KeyCode::Char(' ') => {
                app.submit_finish_match();
            }
            _ => {}
        },
        Screen::Scoring => match code {
            KeyCode::Esc | KeyCode::Char('m') | KeyCode::Char('M') => {
                app.screen = Screen::Home;
                app.reload_matches();
                app.status_message = "Returned to match list.".to_string();
            }
            KeyCode::Char('q') | KeyCode::Char('Q') => app.should_quit = true,

            // Deduct point from Fighter A
            KeyCode::Char('1') | KeyCode::Left | KeyCode::Char('a') => {
                app.deduct_point(FighterSide::FighterA);
            }
            // Restore point to Fighter A
            KeyCode::Char('!') | KeyCode::Char('A') | KeyCode::Char('w') | KeyCode::Char('W') => {
                app.restore_point(FighterSide::FighterA);
            }

            // Deduct point from Fighter B
            KeyCode::Char('2') | KeyCode::Right | KeyCode::Char('d') => {
                app.deduct_point(FighterSide::FighterB);
            }
            // Restore point to Fighter B
            KeyCode::Char('@') | KeyCode::Char('D') | KeyCode::Char('e') | KeyCode::Char('E') => {
                app.restore_point(FighterSide::FighterB);
            }

            // Lock round in
            KeyCode::Enter | KeyCode::Char(' ') => {
                app.lock_current_round();
            }

            // Undo last locked round (or step back to Walkout)
            KeyCode::Char('u') | KeyCode::Char('U') => {
                app.undo_last_locked_round();
            }

            // Jump directly to Pre-Fight Walkout scoring
            KeyCode::Char('0') => {
                app.open_walkout_stage();
            }

            // Open optional BoxRec ID editor after the fact
            KeyCode::Char('b') | KeyCode::Char('B') => {
                app.open_boxrec_editor();
            }

            // Record Stoppage (TKO, Towel/RTD, KO, DQ) or Mark Fight Complete mid-way
            KeyCode::Char('f')
            | KeyCode::Char('F')
            | KeyCode::Char('s')
            | KeyCode::Char('S')
            | KeyCode::Char('x')
            | KeyCode::Char('X') => {
                app.open_finish_match_modal();
            }
            KeyCode::Char('k') | KeyCode::Char('K') => {
                app.open_finish_match_modal_with(Some(FinishMethod::Ko));
            }
            KeyCode::Char('r') | KeyCode::Char('R') => {
                app.open_finish_match_modal_with(Some(FinishMethod::Rtd));
            }

            // Timer controls
            KeyCode::Char('p') | KeyCode::Char('P') => {
                if let Some(ref mut timer) = app.timer {
                    timer.toggle_pause();
                    app.status_message = if timer.paused {
                        "Timer paused.".to_string()
                    } else {
                        "Timer resumed.".to_string()
                    };
                }
            }
            KeyCode::Char('t') | KeyCode::Char('T') => {
                if let Some(ref mut timer) = app.timer {
                    if let Some(prompt) = timer.skip_phase() {
                        app.status_message = prompt.message();
                        app.alert_until = Some(Instant::now() + Duration::from_secs(3));
                    }
                }
            }
            _ => {}
        },
    }
}
