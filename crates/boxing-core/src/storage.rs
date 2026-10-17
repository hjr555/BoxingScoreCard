use crate::models::BoxingMatch;
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("I/O error accessing storage: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Match not found: {0}")]
    NotFound(Uuid),
}

/// Frontend- and backend-agnostic repository trait for persisting boxing matches.
pub trait MatchRepository: Send + Sync {
    fn list_matches(&self) -> Result<Vec<BoxingMatch>, StorageError>;
    fn get_match(&self, id: Uuid) -> Result<BoxingMatch, StorageError>;
    fn save_match(&self, boxing_match: &BoxingMatch) -> Result<(), StorageError>;
    fn delete_match(&self, id: Uuid) -> Result<(), StorageError>;
}

/// JSON file repository implementation for the MVP.
#[derive(Debug, Clone)]
pub struct JsonMatchRepository {
    file_path: PathBuf,
}

impl JsonMatchRepository {
    pub fn new(file_path: impl Into<PathBuf>) -> Self {
        Self {
            file_path: file_path.into(),
        }
    }

    pub fn file_path(&self) -> &Path {
        &self.file_path
    }

    fn write_all(&self, matches: &[BoxingMatch]) -> Result<(), StorageError> {
        if let Some(parent) = self.file_path.parent() {
            if !parent.as_os_str().is_empty() && !parent.exists() {
                fs::create_dir_all(parent)?;
            }
        }
        let json = serde_json::to_string_pretty(matches)?;
        fs::write(&self.file_path, json)?;
        Ok(())
    }
}

impl MatchRepository for JsonMatchRepository {
    fn list_matches(&self) -> Result<Vec<BoxingMatch>, StorageError> {
        if !self.file_path.exists() {
            return Ok(Vec::new());
        }
        let content = fs::read_to_string(&self.file_path)?;
        if content.trim().is_empty() {
            return Ok(Vec::new());
        }
        let matches: Vec<BoxingMatch> = serde_json::from_str(&content)?;
        Ok(matches)
    }

    fn get_match(&self, id: Uuid) -> Result<BoxingMatch, StorageError> {
        let matches = self.list_matches()?;
        matches
            .into_iter()
            .find(|m| m.id == id)
            .ok_or(StorageError::NotFound(id))
    }

    fn save_match(&self, boxing_match: &BoxingMatch) -> Result<(), StorageError> {
        let mut matches = self.list_matches()?;
        if let Some(existing) = matches.iter_mut().find(|m| m.id == boxing_match.id) {
            *existing = boxing_match.clone();
        } else {
            matches.push(boxing_match.clone());
        }
        self.write_all(&matches)
    }

    fn delete_match(&self, id: Uuid) -> Result<(), StorageError> {
        let mut matches = self.list_matches()?;
        let initial_len = matches.len();
        matches.retain(|m| m.id != id);
        if matches.len() == initial_len {
            return Err(StorageError::NotFound(id));
        }
        self.write_all(&matches)
    }
}
