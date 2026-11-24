use std::path::Path;
use std::sync::Mutex;

use rusqlite::{params, Connection, OptionalExtension};

use crate::error::Result;
use crate::models::Recording;
use crate::DEFAULT_DAILY_GOAL_MINUTES;

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS recordings (
    id TEXT PRIMARY KEY NOT NULL,
    path TEXT NOT NULL,
    duration_ms INTEGER NOT NULL,
    created_at INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS recordings_created_at ON recordings (created_at DESC);
CREATE TABLE IF NOT EXISTS preferences (
    key TEXT PRIMARY KEY NOT NULL,
    value TEXT NOT NULL
);
";

const DAILY_GOAL_KEY: &str = "daily_goal_minutes";

/// SQLite backed metadata store. Audio files live outside the database.
pub struct Database {
    connection: Mutex<Connection>,
}

impl Database {
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        Self::from_connection(Connection::open(path)?)
    }

    #[cfg(test)]
    pub fn open_in_memory() -> Result<Self> {
        Self::from_connection(Connection::open_in_memory()?)
    }

    fn from_connection(connection: Connection) -> Result<Self> {
        connection.pragma_update(None, "journal_mode", "WAL")?;
        connection.execute_batch(SCHEMA)?;
        Ok(Self {
            connection: Mutex::new(connection),
        })
    }

    pub fn insert_recording(&self, recording: &Recording) -> Result<()> {
        let connection = self.lock();
        connection.execute(
            "INSERT INTO recordings (id, path, duration_ms, created_at) VALUES (?1, ?2, ?3, ?4)",
            params![
                recording.id,
                recording.path,
                recording.duration_ms,
                recording.created_at
            ],
        )?;
        Ok(())
    }

    /// All recordings, newest first.
    pub fn recordings(&self) -> Result<Vec<Recording>> {
        let connection = self.lock();
        let mut statement = connection.prepare(
            "SELECT id, path, duration_ms, created_at FROM recordings ORDER BY created_at DESC",
        )?;
        let rows = statement.query_map([], |row| {
            Ok(Recording {
                id: row.get(0)?,
                path: row.get(1)?,
                duration_ms: row.get(2)?,
                created_at: row.get(3)?,
            })
        })?;
        let mut recordings = Vec::new();
        for row in rows {
            recordings.push(row?);
        }
        Ok(recordings)
    }

    pub fn recording(&self, id: &str) -> Result<Option<Recording>> {
        let connection = self.lock();
        let recording = connection
            .query_row(
                "SELECT id, path, duration_ms, created_at FROM recordings WHERE id = ?1",
                params![id],
                |row| {
                    Ok(Recording {
                        id: row.get(0)?,
                        path: row.get(1)?,
                        duration_ms: row.get(2)?,
                        created_at: row.get(3)?,
                    })
                },
            )
            .optional()?;
        Ok(recording)
    }

    /// Removes the row and returns the audio path so the caller can delete it.
    pub fn delete_recording(&self, id: &str) -> Result<Option<String>> {
        let connection = self.lock();
        let path: Option<String> = connection
            .query_row(
                "SELECT path FROM recordings WHERE id = ?1",
                params![id],
                |row| row.get(0),
            )
            .optional()?;
        if path.is_some() {
            connection.execute("DELETE FROM recordings WHERE id = ?1", params![id])?;
        }
        Ok(path)
    }

    pub fn daily_goal_minutes(&self) -> u32 {
        let connection = self.lock();
        let stored: Option<String> = connection
            .query_row(
                "SELECT value FROM preferences WHERE key = ?1",
                params![DAILY_GOAL_KEY],
                |row| row.get(0),
            )
            .optional()
            .unwrap_or(None);
        stored
            .and_then(|value| value.parse().ok())
            .filter(|minutes: &u32| *minutes > 0)
            .unwrap_or(DEFAULT_DAILY_GOAL_MINUTES)
    }

    pub fn set_daily_goal_minutes(&self, minutes: u32) -> Result<()> {
        let connection = self.lock();
        connection.execute(
            "INSERT INTO preferences (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![DAILY_GOAL_KEY, minutes.to_string()],
        )?;
        Ok(())
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Connection> {
        self.connection
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn recording(id: &str, created_at: i64) -> Recording {
        Recording {
            id: id.to_string(),
            path: format!("/data/user/0/com.belmonde.recoder/files/recordings/{id}.wav"),
            duration_ms: 42_000,
            created_at,
        }
    }

    #[test]
    fn recordings_survive_and_are_ordered() {
        let database = Database::open_in_memory().expect("database");
        database.insert_recording(&recording("older", 1_000)).unwrap();
        database.insert_recording(&recording("newer", 2_000)).unwrap();

        let stored = database.recordings().unwrap();
        assert_eq!(stored.len(), 2);
        assert_eq!(stored[0].id, "newer");
        assert_eq!(stored[1].duration_ms, 42_000);
        assert_eq!(
            database.recording("older").unwrap().unwrap().path,
            "/data/user/0/com.belmonde.recoder/files/recordings/older.wav"
        );
    }

    #[test]
    fn deleting_returns_the_audio_path() {
        let database = Database::open_in_memory().expect("database");
        database.insert_recording(&recording("take", 10)).unwrap();

        let path = database.delete_recording("take").unwrap();
        assert!(path.is_some());
        assert!(database.recordings().unwrap().is_empty());
        assert!(database.delete_recording("take").unwrap().is_none());
    }

    #[test]
    fn daily_goal_is_persisted_with_default() {
        let database = Database::open_in_memory().expect("database");
        assert_eq!(database.daily_goal_minutes(), DEFAULT_DAILY_GOAL_MINUTES);

        database.set_daily_goal_minutes(25).unwrap();
        assert_eq!(database.daily_goal_minutes(), 25);

        database.set_daily_goal_minutes(15).unwrap();
        assert_eq!(database.daily_goal_minutes(), 15);
    }
}
