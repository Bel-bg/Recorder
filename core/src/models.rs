/// A saved speaking practice take.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Recording {
    pub id: String,
    /// Absolute path of the audio file inside the app private storage.
    pub path: String,
    pub duration_ms: i64,
    /// Creation time, milliseconds since the Unix epoch.
    pub created_at: i64,
}
