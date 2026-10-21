pub mod models;
pub mod service;
pub mod storage;
pub mod timer;

pub use models::{
    fighter_last_name, ActiveRound, BoxingMatch, FighterSide, FinishMethod, MatchOutcome,
    RoundScore, DEFAULT_BREAK_DURATION_SECS, DEFAULT_INITIAL_POINTS, DEFAULT_ROUND_DURATION_SECS,
    DEFAULT_SCHEDULED_ROUNDS, WALKOUT_ROUND_NUMBER,
};
pub use service::ScoringService;
pub use storage::{JsonMatchRepository, MatchRepository, StorageError};
pub use timer::{MatchTimer, TimerPhase, TimerPrompt};

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use std::time::Duration;

    #[test]
    fn test_title_format() {
        let date = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
        let m = BoxingMatch::new("Joshua", "Fury", date);
        assert_eq!(m.title(), "Joshua v Fury (01 Jan 2026)");
    }

    #[test]
    fn test_walkout_and_round_scoring() {
        let date = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
        let mut m = BoxingMatch::with_config("Joshua", "Fury", date, 3, 180, 60);

        // Starts at Pre-Fight Walkout (round_number 0), both at 10
        let active = m.active_round.as_ref().unwrap();
        assert!(active.is_walkout());
        assert_eq!(active.round_number, 0);
        assert_eq!(active.fighter_a_score, 10);
        assert_eq!(active.fighter_b_score, 10);

        // Score Walkout: Joshua 9 - 10 Fury
        assert!(m.deduct_point(FighterSide::FighterA));
        let locked_walkout = m.lock_round().unwrap();
        assert!(locked_walkout.is_walkout());
        assert_eq!(locked_walkout.fighter_a_score, 9);
        assert_eq!(locked_walkout.fighter_b_score, 10);
        assert!(m.walkout_score.is_some());

        // Official total excludes the fun walkout score
        assert_eq!(m.total_scores(), (0, 0));

        // Now in Round 1, both start at 10
        let active_r1 = m.active_round.as_ref().unwrap();
        assert_eq!(active_r1.round_number, 1);
        assert_eq!(active_r1.fighter_a_score, 10);
        assert_eq!(active_r1.fighter_b_score, 10);

        // Deduct point from Fury (10 -> 9 -> 8) and restore one (8 -> 9)
        assert!(m.deduct_point(FighterSide::FighterB));
        assert!(m.deduct_point(FighterSide::FighterB));
        assert!(m.restore_point(FighterSide::FighterB));
        assert_eq!(m.active_round.as_ref().unwrap().fighter_b_score, 9);

        // Lock Round 1 in -> official total is now 10 - 9
        let locked_r1 = m.lock_round().unwrap();
        assert_eq!(locked_r1.round_number, 1);
        assert_eq!(m.total_scores(), (10, 9));

        // Undo Round 1 -> back to Round 1, then undo again -> back to Walkout
        assert!(m.undo_last_round());
        assert_eq!(m.active_round.as_ref().unwrap().round_number, 1);
        assert!(m.undo_last_round());
        assert!(m.active_round.as_ref().unwrap().is_walkout());
        assert!(m.walkout_score.is_none());
    }

    #[test]
    fn test_timer_transitions_and_prompts() {
        let mut timer = MatchTimer::new(2, 180, 60);
        assert_eq!(timer.phase, TimerPhase::InRound { round: 1 });

        // Tick 170s -> 10s left warning
        let prompt = timer.tick(Duration::from_secs(170));
        assert_eq!(
            prompt,
            Some(TimerPrompt::TenSecondWarning {
                phase: TimerPhase::InRound { round: 1 }
            })
        );

        // Tick remaining 10s -> transitions to Break
        let prompt = timer.tick(Duration::from_secs(10));
        assert_eq!(
            prompt,
            Some(TimerPrompt::BreakStarted {
                after_round: 1,
                next_round: 2,
                break_duration_secs: 60,
            })
        );
        assert_eq!(
            timer.phase,
            TimerPhase::InBreak {
                after_round: 1,
                next_round: 2
            }
        );

        // Complete break -> transitions to Round 2
        let prompt = timer.tick(Duration::from_secs(60));
        assert_eq!(prompt, Some(TimerPrompt::RoundStarted { round: 2 }));
        assert_eq!(timer.phase, TimerPhase::InRound { round: 2 });

        // Complete Round 2 -> FinalBell
        let prompt = timer.tick(Duration::from_secs(180));
        assert_eq!(prompt, Some(TimerPrompt::FinalBell { final_round: 2 }));
        assert_eq!(timer.phase, TimerPhase::Completed);
    }

    #[test]
    fn test_post_hoc_boxrec_ids() {
        let date = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
        let mut m = BoxingMatch::new("Joshua", "Fury", date);
        assert_eq!(m.boxrec_id(FighterSide::FighterA), None);
        assert_eq!(m.boxrec_id(FighterSide::FighterB), None);

        // Set BoxRec IDs after match creation
        m.set_boxrec_id(FighterSide::FighterA, " 659461 ");
        m.set_boxrec_id(FighterSide::FighterB, "479205");
        assert_eq!(m.boxrec_id(FighterSide::FighterA), Some("659461"));
        assert_eq!(m.boxrec_id(FighterSide::FighterB), Some("479205"));

        // Clear a BoxRec ID with empty string
        m.set_boxrec_id(FighterSide::FighterA, "   ");
        assert_eq!(m.boxrec_id(FighterSide::FighterA), None);
    }

    #[test]
    fn test_stoppage_and_completion() {
        let date = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
        let mut m = BoxingMatch::new("Dubois", "Wardley", date);

        // Lock walkout & Round 1
        m.lock_round();
        m.lock_round();
        assert!(!m.is_complete());

        // Deduct a knockdown point in Round 2 before stoppage
        m.deduct_point(FighterSide::FighterB);
        m.deduct_point(FighterSide::FighterB);

        // Corner throws in the towel in Round 2 (RTD win for Fighter A)
        m.finish_match(MatchOutcome {
            method: FinishMethod::Rtd,
            winner: Some(FighterSide::FighterA),
            round: 2,
        });

        assert!(m.is_complete());
        assert!(m.active_round.is_none());
        assert_eq!(m.outcome_summary().as_deref(), Some("Dubois by RTD (R2)"));

        // Editing walkout after a stoppage does not erase the stoppage outcome
        m.open_walkout();
        m.deduct_point(FighterSide::FighterB);
        m.lock_round();
        assert!(m.is_complete());
        assert_eq!(m.outcome_summary().as_deref(), Some("Dubois by RTD (R2)"));
        assert_eq!(m.walkout_score.as_ref().unwrap().fighter_b_score, 9);

        // Undo reopens Round 2 with its partial 10-8 score if stopped by mistake
        assert!(m.undo_last_round());
        assert!(!m.is_complete());
        let active = m.active_round.as_ref().unwrap();
        assert_eq!(active.round_number, 2);
        assert_eq!(active.fighter_a_score, 10);
        assert_eq!(active.fighter_b_score, 8);
    }

    #[test]
    fn test_mid_match_walkout_preserves_active_round() {
        let date = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
        let mut m = BoxingMatch::new("Usyk", "Fury", date);

        // Lock Walkout & Round 1
        m.lock_round();
        m.lock_round();

        // In Round 2, deduct a point from Fury (10 - 9)
        m.deduct_point(FighterSide::FighterB);
        assert_eq!(m.active_round.as_ref().unwrap().round_number, 2);
        assert_eq!(m.active_round.as_ref().unwrap().fighter_b_score, 9);

        // Jump to Walkout mid-Round 2, score it 10 - 8, and lock Walkout
        m.open_walkout();
        assert!(m.active_round.as_ref().unwrap().is_walkout());
        m.deduct_point(FighterSide::FighterB);
        m.deduct_point(FighterSide::FighterB);
        m.lock_round();

        // Should resume Round 2 with its in-progress 10 - 9 score intact!
        let active = m.active_round.as_ref().unwrap();
        assert_eq!(active.round_number, 2);
        assert_eq!(active.fighter_a_score, 10);
        assert_eq!(active.fighter_b_score, 9);
        assert_eq!(m.walkout_score.as_ref().unwrap().fighter_b_score, 8);
    }

    #[test]
    fn test_v0_1_0_matches_json_backwards_compatibility() {
        // Snapshot of the v0.1.0 matches.json schema (both in-progress and finished matches)
        // Future versions must always be able to deserialize this format without data loss.
        let v0_1_0_json = r#"[
  {
    "id": "60631245-35ba-4a92-ba3f-7c49253bd9a0",
    "fighter_a": "Daniel Dubois",
    "fighter_b": "Fabio Wardley",
    "date": "2026-10-17",
    "scheduled_rounds": 12,
    "round_duration_secs": 180,
    "break_duration_secs": 60,
    "walkout_score": {
      "round_number": 0,
      "fighter_a_score": 9,
      "fighter_b_score": 10
    },
    "locked_rounds": [
      {
        "round_number": 1,
        "fighter_a_score": 10,
        "fighter_b_score": 9
      }
    ],
    "active_round": {
      "round_number": 2,
      "fighter_a_score": 10,
      "fighter_b_score": 9
    }
  },
  {
    "id": "11111111-2222-3333-4444-555555555555",
    "fighter_a": "Oleksandr Usyk",
    "fighter_b": "Tyson Fury",
    "fighter_a_boxrec_id": "659772",
    "fighter_b_boxrec_id": "479205",
    "date": "2024-05-18",
    "scheduled_rounds": 12,
    "round_duration_secs": 180,
    "break_duration_secs": 60,
    "walkout_score": null,
    "locked_rounds": [
      {
        "round_number": 1,
        "fighter_a_score": 10,
        "fighter_b_score": 9
      }
    ],
    "active_round": null,
    "outcome": {
      "method": "decision",
      "winner": "fighter_a",
      "round": 12
    }
  }
]"#;

        let matches: Vec<BoxingMatch> =
            serde_json::from_str(v0_1_0_json).expect("v0.1.0 matches.json must deserialize");
        assert_eq!(matches.len(), 2);

        let m1 = &matches[0];
        assert_eq!(m1.fighter_a, "Daniel Dubois");
        assert_eq!(m1.fighter_b, "Fabio Wardley");
        assert_eq!(m1.total_scores(), (10, 9));
        assert_eq!(m1.active_round.as_ref().unwrap().round_number, 2);
        assert!(!m1.is_complete());

        let m2 = &matches[1];
        assert_eq!(m2.boxrec_id(FighterSide::FighterA), Some("659772"));
        assert!(m2.is_complete());
        assert_eq!(
            m2.outcome_summary().as_deref(),
            Some("Usyk (DEC)")
        );
    }

    #[test]
    fn test_matches_test_json_integrity() {
        let content = std::fs::read_to_string("../../matches.test.json")
            .expect("matches.test.json must exist");
        let matches: Vec<BoxingMatch> =
            serde_json::from_str(&content).expect("matches.test.json must deserialize cleanly");
        assert_eq!(matches.len(), 100);

        let mut fighter_to_id = std::collections::HashMap::new();
        let mut id_to_fighter = std::collections::HashMap::new();
        for m in &matches {
            let id_a = m.fighter_a_boxrec_id.as_deref().expect("Fighter A BoxRec ID");
            let id_b = m.fighter_b_boxrec_id.as_deref().expect("Fighter B BoxRec ID");
            assert!(id_a.chars().all(|c| c.is_ascii_digit()));
            assert!(id_b.chars().all(|c| c.is_ascii_digit()));

            if let Some(prev) = fighter_to_id.insert(m.fighter_a.clone(), id_a.to_string()) {
                assert_eq!(prev, id_a, "Inconsistent BoxRec ID for {}", m.fighter_a);
            }
            if let Some(prev) = fighter_to_id.insert(m.fighter_b.clone(), id_b.to_string()) {
                assert_eq!(prev, id_b, "Inconsistent BoxRec ID for {}", m.fighter_b);
            }
            if let Some(prev_f) = id_to_fighter.insert(id_a.to_string(), m.fighter_a.clone()) {
                assert_eq!(prev_f, m.fighter_a, "Duplicate BoxRec ID {} shared by {} and {}", id_a, prev_f, m.fighter_a);
            }
            if let Some(prev_f) = id_to_fighter.insert(id_b.to_string(), m.fighter_b.clone()) {
                assert_eq!(prev_f, m.fighter_b, "Duplicate BoxRec ID {} shared by {} and {}", id_b, prev_f, m.fighter_b);
            }
        }
        assert_eq!(fighter_to_id.get("Francis Ngannou").map(String::as_str), Some("1170712"));
        assert_eq!(fighter_to_id.get("Anthony Joshua").map(String::as_str), Some("659461"));
        assert_eq!(fighter_to_id.get("Oleksandr Usyk").map(String::as_str), Some("659772"));
        assert_eq!(fighter_to_id.get("Tyson Fury").map(String::as_str), Some("479205"));
    }
}
