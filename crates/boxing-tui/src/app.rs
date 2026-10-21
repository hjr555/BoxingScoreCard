use boxing_core::{
    fighter_last_name, BoxingMatch, FighterSide, FinishMethod, JsonMatchRepository, MatchOutcome,
    MatchTimer, ScoringService, TimerPhase, DEFAULT_SCHEDULED_ROUNDS,
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
    pub confirm_delete_id: Option<Uuid>,
    pub clickable_links: Vec<(u16, u16, u16, String)>,

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
    pub finish_cards_summary: String,
    pub finish_max_rounds: u8,
    pub finish_field: FinishField,
    pub finish_method: FinishMethod,
    pub finish_winner: Option<FighterSide>,
    pub finish_round: u8,
}

impl App {
    pub fn new(service: ScoringService<JsonMatchRepository>) -> Self {
        let (matches, init_status) = match service.list_matches() {
            Ok(list) => (list, "Ready".to_string()),
            Err(e) => (
                Vec::new(),
                format!("Warning — failed to read matches file: {}", e),
            ),
        };
        let today = Local::now().date_naive();
        Self {
            service,
            screen: Screen::Home,
            previous_screen: Screen::Home,
            matches,
            selected_match_idx: 0,
            current_match: None,
            timer: None,
            status_message: init_status,
            alert_until: None,
            should_quit: false,
            confirm_delete_id: None,
            clickable_links: Vec::new(),
            form_field: FormField::FighterA,
            input_fighter_a: String::new(),
            input_fighter_b: String::new(),
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
            finish_cards_summary: String::new(),
            finish_max_rounds: DEFAULT_SCHEDULED_ROUNDS,
            finish_field: FinishField::Method,
            finish_method: FinishMethod::Tko,
            finish_winner: Some(FighterSide::FighterA),
            finish_round: 1,
        }
    }

    pub fn reload_matches(&mut self) {
        match self.service.list_matches() {
            Ok(list) => {
                self.matches = list;
                if self.selected_match_idx >= self.matches.len() && !self.matches.is_empty() {
                    self.selected_match_idx = self.matches.len() - 1;
                }
            }
            Err(e) => {
                self.status_message = format!("Storage error: {}", e);
                self.alert_until = Some(Instant::now() + Duration::from_secs(4));
            }
        }
    }

    /// Saves `current_match` to disk and updates the in-memory `matches` list without a redundant file read.
    /// Surfaces any I/O error to `status_message`.
    fn save_current_match(&mut self) -> bool {
        let Some(ref m) = self.current_match else {
            return false;
        };
        match self.service.save_match(m) {
            Ok(()) => {
                if let Some(existing) = self.matches.iter_mut().find(|item| item.id == m.id) {
                    *existing = m.clone();
                } else {
                    self.matches.push(m.clone());
                }
                true
            }
            Err(e) => {
                self.status_message = format!("Save failed: {}", e);
                self.alert_until = Some(Instant::now() + Duration::from_secs(4));
                false
            }
        }
    }

    pub fn request_delete_selected_match(&mut self) {
        let Some(m) = self.matches.get(self.selected_match_idx) else {
            self.confirm_delete_id = None;
            return;
        };
        let id = m.id;
        let title = m.title();

        if self.confirm_delete_id == Some(id) {
            self.confirm_delete_id = None;
            match self.service.delete_match(id) {
                Ok(()) => {
                    self.reload_matches();
                    self.status_message = format!("Deleted {}", title);
                }
                Err(e) => {
                    self.status_message = format!("Delete failed: {}", e);
                    self.alert_until = Some(Instant::now() + Duration::from_secs(4));
                }
            }
        } else {
            self.confirm_delete_id = Some(id);
            self.status_message = format!(
                "Delete '{}'? [Y]/[D] Confirm  [Esc] Cancel",
                title
            );
        }
    }

    pub fn open_new_match_form(&mut self) {
        self.confirm_delete_id = None;
        self.form_field = FormField::FighterA;
        self.input_fighter_a.clear();
        self.input_fighter_b.clear();
        self.input_date = Local::now().date_naive().format("%d %b %Y").to_string();
        self.input_rounds = DEFAULT_SCHEDULED_ROUNDS;
        self.screen = Screen::NewMatch;
        self.status_message = "New match".to_string();
    }

    /// Opens the BoxRec ID modal for either the active match or selected match.
    pub fn open_boxrec_editor(&mut self) {
        self.confirm_delete_id = None;
        let target = match self.screen {
            Screen::Scoring => self.current_match.clone(),
            Screen::Home => self.matches.get(self.selected_match_idx).cloned(),
            _ => None,
        };

        let Some(m) = target else {
            self.status_message = "No match selected.".to_string();
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
        self.status_message = "Edit BoxRec IDs".to_string();
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
                self.status_message = format!("Saved BoxRec IDs for {}", updated.title());
            }
            Err(e) => {
                self.status_message = format!("Failed to save BoxRec IDs: {}", e);
            }
        }
    }

    /// Opens the Finish / Stoppage modal (TKO, RTD/Towel, KO, DQ, TD, Decision).
    pub fn open_finish_match_modal(&mut self) {
        self.open_finish_match_modal_with(None);
    }

    pub fn open_finish_match_modal_with(&mut self, preset_method: Option<FinishMethod>) {
        self.confirm_delete_id = None;
        let target = match self.screen {
            Screen::Scoring => self.current_match.clone(),
            Screen::Home => self.matches.get(self.selected_match_idx).cloned(),
            _ => None,
        };

        let Some(m) = target else {
            self.status_message = "No match selected.".to_string();
            return;
        };

        self.previous_screen = self.screen;
        self.finish_target_match_id = Some(m.id);
        self.finish_target_title = m.title();
        self.finish_fighter_a_name = m.fighter_a.clone();
        self.finish_fighter_b_name = m.fighter_b.clone();
        self.finish_max_rounds = m.scheduled_rounds;
        self.finish_field = FinishField::Method;

        let (tot_a, tot_b) = m.total_scores();
        self.finish_cards_summary = format!(
            "{} {} - {} {} ({} {})",
            m.fighter_a,
            tot_a,
            tot_b,
            m.fighter_b,
            m.locked_rounds.len(),
            if m.locked_rounds.len() == 1 {
                "rd"
            } else {
                "rds"
            }
        );

        if let Some(existing) = &m.outcome {
            self.finish_method = preset_method.unwrap_or(existing.method);
            self.finish_winner = existing.winner;
            self.finish_round = existing.round;
        } else {
            self.finish_method = preset_method.unwrap_or(FinishMethod::Tko);
            self.finish_winner = if tot_b > tot_a {
                Some(FighterSide::FighterB)
            } else {
                Some(FighterSide::FighterA)
            };

            // Determine the current round smartly from the active round, resumed round, or running timer
            let timer_round = if self.screen == Screen::Scoring {
                self.timer.as_ref().and_then(|t| match t.phase {
                    TimerPhase::InRound { round } => Some(round),
                    TimerPhase::InBreak { after_round, .. } => Some(after_round),
                    _ => None,
                })
            } else {
                None
            };

            let current_rnd = match &m.active_round {
                Some(r) if !r.is_walkout() => r.round_number,
                _ => m
                    .resume_round
                    .as_ref()
                    .map(|r| r.round_number)
                    .or(timer_round)
                    .unwrap_or_else(|| ((m.locked_rounds.len() as u8) + 1).min(m.scheduled_rounds)),
            };
            self.finish_round = current_rnd.clamp(1, m.scheduled_rounds);
        }

        self.screen = Screen::FinishMatch;
        self.status_message = "Finish match".to_string();
    }

    pub fn preview_finish_outcome(&self) -> String {
        match self.finish_winner {
            Some(side) => {
                let winner_name = match side {
                    FighterSide::FighterA => fighter_last_name(&self.finish_fighter_a_name),
                    FighterSide::FighterB => fighter_last_name(&self.finish_fighter_b_name),
                };
                if self.finish_method == FinishMethod::Decision {
                    format!("{} ({})", winner_name, self.finish_method.short_label())
                } else {
                    format!(
                        "{} by {} (R{})",
                        winner_name,
                        self.finish_method.short_label(),
                        self.finish_round
                    )
                }
            }
            None => {
                if self.finish_method == FinishMethod::Decision {
                    format!("Draw ({})", self.finish_method.short_label())
                } else {
                    format!(
                        "Draw ({} R{})",
                        self.finish_method.short_label(),
                        self.finish_round
                    )
                }
            }
        }
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

        // If finishing the currently active match in Scoring, apply directly to preserve in-memory state
        if self
            .current_match
            .as_ref()
            .map(|m| m.id == match_id)
            .unwrap_or(false)
        {
            if let Some(ref mut m) = self.current_match {
                m.finish_match(outcome);
            }
            if self.save_current_match() {
                let summary = self
                    .current_match
                    .as_ref()
                    .and_then(|m| m.outcome_summary())
                    .unwrap_or_else(|| "Complete".to_string());
                if let Some(ref mut timer) = self.timer {
                    timer.phase = TimerPhase::Completed;
                    timer.remaining = Duration::ZERO;
                    timer.paused = true;
                    timer.last_prompt = None;
                }
                self.screen = self.previous_screen;
                self.status_message = format!("Result: {}", summary);
            }
            return;
        }

        match self.service.finish_match(match_id, outcome) {
            Ok(updated) => {
                let summary = updated
                    .outcome_summary()
                    .unwrap_or_else(|| "Complete".to_string());
                self.reload_matches();
                self.screen = self.previous_screen;
                self.status_message = format!("Result: {}", summary);
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
            "Fighter 1"
        } else {
            self.input_fighter_a.trim()
        };
        let b = if self.input_fighter_b.trim().is_empty() {
            "Fighter 2"
        } else {
            self.input_fighter_b.trim()
        };
        let date_str = self
            .parse_input_date()
            .map(|d| d.format("%d %b %Y").to_string())
            .unwrap_or_else(|| "Invalid Date (e.g. 01 Jan 2026)".to_string());
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
                "Invalid date (use '01 Jan 2026' or '2026-01-01').".to_string();
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
        self.confirm_delete_id = None;
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
            self.status_message = boxing_match
                .outcome_summary()
                .unwrap_or_else(|| "Match complete".to_string());
            self.alert_until = None;
        } else {
            match &boxing_match.active_round {
                Some(active) if active.is_walkout() => {
                    let next_rnd = boxing_match
                        .resume_round
                        .as_ref()
                        .map(|r| r.round_number)
                        .unwrap_or(1);
                    timer.phase = TimerPhase::Ready {
                        next_round: next_rnd,
                    };
                    timer.last_prompt = None;
                    self.status_message = "Walkout — [Enter] Lock & start R1".to_string();
                }
                Some(active) => {
                    timer.sync_to_round(active.round_number);
                    self.status_message = format!("Scoring Round {}", active.round_number);
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
            let fighter_name = match side {
                FighterSide::FighterA => m.fighter_a.clone(),
                FighterSide::FighterB => m.fighter_b.clone(),
            };
            let restore_hint = match side {
                FighterSide::FighterA => "[!/W]",
                FighterSide::FighterB => "[@/E]",
            };
            if self.save_current_match() {
                self.status_message = format!("-1 {} ({} restore)", fighter_name, restore_hint);
            }
        } else if m.is_complete() {
            self.status_message = "Match complete — [U]ndo or [F]inish".to_string();
        }
    }

    pub fn restore_point(&mut self, side: FighterSide) {
        let Some(ref mut m) = self.current_match else {
            return;
        };
        if m.restore_point(side) {
            let fighter_name = match side {
                FighterSide::FighterA => m.fighter_a.clone(),
                FighterSide::FighterB => m.fighter_b.clone(),
            };
            if self.save_current_match() {
                self.status_message = format!("+1 {}", fighter_name);
            }
        } else if !m.is_complete() {
            self.status_message = "Already at 10 pts".to_string();
        }
    }

    pub fn lock_current_round(&mut self) {
        let Some(ref mut m) = self.current_match else {
            return;
        };
        let Some(locked) = m.lock_round() else {
            self.status_message = "Match complete — [U]ndo or [F]inish".to_string();
            return;
        };

        let fighter_a = m.fighter_a.clone();
        let fighter_b = m.fighter_b.clone();
        let next_round_num = m.active_round.as_ref().map(|r| r.round_number);
        let (tot_a, tot_b) = m.total_scores();
        let outcome_summary = m.outcome_summary();

        if !self.save_current_match() {
            return;
        }

        if locked.is_walkout() {
            if let Some(next_rnd) = next_round_num {
                if let Some(ref mut timer) = self.timer {
                    if matches!(timer.phase, TimerPhase::Ready { .. }) {
                        timer.sync_to_round(next_rnd);
                        timer.paused = false;
                    }
                }
                self.alert_until = Some(Instant::now() + Duration::from_secs(3));
                self.status_message = format!(
                    "Walkout {}-{} locked — 🔔 Round {}",
                    locked.fighter_a_score, locked.fighter_b_score, next_rnd
                );
            } else {
                if let Some(ref mut timer) = self.timer {
                    timer.phase = TimerPhase::Completed;
                    timer.remaining = Duration::ZERO;
                    timer.paused = true;
                    timer.last_prompt = None;
                }
                self.status_message = format!(
                    "Walkout {}-{} — {}",
                    locked.fighter_a_score,
                    locked.fighter_b_score,
                    outcome_summary.unwrap_or_else(|| "Complete".to_string())
                );
            }
        } else if let Some(next_rnd) = next_round_num {
            if let Some(ref mut timer) = self.timer {
                // Keep timer aligned with the newly opened round unless currently in the break leading into it
                let in_matching_break = matches!(
                    timer.phase,
                    TimerPhase::InBreak { next_round, .. } if next_round == next_rnd
                );
                if !in_matching_break {
                    timer.sync_to_round(next_rnd);
                }
            }
            self.status_message = format!(
                "R{} locked ({}-{}) — Scoring R{}",
                locked.round_number,
                locked.fighter_a_score,
                locked.fighter_b_score,
                next_rnd
            );
        } else {
            if let Some(ref mut timer) = self.timer {
                timer.phase = TimerPhase::Completed;
                timer.remaining = Duration::ZERO;
                timer.paused = true;
                timer.last_prompt = None;
            }
            self.status_message = format!(
                "Final: {} {} - {} {}",
                fighter_a, tot_a, tot_b, fighter_b
            );
        }
    }

    pub fn undo_last_locked_round(&mut self) {
        let Some(ref mut m) = self.current_match else {
            return;
        };
        if m.undo_last_round() {
            let active_copy = m.active_round.clone();
            let outcome_summary = m.outcome_summary();
            if !self.save_current_match() {
                return;
            }
            if let Some(active) = active_copy {
                if active.is_walkout() {
                    if let Some(ref mut timer) = self.timer {
                        timer.phase = TimerPhase::Ready { next_round: 1 };
                        timer.remaining = timer.round_duration;
                        timer.last_prompt = None;
                    }
                    self.status_message = "Re-opened Walkout".to_string();
                } else {
                    if let Some(ref mut timer) = self.timer {
                        timer.sync_to_round(active.round_number);
                        timer.paused = false;
                    }
                    self.status_message = format!("Re-opened Round {}", active.round_number);
                }
            } else if let Some(summary) = outcome_summary {
                self.status_message = format!("Result: {}", summary);
            }
        } else {
            self.status_message = "Already at Walkout".to_string();
        }
    }

    pub fn open_walkout_stage(&mut self) {
        let Some(ref mut m) = self.current_match else {
            return;
        };
        m.open_walkout();
        let has_locked_or_resumed =
            !m.locked_rounds.is_empty() || m.resume_round.is_some() || m.outcome.is_some();
        if !self.save_current_match() {
            return;
        }
        if !has_locked_or_resumed {
            if let Some(ref mut timer) = self.timer {
                timer.phase = TimerPhase::Ready { next_round: 1 };
                timer.remaining = timer.round_duration;
                timer.last_prompt = None;
            }
        }
        self.status_message = "Walkout — [Enter] Lock".to_string();
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
