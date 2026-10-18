use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "phase", rename_all = "snake_case")]
pub enum TimerPhase {
    /// Before the opening bell of Round 1
    Ready { next_round: u8 },
    /// Active boxing round in progress
    InRound { round: u8 },
    /// Rest break between rounds
    InBreak { after_round: u8, next_round: u8 },
    /// All scheduled rounds have finished timing
    Completed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum TimerPrompt {
    RoundStarted {
        round: u8,
    },
    TenSecondWarning {
        phase: TimerPhase,
    },
    BreakStarted {
        after_round: u8,
        next_round: u8,
        #[serde(default = "default_break_secs")]
        break_duration_secs: u32,
    },
    FinalBell {
        final_round: u8,
    },
}

fn default_break_secs() -> u32 {
    crate::models::DEFAULT_BREAK_DURATION_SECS
}

impl TimerPrompt {
    pub fn message(&self) -> String {
        match self {
            TimerPrompt::RoundStarted { round } => {
                format!("🔔 Round {} started", round)
            }
            TimerPrompt::TenSecondWarning {
                phase: TimerPhase::InRound { round },
            } => format!("⏱ 10s left in Round {}", round),
            TimerPrompt::TenSecondWarning {
                phase: TimerPhase::InBreak { next_round, .. },
            } => format!("⏱ Seconds out — Round {} in 10s", next_round),
            TimerPrompt::TenSecondWarning { .. } => "⏱ 10s remaining".to_string(),
            TimerPrompt::BreakStarted {
                after_round,
                next_round: _,
                break_duration_secs,
            } => {
                let break_label = if *break_duration_secs == 60 {
                    "1m break".to_string()
                } else if *break_duration_secs % 60 == 0 {
                    format!("{}m break", break_duration_secs / 60)
                } else {
                    format!("{}s break", break_duration_secs)
                };
                format!(
                    "🔔 End of R{} — {} ([Enter] Lock R{})",
                    after_round, break_label, after_round
                )
            }
            TimerPrompt::FinalBell { final_round } => {
                format!("🔔 Final Bell! [Enter] Lock R{}", final_round)
            }
        }
    }
}

/// A loose, non-blocking round and break countdown timer.
/// It prompts the user when rounds start and breaks occur without restricting manual round lock-ins.
#[derive(Debug, Clone)]
pub struct MatchTimer {
    pub scheduled_rounds: u8,
    pub round_duration: Duration,
    pub break_duration: Duration,
    pub phase: TimerPhase,
    pub remaining: Duration,
    pub paused: bool,
    pub last_prompt: Option<TimerPrompt>,
    ten_sec_warned: bool,
}

impl MatchTimer {
    pub fn new(scheduled_rounds: u8, round_duration_secs: u32, break_duration_secs: u32) -> Self {
        let round_duration = Duration::from_secs(round_duration_secs.max(1) as u64);
        let break_duration = Duration::from_secs(break_duration_secs.max(1) as u64);
        Self {
            scheduled_rounds: scheduled_rounds.max(1),
            round_duration,
            break_duration,
            phase: TimerPhase::InRound { round: 1 },
            remaining: round_duration,
            paused: false,
            last_prompt: Some(TimerPrompt::RoundStarted { round: 1 }),
            ten_sec_warned: false,
        }
    }

    /// Synchronizes the timer to the start of a specific official round (`1..=scheduled_rounds`).
    pub fn sync_to_round(&mut self, round: u8) {
        let clamped = round.clamp(1, self.scheduled_rounds);
        self.phase = TimerPhase::InRound { round: clamped };
        self.remaining = self.round_duration;
        self.ten_sec_warned = false;
        self.last_prompt = Some(TimerPrompt::RoundStarted { round: clamped });
    }

    /// Starts or resumes the timer.
    pub fn toggle_pause(&mut self) {
        if let TimerPhase::Ready { next_round } = self.phase {
            self.sync_to_round(next_round);
            self.paused = false;
        } else if self.phase != TimerPhase::Completed {
            self.paused = !self.paused;
        }
    }

    /// Advances the timer by `delta`, returning a `TimerPrompt` if a transition or alert occurred.
    pub fn tick(&mut self, delta: Duration) -> Option<TimerPrompt> {
        if self.paused {
            return None;
        }

        match self.phase {
            TimerPhase::Ready { .. } | TimerPhase::Completed => None,
            TimerPhase::InRound { round } => {
                if self.remaining > delta {
                    self.remaining -= delta;
                    if !self.ten_sec_warned
                        && self.remaining <= Duration::from_secs(10)
                        && self.round_duration > Duration::from_secs(10)
                    {
                        self.ten_sec_warned = true;
                        let prompt = TimerPrompt::TenSecondWarning { phase: self.phase };
                        self.last_prompt = Some(prompt.clone());
                        return Some(prompt);
                    }
                    None
                } else if round < self.scheduled_rounds {
                    let overflow = delta.saturating_sub(self.remaining);
                    let next_round = round + 1;
                    self.phase = TimerPhase::InBreak {
                        after_round: round,
                        next_round,
                    };
                    self.remaining = self.break_duration.saturating_sub(overflow);
                    self.ten_sec_warned = false;
                    let prompt = TimerPrompt::BreakStarted {
                        after_round: round,
                        next_round,
                        break_duration_secs: self.break_duration.as_secs() as u32,
                    };
                    self.last_prompt = Some(prompt.clone());
                    Some(prompt)
                } else {
                    self.phase = TimerPhase::Completed;
                    self.remaining = Duration::ZERO;
                    self.ten_sec_warned = false;
                    let prompt = TimerPrompt::FinalBell { final_round: round };
                    self.last_prompt = Some(prompt.clone());
                    Some(prompt)
                }
            }
            TimerPhase::InBreak { next_round, .. } => {
                if self.remaining > delta {
                    self.remaining -= delta;
                    if !self.ten_sec_warned
                        && self.remaining <= Duration::from_secs(10)
                        && self.break_duration > Duration::from_secs(10)
                    {
                        self.ten_sec_warned = true;
                        let prompt = TimerPrompt::TenSecondWarning { phase: self.phase };
                        self.last_prompt = Some(prompt.clone());
                        return Some(prompt);
                    }
                    None
                } else {
                    let overflow = delta.saturating_sub(self.remaining);
                    self.phase = TimerPhase::InRound { round: next_round };
                    self.remaining = self.round_duration.saturating_sub(overflow);
                    self.ten_sec_warned = false;
                    let prompt = TimerPrompt::RoundStarted { round: next_round };
                    self.last_prompt = Some(prompt.clone());
                    Some(prompt)
                }
            }
        }
    }

    /// Manually skips to the next timer phase (e.g. from Round to Break, or Break to next Round).
    pub fn skip_phase(&mut self) -> Option<TimerPrompt> {
        if let TimerPhase::Ready { next_round } = self.phase {
            self.sync_to_round(next_round);
            return self.last_prompt.clone();
        }
        let remaining = self.remaining;
        let was_paused = self.paused;
        self.paused = false;
        let prompt = self.tick(remaining);
        self.paused = was_paused;
        prompt
    }

    /// Formats the remaining time as `MM:SS`.
    pub fn formatted_remaining(&self) -> String {
        let total_secs = self.remaining.as_secs();
        let mins = total_secs / 60;
        let secs = total_secs % 60;
        format!("{:02}:{:02}", mins, secs)
    }

    /// Returns a human-readable label for the current timer phase.
    pub fn phase_label(&self) -> String {
        match self.phase {
            TimerPhase::Ready { next_round } => format!("READY (R{})", next_round),
            TimerPhase::InRound { round } => format!("ROUND {}", round),
            TimerPhase::InBreak { next_round, .. } => {
                format!("BREAK (NEXT: R{})", next_round)
            }
            TimerPhase::Completed => "COMPLETE".to_string(),
        }
    }
}
