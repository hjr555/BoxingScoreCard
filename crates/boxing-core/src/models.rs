use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const DEFAULT_INITIAL_POINTS: u8 = 10;
pub const DEFAULT_SCHEDULED_ROUNDS: u8 = 12;
pub const DEFAULT_ROUND_DURATION_SECS: u32 = 180; // 3 minutes
pub const DEFAULT_BREAK_DURATION_SECS: u32 = 60;  // 1 minute

/// Round number `0` represents the pre-fight walkout stage before Round 1.
pub const WALKOUT_ROUND_NUMBER: u8 = 0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FighterSide {
    FighterA,
    FighterB,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FinishMethod {
    /// Referee stops the fight (Technical Knockout)
    Tko,
    /// Corner throws in the towel / fighter retires between rounds
    Rtd,
    /// 10-count Knockout
    Ko,
    /// Disqualification
    Dq,
    /// Technical Decision (accidental foul, goes to cards early)
    Td,
    /// Full distance / Points Decision
    Decision,
}

impl FinishMethod {
    pub fn short_label(self) -> &'static str {
        match self {
            FinishMethod::Tko => "TKO",
            FinishMethod::Rtd => "RTD",
            FinishMethod::Ko => "KO",
            FinishMethod::Dq => "DQ",
            FinishMethod::Td => "TD",
            FinishMethod::Decision => "DEC",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            FinishMethod::Tko => "TKO (Referee Stoppage)",
            FinishMethod::Rtd => "RTD (Towel Thrown In / Corner Retirement)",
            FinishMethod::Ko => "KO (Knockout)",
            FinishMethod::Dq => "DQ (Disqualification)",
            FinishMethod::Td => "TD (Technical Decision)",
            FinishMethod::Decision => "Decision / Mark Complete",
        }
    }

    pub fn next(self) -> Self {
        match self {
            FinishMethod::Tko => FinishMethod::Rtd,
            FinishMethod::Rtd => FinishMethod::Ko,
            FinishMethod::Ko => FinishMethod::Dq,
            FinishMethod::Dq => FinishMethod::Td,
            FinishMethod::Td => FinishMethod::Decision,
            FinishMethod::Decision => FinishMethod::Tko,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            FinishMethod::Tko => FinishMethod::Decision,
            FinishMethod::Rtd => FinishMethod::Tko,
            FinishMethod::Ko => FinishMethod::Rtd,
            FinishMethod::Dq => FinishMethod::Ko,
            FinishMethod::Td => FinishMethod::Dq,
            FinishMethod::Decision => FinishMethod::Td,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MatchOutcome {
    pub method: FinishMethod,
    pub winner: Option<FighterSide>,
    pub round: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoundScore {
    pub round_number: u8,
    pub fighter_a_score: u8,
    pub fighter_b_score: u8,
}

impl RoundScore {
    pub fn is_walkout(&self) -> bool {
        self.round_number == WALKOUT_ROUND_NUMBER
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActiveRound {
    /// `0` for the Pre-Fight Walkout, `1..=scheduled_rounds` for official boxing rounds.
    pub round_number: u8,
    pub fighter_a_score: u8,
    pub fighter_b_score: u8,
}

impl ActiveRound {
    pub fn new(round_number: u8) -> Self {
        Self {
            round_number,
            fighter_a_score: DEFAULT_INITIAL_POINTS,
            fighter_b_score: DEFAULT_INITIAL_POINTS,
        }
    }

    pub fn is_walkout(&self) -> bool {
        self.round_number == WALKOUT_ROUND_NUMBER
    }

    /// Deducts 1 point from the specified fighter (minimum 0).
    /// Returns true if the score was changed.
    pub fn deduct_point(&mut self, side: FighterSide) -> bool {
        let score = match side {
            FighterSide::FighterA => &mut self.fighter_a_score,
            FighterSide::FighterB => &mut self.fighter_b_score,
        };
        if *score > 0 {
            *score -= 1;
            true
        } else {
            false
        }
    }

    /// Restores 1 point to the specified fighter in case of error (maximum 10).
    /// Returns true if the score was changed.
    pub fn restore_point(&mut self, side: FighterSide) -> bool {
        let score = match side {
            FighterSide::FighterA => &mut self.fighter_a_score,
            FighterSide::FighterB => &mut self.fighter_b_score,
        };
        if *score < DEFAULT_INITIAL_POINTS {
            *score += 1;
            true
        } else {
            false
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BoxingMatch {
    pub id: Uuid,
    pub fighter_a: String,
    pub fighter_b: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fighter_a_boxrec_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fighter_b_boxrec_id: Option<String>,
    pub date: NaiveDate,
    pub scheduled_rounds: u8,
    pub round_duration_secs: u32,
    pub break_duration_secs: u32,
    #[serde(default)]
    pub walkout_score: Option<RoundScore>,
    pub locked_rounds: Vec<RoundScore>,
    pub active_round: Option<ActiveRound>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub outcome: Option<MatchOutcome>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resume_round: Option<ActiveRound>,
}

impl BoxingMatch {
    pub fn new(fighter_a: impl Into<String>, fighter_b: impl Into<String>, date: NaiveDate) -> Self {
        Self::with_config(
            fighter_a,
            fighter_b,
            date,
            DEFAULT_SCHEDULED_ROUNDS,
            DEFAULT_ROUND_DURATION_SECS,
            DEFAULT_BREAK_DURATION_SECS,
        )
    }

    pub fn with_config(
        fighter_a: impl Into<String>,
        fighter_b: impl Into<String>,
        date: NaiveDate,
        scheduled_rounds: u8,
        round_duration_secs: u32,
        break_duration_secs: u32,
    ) -> Self {
        let scheduled = scheduled_rounds.max(1);
        Self {
            id: Uuid::new_v4(),
            fighter_a: fighter_a.into().trim().to_string(),
            fighter_b: fighter_b.into().trim().to_string(),
            fighter_a_boxrec_id: None,
            fighter_b_boxrec_id: None,
            date,
            scheduled_rounds: scheduled,
            round_duration_secs,
            break_duration_secs,
            walkout_score: None,
            locked_rounds: Vec::with_capacity(scheduled as usize),
            // Start at Round 0 (Pre-Fight Walkout)
            active_round: Some(ActiveRound::new(WALKOUT_ROUND_NUMBER)),
            outcome: None,
            resume_round: None,
        }
    }

    /// Formats the match title strictly as `Joshua v Fury (01 Jan 2026)`.
    pub fn title(&self) -> String {
        format!(
            "{} v {} ({})",
            self.fighter_a,
            self.fighter_b,
            self.date.format("%d %b %Y")
        )
    }

    /// Returns a human-readable summary of the match outcome if finished (e.g. `Dubois by TKO (R5)`).
    pub fn outcome_summary(&self) -> Option<String> {
        let out = self.outcome.as_ref()?;
        match out.winner {
            Some(side) => {
                let winner_name = match side {
                    FighterSide::FighterA => self.fighter_a.as_str(),
                    FighterSide::FighterB => self.fighter_b.as_str(),
                };
                if out.method == FinishMethod::Decision {
                    Some(format!("{} ({})", winner_name, out.method.short_label()))
                } else {
                    Some(format!(
                        "{} by {} (R{})",
                        winner_name,
                        out.method.short_label(),
                        out.round
                    ))
                }
            }
            None => {
                if out.method == FinishMethod::Decision {
                    Some(format!("Draw ({})", out.method.short_label()))
                } else {
                    Some(format!("Draw ({} R{})", out.method.short_label(), out.round))
                }
            }
        }
    }

    /// Returns the optional BoxRec ID for the given fighter side.
    pub fn boxrec_id(&self, side: FighterSide) -> Option<&str> {
        match side {
            FighterSide::FighterA => self.fighter_a_boxrec_id.as_deref(),
            FighterSide::FighterB => self.fighter_b_boxrec_id.as_deref(),
        }
    }

    /// Sets or clears the BoxRec ID for the given fighter after the fact.
    pub fn set_boxrec_id(&mut self, side: FighterSide, boxrec_id: impl Into<String>) {
        let trimmed = boxrec_id.into().trim().to_string();
        let value = if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        };
        match side {
            FighterSide::FighterA => self.fighter_a_boxrec_id = value,
            FighterSide::FighterB => self.fighter_b_boxrec_id = value,
        }
    }

    /// Deducts a point from the specified fighter in the active round (or walkout).
    pub fn deduct_point(&mut self, side: FighterSide) -> bool {
        if let Some(active) = &mut self.active_round {
            active.deduct_point(side)
        } else {
            false
        }
    }

    /// Restores a point to the specified fighter in the active round (or walkout, up to 10).
    pub fn restore_point(&mut self, side: FighterSide) -> bool {
        if let Some(active) = &mut self.active_round {
            active.restore_point(side)
        } else {
            false
        }
    }

    /// Opens or re-opens the Pre-Fight Walkout (`round_number == 0`) for scoring,
    /// preserving any in-progress official round or completed outcome.
    pub fn open_walkout(&mut self) {
        if let Some(current) = self.active_round.take() {
            if !current.is_walkout() {
                self.resume_round = Some(current);
            }
        }
        let (a, b) = self
            .walkout_score
            .take()
            .map(|w| (w.fighter_a_score, w.fighter_b_score))
            .unwrap_or((DEFAULT_INITIAL_POINTS, DEFAULT_INITIAL_POINTS));
        self.active_round = Some(ActiveRound {
            round_number: WALKOUT_ROUND_NUMBER,
            fighter_a_score: a,
            fighter_b_score: b,
        });
    }

    /// Next official round number that has not been locked yet (`1..=scheduled_rounds`), or `None` if all rounds are locked.
    pub fn next_unlocked_round_number(&self) -> Option<u8> {
        let next = (self.locked_rounds.len() as u8) + 1;
        if next <= self.scheduled_rounds {
            Some(next)
        } else {
            None
        }
    }

    /// Locks the current active stage (Pre-Fight Walkout or Round 1..=N) in and advances.
    /// Returns the locked `RoundScore` if there was an active round.
    pub fn lock_round(&mut self) -> Option<RoundScore> {
        let active = self.active_round.take()?;
        let locked = RoundScore {
            round_number: active.round_number,
            fighter_a_score: active.fighter_a_score,
            fighter_b_score: active.fighter_b_score,
        };

        if active.is_walkout() {
            self.walkout_score = Some(locked.clone());
            if self.outcome.is_some() {
                self.active_round = None;
            } else if let Some(resumed) = self.resume_round.take() {
                self.active_round = Some(resumed);
            } else {
                self.active_round = self.next_unlocked_round_number().map(ActiveRound::new);
            }
        } else {
            self.resume_round = None;
            self.locked_rounds.push(locked.clone());
            self.active_round = self.next_unlocked_round_number().map(ActiveRound::new);
            if self.active_round.is_none() && self.outcome.is_none() {
                let (tot_a, tot_b) = self.total_scores();
                let winner = if tot_a > tot_b {
                    Some(FighterSide::FighterA)
                } else if tot_b > tot_a {
                    Some(FighterSide::FighterB)
                } else {
                    None
                };
                self.outcome = Some(MatchOutcome {
                    method: FinishMethod::Decision,
                    winner,
                    round: self.scheduled_rounds,
                });
            }
        }

        Some(locked)
    }

    /// Marks the fight as complete (e.g. via TKO/referee stoppage, RTD/towel thrown in, KO, or Decision).
    /// Clears `active_round` so the match is permanently marked complete and will not restart the timer.
    pub fn finish_match(&mut self, outcome: MatchOutcome) {
        if let Some(active) = self.active_round.take() {
            if active.is_walkout() {
                self.walkout_score = Some(RoundScore {
                    round_number: WALKOUT_ROUND_NUMBER,
                    fighter_a_score: active.fighter_a_score,
                    fighter_b_score: active.fighter_b_score,
                });
            } else {
                self.resume_round = Some(active);
            }
        }
        self.outcome = Some(outcome);
    }

    /// Undoes a stoppage/completion or the last locked round (or steps back from Round 1 into Walkout).
    pub fn undo_last_round(&mut self) -> bool {
        // If we opened walkout mid-match, undo returns to the resumed round (or completed state)
        if self
            .active_round
            .as_ref()
            .map(|r| r.is_walkout())
            .unwrap_or(false)
        {
            if let Some(resumed) = self.resume_round.take() {
                self.active_round = Some(resumed);
                return true;
            }
            if self.outcome.is_some() {
                self.active_round = None;
                return true;
            }
            return false;
        }

        // If the match was finished early (before all scheduled rounds were locked), reopen the interrupted round
        if self.outcome.is_some() && self.active_round.is_none() {
            self.outcome = None;
            if let Some(resumed) = self.resume_round.take() {
                self.active_round = Some(resumed);
                return true;
            }
            if let Some(next_round) = self.next_unlocked_round_number() {
                self.active_round = Some(ActiveRound::new(next_round));
                return true;
            }
        }

        if let Some(last) = self.locked_rounds.pop() {
            self.outcome = None;
            self.resume_round = None;
            self.active_round = Some(ActiveRound {
                round_number: last.round_number,
                fighter_a_score: last.fighter_a_score,
                fighter_b_score: last.fighter_b_score,
            });
            true
        } else if self
            .active_round
            .as_ref()
            .map(|r| !r.is_walkout())
            .unwrap_or(true)
        {
            self.outcome = None;
            self.resume_round = None;
            self.open_walkout();
            true
        } else {
            false
        }
    }

    /// Returns the cumulative official locked score totals `(fighter_a_total, fighter_b_total)`.
    /// Note: The fun Pre-Fight Walkout score is intentionally excluded from the official boxing total.
    pub fn total_scores(&self) -> (u16, u16) {
        self.locked_rounds.iter().fold((0u16, 0u16), |(a, b), r| {
            (a + r.fighter_a_score as u16, b + r.fighter_b_score as u16)
        })
    }

    /// Returns true if the match is marked complete (either via stoppage/outcome or all scheduled rounds locked).
    pub fn is_complete(&self) -> bool {
        self.active_round.is_none()
    }
}
