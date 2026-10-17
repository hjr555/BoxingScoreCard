use boxing_core::{
    BoxingMatch, FighterSide, FinishMethod, JsonMatchRepository, MatchOutcome, MatchTimer,
    ScoringService, TimerPhase, TimerPrompt, DEFAULT_SCHEDULED_ROUNDS,
};
use chrono::{Local, NaiveDate};
use std::time::{Duration, Instant};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Home,
    NewMatch,
    Scoring,
    EditBoxRec,
    FinishMatch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormField {
    FighterA,
    FighterB,
    Date,
    Rounds,
}

impl FormField {
    pub fn next(self) -> Self {
        match self {
            FormField::FighterA => FormField::FighterB,
            FormField::FighterB => FormField::Date,
            FormField::Date => FormField::Rounds,
            FormField::Rounds => FormField::FighterA,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            FormField::FighterA => FormField::Rounds,
            FormField::FighterB => FormField::FighterA,
            FormField::Date => FormField::FighterB,
            FormField::Rounds => FormField::Date,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FinishField {
    Method,
    Winner,
    Round,
}

impl FinishField {
    pub fn next(self) -> Self {
        match self {
            FinishField::Method => FinishField::Winner,
            FinishField::Winner => FinishField::Round,
            FinishField::Round => FinishField::Method,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            FinishField::Method => FinishField::Round,
            FinishField::Winner => FinishField::Method,
            FinishField::Round => FinishField::Winner,
        }
    }
}

pub struct App {
    pub service: ScoringService<JsonMatchRepository>,
    pub screen: Screen,
    pub previous_screen: Screen,
    pub matches: Vec<BoxingMatch>,
    pub selected_match_idx: usize,
    pub current_match: Option<BoxingMatch>,
    pub timer: Option<MatchTimer>,
    pub status_message: String,
    pub alert_until: Option<Instant>,
    pub should_quit: bool,

    // New match form state (minimal, fast: no BoxRec required)
    pub form_field: FormField,
    pub input_fighter_a: String,
    pub input_fighter_b: String,
    pub input_date: String,
    pub input_rounds: u8,

    // Post-hoc BoxRec ID editor state
    pub boxrec_target_match_id: Option<Uuid>,
    pub boxrec_target_title: String,
    pub boxrec_fighter_a_name: String,
    pub boxrec_fighter_b_name: String,
    pub boxrec_side: FighterSide,
    pub input_boxrec_a: String,
    pub input_boxrec_b: String,

    // Stoppage / Finish Match modal state
    pub finish_target_match_id: Option<Uuid>,
    pub finish_target_title: String,
    pub finish_fighter_a_name: String,
    pub finish_fighter_b_name: String,
    pub finish_max_rounds: u8,
    pub finish_field: FinishField,
    pub finish_method: FinishMethod,
    pub finish_winner: Option<FighterSide>,
    pub finish_round: u8,
}

impl App {
    pub fn new(service: ScoringService<JsonMatchRepository>) -> Self {
        let matches = service.list_matches().unwrap_or_default();
        let today = Local::now().date_naive();
        Self {
            service,
            screen: Screen::Home,
            previous_screen: Screen::Home,
            matches,
            selected_match_idx: 0,
            current_match: None,
            timer: None,
            status_message: "Welcome! Press [N] to score a new match, or [Enter] to open selected."
                .to_string(),
            alert_until: None,
            should_quit: false,
            form_field: FormField::FighterA,
            input_fighter_a: "Joshua".to_string(),
            input_fighter_b: "Fury".to_string(),
            input_date: today.format("%d %b %Y").to_string(),
            input_rounds: DEFAULT_SCHEDULED_ROUNDS,
            boxrec_target_match_id: None,
            boxrec_target_title: String::new(),
            boxrec_fighter_a_name: String::new(),
            boxrec_fighter_b_name: String::new(),
            boxrec_side: FighterSide::FighterA,
            input_boxrec_a: String::new(),
            input_boxrec_b: String::new(),
            finish_target_match_id: None,
            finish_target_title: String::new(),
            finish_fighter_a_name: String::new(),
            finish_fighter_b_name: String::new(),
            finish_max_rounds: DEFAULT_SCHEDULED_ROUNDS,
            finish_field: FinishField::Method,
            finish_method: FinishMethod::Tko,
            finish_winner: Some(FighterSide::FighterA),
            finish_round: 1,
        }
    }

    pub fn reload_matches(&mut self) {
        if let Ok(list) = self.service.list_matches() {
            self.matches = list;
            if self.selected_match_idx >= self.matches.len() && !self.matches.is_empty() {
                self.selected_match_idx = self.matches.len() - 1;
            }
        }
    }

    pub fn open_new_match_form(&mut self) {
        self.form_field = FormField::FighterA;
        if self.input_fighter_a.is_empty() {
            self.input_fighter_a = "Joshua".to_string();
        }
        if self.input_fighter_b.is_empty() {
            self.input_fighter_b = "Fury".to_string();
        }
        if self.input_date.is_empty() {
            self.input_date = Local::now().date_naive().format("%d %b %Y").to_string();
        }
        self.screen = Screen::NewMatch;
        self.status_message =
            "Enter match details. [Tab/↑/↓] switch fields, [Enter] start match.".to_string();
    }

    /// Opens the optional BoxRec ID modal after the fact for either the active match or selected match.
    pub fn open_boxrec_editor(&mut self) {
        let target = match self.screen {
            Screen::Scoring => self.current_match.clone(),
            Screen::Home => self.matches.get(self.selected_match_idx).cloned(),
            _ => None,
        };

        let Some(m) = target else {
            self.status_message = "Select or open a match first to add BoxRec IDs.".to_string();
            return;
        };

        self.previous_screen = self.screen;
        self.boxrec_target_match_id = Some(m.id);
        self.boxrec_target_title = m.title();
        self.boxrec_fighter_a_name = m.fighter_a.clone();
        self.boxrec_fighter_b_name = m.fighter_b.clone();
        self.boxrec_side = FighterSide::FighterA;
        self.input_boxrec_a = m.fighter_a_boxrec_id.unwrap_or_default();
        self.input_boxrec_b = m.fighter_b_boxrec_id.unwrap_or_default();
        self.screen = Screen::EditBoxRec;
        self.status_message =
            "Optional BoxRec IDs: [Tab/↑/↓] switch fighter, [Enter] save, [Esc] cancel.".to_string();
    }

    pub fn submit_boxrec_ids(&mut self) {
        let Some(match_id) = self.boxrec_target_match_id else {
            self.screen = self.previous_screen;
            return;
        };

        match self.service.set_boxrec_ids(
            match_id,
            self.input_boxrec_a.clone(),
            self.input_boxrec_b.clone(),
        ) {
            Ok(updated) => {
                if self
                    .current_match
                    .as_ref()
                    .map(|m| m.id == updated.id)
                    .unwrap_or(false)
                {
                    self.current_match = Some(updated.clone());
                }
                self.reload_matches();
                self.screen = self.previous_screen;
                self.status_message = format!("Saved BoxRec IDs for {}.", updated.title());
            }
            Err(e) => {
                self.status_message = format!("Failed to save BoxRec IDs: {}", e);
            }
        }
    }

    /// Opens the Finish / Stoppage modal (TKO, RTD/Towel, KO, Decision/Complete) to record the result and mark complete.
    pub fn open_finish_match_modal(&mut self) {
        let target = match self.screen {
            Screen::Scoring => self.current_match.clone(),
            Screen::Home => self.matches.get(self.selected_match_idx).cloned(),
            _ => None,
        };

        let Some(m) = target else {
            self.status_message = "Select or open a match first to finish/stop it.".to_string();
            return;
        };

        self.previous_screen = self.screen;
        self.finish_target_match_id = Some(m.id);
        self.finish_target_title = m.title();
        self.finish_fighter_a_name = m.fighter_a.clone();
        self.finish_fighter_b_name = m.fighter_b.clone();
        self.finish_max_rounds = m.scheduled_rounds;
        self.finish_field = FinishField::Method;

        if let Some(existing) = &m.outcome {
            self.finish_method = existing.method;
            self.finish_winner = existing.winner;
            self.finish_round = existing.round;
        } else {
            self.finish_method = FinishMethod::Tko;
            let (tot_a, tot_b) = m.total_scores();
            self.finish_winner = if tot_b > tot_a {
                Some(FighterSide::FighterB)
            } else {
                Some(FighterSide::FighterA)
            };
            let current_rnd = m
                .active_round
                .as_ref()
                .map(|r| r.round_number.max(1))
                .unwrap_or_else(|| (m.locked_rounds.len() as u8).max(1));
            self.finish_round = current_rnd.min(m.scheduled_rounds);
        }

        self.screen = Screen::FinishMatch;
        self.status_message =
            "Record Stoppage / Complete Fight: [Tab/↑/↓] field, [←/→] change option, [Enter] confirm."
                .to_string();
    }

    pub fn cycle_finish_winner_next(&mut self) {
        self.finish_winner = match self.finish_winner {
            Some(FighterSide::FighterA) => Some(FighterSide::FighterB),
            Some(FighterSide::FighterB) => None,
            None => Some(FighterSide::FighterA),
        };
    }

    pub fn cycle_finish_winner_prev(&mut self) {
        self.finish_winner = match self.finish_winner {
            Some(FighterSide::FighterA) => None,
            Some(FighterSide::FighterB) => Some(FighterSide::FighterA),
            None => Some(FighterSide::FighterB),
        };
    }

    pub fn submit_finish_match(&mut self) {
        let Some(match_id) = self.finish_target_match_id else {
            self.screen = self.previous_screen;
            return;
        };

        let outcome = MatchOutcome {
            method: self.finish_method,
            winner: self.finish_winner,
            round: self.finish_round,
        };

        match self.service.finish_match(match_id, outcome) {
            Ok(updated) => {
                let summary = updated
                    .outcome_summary()
                    .unwrap_or_else(|| "Complete".to_string());
                if self
                    .current_match
                    .as_ref()
                    .map(|m| m.id == updated.id)
                    .unwrap_or(false)
                {
                    self.current_match = Some(updated);
                    if let Some(ref mut timer) = self.timer {
                        timer.phase = TimerPhase::Completed;
                        timer.remaining = Duration::ZERO;
                        timer.paused = true;
                        timer.last_prompt = None;
                    }
                }
                self.reload_matches();
                self.screen = self.previous_screen;
                self.status_message = format!(
                    "Fight marked COMPLETE — Result: {} (Timer stopped permanently)",
                    summary
                );
            }
            Err(e) => {
                self.status_message = format!("Failed to finish match: {}", e);
            }
        }
    }

    pub fn parse_input_date(&self) -> Option<NaiveDate> {
        let s = self.input_date.trim();
        NaiveDate::parse_from_str(s, "%d %b %Y")
            .or_else(|_| NaiveDate::parse_from_str(s, "%d %B %Y"))
            .or_else(|_| NaiveDate::parse_from_str(s, "%Y-%m-%d"))
            .ok()
    }

    pub fn preview_title(&self) -> String {
        let a = if self.input_fighter_a.trim().is_empty() {
            "Fighter A"
        } else {
            self.input_fighter_a.trim()
        };
        let b = if self.input_fighter_b.trim().is_empty() {
            "Fighter B"
        } else {
            self.input_fighter_b.trim()
        };
        let date_str = self
            .parse_input_date()
            .map(|d| d.format("%d %b %Y").to_string())
            .unwrap_or_else(|| "Invalid Date (use DD Mon YYYY, e.g. 01 Jan 2026)".to_string());
        format!("{} v {} ({})", a, b, date_str)
    }

    pub fn submit_new_match(&mut self) {
        let a = self.input_fighter_a.trim().to_string();
        let b = self.input_fighter_b.trim().to_string();
        if a.is_empty() || b.is_empty() {
            self.status_message = "Both fighter names are required.".to_string();
            return;
        }
        let Some(date) = self.parse_input_date() else {
            self.status_message =
                "Invalid date! Use format '01 Jan 2026' or '2026-01-01'.".to_string();
            return;
        };

        match self.service.create_match(a, b, date, self.input_rounds) {
            Ok(created) => {
                self.reload_matches();
                self.start_scoring_match(created);
            }
            Err(e) => {
                self.status_message = format!("Failed to save match: {}", e);
            }
        }
    }

    pub fn start_scoring_match(&mut self, boxing_match: BoxingMatch) {
        let mut timer = MatchTimer::new(
            boxing_match.scheduled_rounds,
            boxing_match.round_duration_secs,
            boxing_match.break_duration_secs,
        );

        if boxing_match.is_complete() {
            timer.phase = TimerPhase::Completed;
            timer.remaining = Duration::ZERO;
            timer.paused = true;
            timer.last_prompt = None;
            let res = boxing_match
                .outcome_summary()
                .map(|s| format!(" — Result: {}", s))
                .unwrap_or_default();
            self.status_message = format!(
                "Viewing completed scorecard for {}{}. Press [F] to edit result or [U] to reopen.",
                boxing_match.title(),
                res
            );
            self.alert_until = None;
        } else {
            match &boxing_match.active_round {
                Some(active) if active.is_walkout() => {
                    timer.phase = TimerPhase::Ready { next_round: 1 };
                    timer.last_prompt = None;
                    self.status_message = format!(
                        "🎵 Pre-Fight Walkout for {} — Score the ring walks, then press [Enter/Space] to start Round 1!",
                        boxing_match.title()
                    );
                }
                Some(active) => {
                    timer.phase = TimerPhase::InRound {
                        round: active.round_number,
                    };
                    self.status_message = format!(
                        "Scoring {} — [1/2]: -1 pt | [!/Shift+1] or [@/Shift+2]: +1 pt | [Enter]: Lock | [F]: Stoppage/Finish",
                        boxing_match.title()
                    );
                }
                None => {
                    timer.phase = TimerPhase::Completed;
                    timer.remaining = Duration::ZERO;
                    timer.paused = true;
                }
            }
            self.alert_until = Some(Instant::now() + Duration::from_secs(2));
        }

        self.current_match = Some(boxing_match);
        self.timer = Some(timer);
        self.screen = Screen::Scoring;
    }

    pub fn deduct_point(&mut self, side: FighterSide) {
        let Some(ref mut m) = self.current_match else {
            return;
        };
        if m.deduct_point(side) {
            let _ = self.service.save_match(m);
            let (fighter_name, restore_hint) = match side {
                FighterSide::FighterA => (&m.fighter_a, "[!] or [Shift+A] / [W]"),
                FighterSide::FighterB => (&m.fighter_b, "[@] or [Shift+D] / [E]"),
            };
            self.status_message = format!(
                "Deducted 1 point from {} (Press {} to restore)",
                fighter_name, restore_hint
            );
            self.reload_matches();
        } else if m.is_complete() {
            self.status_message =
                "Match is complete! Press [U] to reopen or [F] to change the result.".to_string();
        }
    }

    pub fn restore_point(&mut self, side: FighterSide) {
        let Some(ref mut m) = self.current_match else {
            return;
        };
        if m.restore_point(side) {
            let _ = self.service.save_match(m);
            let fighter_name = match side {
                FighterSide::FighterA => &m.fighter_a,
                FighterSide::FighterB => &m.fighter_b,
            };
            self.status_message = format!("Restored 1 point to {}", fighter_name);
            self.reload_matches();
        } else if !m.is_complete() {
            self.status_message = "Fighter is already at the maximum 10 points.".to_string();
        }
    }

    pub fn lock_current_round(&mut self) {
        let Some(ref mut m) = self.current_match else {
            return;
        };
        if let Some(locked) = m.lock_round() {
            let _ = self.service.save_match(m);
            if locked.is_walkout() {
                if let Some(ref mut timer) = self.timer {
                    if let TimerPhase::Ready { next_round } = timer.phase {
                        timer.phase = TimerPhase::InRound { round: next_round };
                        timer.remaining = timer.round_duration;
                        timer.paused = false;
                        timer.last_prompt = Some(TimerPrompt::RoundStarted { round: next_round });
                    }
                }
                self.alert_until = Some(Instant::now() + Duration::from_secs(3));
                self.status_message = format!(
                    "Locked Walkout Score ({} {} - {} {})! 🔔 BELL — Round 1 has started!",
                    m.fighter_a, locked.fighter_a_score, locked.fighter_b_score, m.fighter_b
                );
            } else if let Some(next_active) = &m.active_round {
                self.status_message = format!(
                    "Locked Round {} ({} {} - {} {}). Now scoring Round {}.",
                    locked.round_number,
                    m.fighter_a,
                    locked.fighter_a_score,
                    locked.fighter_b_score,
                    m.fighter_b,
                    next_active.round_number
                );
            } else {
                if let Some(ref mut timer) = self.timer {
                    timer.phase = TimerPhase::Completed;
                    timer.remaining = Duration::ZERO;
                    timer.paused = true;
                    timer.last_prompt = None;
                }
                let (tot_a, tot_b) = m.total_scores();
                self.status_message = format!(
                    "Locked final Round {}! Official Final Score: {} {} - {} {} (Match Complete)",
                    locked.round_number, m.fighter_a, tot_a, tot_b, m.fighter_b
                );
            }
            self.reload_matches();
        } else {
            self.status_message =
                "Match is complete! Press [F] to edit result or [U] to undo the last round."
                    .to_string();
        }
    }

    pub fn undo_last_locked_round(&mut self) {
        let Some(ref mut m) = self.current_match else {
            return;
        };
        if m.undo_last_round() {
            let _ = self.service.save_match(m);
            if let Some(active) = &m.active_round {
                if active.is_walkout() {
                    if let Some(ref mut timer) = self.timer {
                        timer.phase = TimerPhase::Ready { next_round: 1 };
                        timer.remaining = timer.round_duration;
                    }
                    self.status_message =
                        "🎵 Scoring Pre-Fight Walkout! Use [1/2] to deduct, [!/Shift+1] or [@/Shift+2] to restore, [Enter] to lock."
                            .to_string();
                } else {
                    if let Some(ref mut timer) = self.timer {
                        timer.phase = TimerPhase::InRound {
                            round: active.round_number,
                        };
                        timer.remaining = timer.round_duration;
                        timer.paused = false;
                    }
                    self.status_message =
                        format!("Re-opened Round {} for editing.", active.round_number);
                }
            }
            self.reload_matches();
        } else {
            self.status_message = "Already at Pre-Fight Walkout.".to_string();
        }
    }

    pub fn open_walkout_stage(&mut self) {
        let Some(ref mut m) = self.current_match else {
            return;
        };
        m.open_walkout();
        let _ = self.service.save_match(m);
        if m.locked_rounds.is_empty() {
            if let Some(ref mut timer) = self.timer {
                timer.phase = TimerPhase::Ready { next_round: 1 };
                timer.remaining = timer.round_duration;
            }
        }
        self.status_message =
            "🎵 Scoring Pre-Fight Walkout! Use [1/2] to deduct, [!/Shift+1] or [@/Shift+2] to restore, [Enter] to lock."
                .to_string();
        self.reload_matches();
    }

    pub fn on_tick(&mut self, delta: Duration) {
        if self.screen == Screen::Scoring {
            if let Some(ref mut timer) = self.timer {
                if let Some(prompt) = timer.tick(delta) {
                    self.status_message = prompt.message();
                    self.alert_until = Some(Instant::now() + Duration::from_secs(4));
                }
            }
        }
    }

    pub fn is_alert_active(&self) -> bool {
        self.alert_until
            .map(|until| Instant::now() < until)
            .unwrap_or(false)
    }
}
