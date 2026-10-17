use crate::models::{BoxingMatch, FighterSide, RoundScore};
use crate::storage::{MatchRepository, StorageError};
use chrono::NaiveDate;
use uuid::Uuid;

/// High-level application service exposing boxing match scoring operations to any frontend
/// (CLI, TUI, MCP server, Web API, Mobile app).
#[derive(Debug, Clone)]
pub struct ScoringService<R: MatchRepository> {
    repository: R,
}

impl<R: MatchRepository> ScoringService<R> {
    pub fn new(repository: R) -> Self {
        Self { repository }
    }

    pub fn repository(&self) -> &R {
        &self.repository
    }

    pub fn list_matches(&self) -> Result<Vec<BoxingMatch>, StorageError> {
        self.repository.list_matches()
    }

    pub fn get_match(&self, id: Uuid) -> Result<BoxingMatch, StorageError> {
        self.repository.get_match(id)
    }

    pub fn create_match(
        &self,
        fighter_a: impl Into<String>,
        fighter_b: impl Into<String>,
        date: NaiveDate,
        scheduled_rounds: u8,
    ) -> Result<BoxingMatch, StorageError> {
        let mut boxing_match = BoxingMatch::new(fighter_a, fighter_b, date);
        boxing_match.scheduled_rounds = scheduled_rounds.max(1);
        self.repository.save_match(&boxing_match)?;
        Ok(boxing_match)
    }

    pub fn save_match(&self, boxing_match: &BoxingMatch) -> Result<(), StorageError> {
        self.repository.save_match(boxing_match)
    }

    pub fn delete_match(&self, id: Uuid) -> Result<(), StorageError> {
        self.repository.delete_match(id)
    }

    /// Deducts 1 point from a fighter in the active round and persists the match.
    pub fn deduct_point(
        &self,
        match_id: Uuid,
        side: FighterSide,
    ) -> Result<BoxingMatch, StorageError> {
        let mut boxing_match = self.repository.get_match(match_id)?;
        boxing_match.deduct_point(side);
        self.repository.save_match(&boxing_match)?;
        Ok(boxing_match)
    }

    /// Restores 1 point to a fighter in the active round (max 10) and persists the match.
    pub fn restore_point(
        &self,
        match_id: Uuid,
        side: FighterSide,
    ) -> Result<BoxingMatch, StorageError> {
        let mut boxing_match = self.repository.get_match(match_id)?;
        boxing_match.restore_point(side);
        self.repository.save_match(&boxing_match)?;
        Ok(boxing_match)
    }

    /// Locks the current round in and persists the match.
    pub fn lock_round(
        &self,
        match_id: Uuid,
    ) -> Result<(BoxingMatch, Option<RoundScore>), StorageError> {
        let mut boxing_match = self.repository.get_match(match_id)?;
        let locked = boxing_match.lock_round();
        self.repository.save_match(&boxing_match)?;
        Ok((boxing_match, locked))
    }

    /// Undoes the last locked round and persists the match.
    pub fn undo_last_round(&self, match_id: Uuid) -> Result<BoxingMatch, StorageError> {
        let mut boxing_match = self.repository.get_match(match_id)?;
        boxing_match.undo_last_round();
        self.repository.save_match(&boxing_match)?;
        Ok(boxing_match)
    }

    /// Sets or updates the BoxRec IDs for the fighters on a match after the fact.
    pub fn set_boxrec_ids(
        &self,
        match_id: Uuid,
        fighter_a_boxrec_id: impl Into<String>,
        fighter_b_boxrec_id: impl Into<String>,
    ) -> Result<BoxingMatch, StorageError> {
        let mut boxing_match = self.repository.get_match(match_id)?;
        boxing_match.set_boxrec_id(FighterSide::FighterA, fighter_a_boxrec_id);
        boxing_match.set_boxrec_id(FighterSide::FighterB, fighter_b_boxrec_id);
        self.repository.save_match(&boxing_match)?;
        Ok(boxing_match)
    }

    /// Records a match stoppage/completion (TKO, RTD/towel, KO, Decision, etc.) and marks the fight complete.
    pub fn finish_match(
        &self,
        match_id: Uuid,
        outcome: crate::models::MatchOutcome,
    ) -> Result<BoxingMatch, StorageError> {
        let mut boxing_match = self.repository.get_match(match_id)?;
        boxing_match.finish_match(outcome);
        self.repository.save_match(&boxing_match)?;
        Ok(boxing_match)
    }
}
