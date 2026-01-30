use sqlx::{sqlite::SqlitePoolOptions, Pool, Sqlite};
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;
use anyhow::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Meeting {
    pub id: String,
    pub title: String,
    pub created_at: DateTime<Utc>,
    pub audio_path: Option<String>,
    pub duration_secs: Option<i64>,
    pub transcript: Option<String>,
    pub summary: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Note {
    pub id: String,
    pub meeting_id: String,
    pub content: String,
    pub timestamps: Vec<f64>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Template {
    pub id: String,
    pub name: String,
    pub structure: serde_json::Value,
}

pub struct Database {
    pool: Pool<Sqlite>,
}

impl Database {
    pub async fn new(db_path: &str) -> Result<Self> {
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect(db_path)
            .await?;

        let db = Self { pool };
        db.init().await?;
        
        Ok(db)
    }

    async fn init(&self) -> Result<()> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS meetings (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                audio_path TEXT,
                duration_secs INTEGER,
                transcript TEXT,
                summary TEXT
            )
            "#
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS notes (
                id TEXT PRIMARY KEY,
                meeting_id TEXT NOT NULL,
                content TEXT NOT NULL DEFAULT '',
                timestamps TEXT DEFAULT '[]',
                updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (meeting_id) REFERENCES meetings(id) ON DELETE CASCADE
            )
            "#
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS templates (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL UNIQUE,
                structure TEXT NOT NULL
            )
            "#
        )
        .execute(&self.pool)
        .await?;

        // Insert default templates
        self.insert_default_templates().await?;

        Ok(())
    }

    async fn insert_default_templates(&self) -> Result<()> {
        let default_templates = vec![
            ("1:1 Meeting", serde_json::json!({
                "sections": ["Updates", "Blockers", "Action Items"]
            })),
            ("Team Standup", serde_json::json!({
                "sections": ["Yesterday", "Today", "Blockers"]
            })),
            ("Sales Call", serde_json::json!({
                "sections": ["Attendees", "Pain Points", "Budget", "Timeline", "Next Steps"]
            })),
            ("Interview", serde_json::json!({
                "sections": ["Candidate", "Experience", "Technical", "Culture Fit", "Decision"]
            })),
        ];

        for (name, structure) in default_templates {
            let id = Uuid::new_v4().to_string();
            let structure_str = structure.to_string();
            
            let _ = sqlx::query(
                "INSERT OR IGNORE INTO templates (id, name, structure) VALUES (?, ?, ?)"
            )
            .bind(&id)
            .bind(name)
            .bind(&structure_str)
            .execute(&self.pool)
            .await;
        }

        Ok(())
    }

    // Meeting operations
    pub async fn create_meeting(&self, title: &str) -> Result<Meeting> {
        let id = Uuid::new_v4().to_string();
        let created_at = Utc::now();

        sqlx::query(
            "INSERT INTO meetings (id, title, created_at) VALUES (?, ?, ?)"
        )
        .bind(&id)
        .bind(title)
        .bind(&created_at)
        .execute(&self.pool)
        .await?;

        // Create empty note for this meeting
        self.create_note(&id).await?;

        Ok(Meeting {
            id,
            title: title.to_string(),
            created_at,
            audio_path: None,
            duration_secs: None,
            transcript: None,
            summary: None,
        })
    }

    pub async fn get_meeting(&self, id: &str) -> Result<Option<Meeting>> {
        let row = sqlx::query_as::<_, MeetingRow>(
            "SELECT id, title, created_at, audio_path, duration_secs, transcript, summary FROM meetings WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| r.into()))
    }

    pub async fn list_meetings(&self) -> Result<Vec<Meeting>> {
        let rows = sqlx::query_as::<_, MeetingRow>(
            "SELECT id, title, created_at, audio_path, duration_secs, transcript, summary FROM meetings ORDER BY created_at DESC"
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    pub async fn update_meeting_audio(&self, id: &str, audio_path: &str, duration_secs: i64) -> Result<()> {
        sqlx::query(
            "UPDATE meetings SET audio_path = ?, duration_secs = ? WHERE id = ?"
        )
        .bind(audio_path)
        .bind(duration_secs)
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn update_meeting_transcript(&self, id: &str, transcript: &str) -> Result<()> {
        sqlx::query(
            "UPDATE meetings SET transcript = ? WHERE id = ?"
        )
        .bind(transcript)
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn update_meeting_summary(&self, id: &str, summary: &str) -> Result<()> {
        sqlx::query(
            "UPDATE meetings SET summary = ? WHERE id = ?"
        )
        .bind(summary)
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn delete_meeting(&self, id: &str) -> Result<()> {
        sqlx::query("DELETE FROM meetings WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    // Note operations
    pub async fn create_note(&self, meeting_id: &str) -> Result<Note> {
        let id = Uuid::new_v4().to_string();
        let updated_at = Utc::now();

        sqlx::query(
            "INSERT INTO notes (id, meeting_id, content, timestamps, updated_at) VALUES (?, ?, '', '[]', ?)"
        )
        .bind(&id)
        .bind(meeting_id)
        .bind(&updated_at)
        .execute(&self.pool)
        .await?;

        Ok(Note {
            id,
            meeting_id: meeting_id.to_string(),
            content: String::new(),
            timestamps: Vec::new(),
            updated_at,
        })
    }

    pub async fn get_note(&self, meeting_id: &str) -> Result<Option<Note>> {
        let row = sqlx::query_as::<_, NoteRow>(
            "SELECT id, meeting_id, content, timestamps, updated_at FROM notes WHERE meeting_id = ?"
        )
        .bind(meeting_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| r.into()))
    }

    pub async fn update_note(&self, meeting_id: &str, content: &str, timestamps: Vec<f64>) -> Result<()> {
        let updated_at = Utc::now();
        let timestamps_json = serde_json::to_string(&timestamps)?;

        sqlx::query(
            "UPDATE notes SET content = ?, timestamps = ?, updated_at = ? WHERE meeting_id = ?"
        )
        .bind(content)
        .bind(&timestamps_json)
        .bind(&updated_at)
        .bind(meeting_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    // Template operations
    pub async fn list_templates(&self) -> Result<Vec<Template>> {
        let rows = sqlx::query_as::<_, TemplateRow>(
            "SELECT id, name, structure FROM templates ORDER BY name"
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    pub async fn get_template(&self, id: &str) -> Result<Option<Template>> {
        let row = sqlx::query_as::<_, TemplateRow>(
            "SELECT id, name, structure FROM templates WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| r.into()))
    }
}

// Database row types for sqlx
#[derive(sqlx::FromRow)]
struct MeetingRow {
    id: String,
    title: String,
    created_at: DateTime<Utc>,
    audio_path: Option<String>,
    duration_secs: Option<i64>,
    transcript: Option<String>,
    summary: Option<String>,
}

impl From<MeetingRow> for Meeting {
    fn from(row: MeetingRow) -> Self {
        Self {
            id: row.id,
            title: row.title,
            created_at: row.created_at,
            audio_path: row.audio_path,
            duration_secs: row.duration_secs,
            transcript: row.transcript,
            summary: row.summary,
        }
    }
}

#[derive(sqlx::FromRow)]
struct NoteRow {
    id: String,
    meeting_id: String,
    content: String,
    timestamps: String,
    updated_at: DateTime<Utc>,
}

impl From<NoteRow> for Note {
    fn from(row: NoteRow) -> Self {
        let timestamps: Vec<f64> = serde_json::from_str(&row.timestamps).unwrap_or_default();
        Self {
            id: row.id,
            meeting_id: row.meeting_id,
            content: row.content,
            timestamps,
            updated_at: row.updated_at,
        }
    }
}

#[derive(sqlx::FromRow)]
struct TemplateRow {
    id: String,
    name: String,
    structure: String,
}

impl From<TemplateRow> for Template {
    fn from(row: TemplateRow) -> Self {
        let structure: serde_json::Value = serde_json::from_str(&row.structure).unwrap_or_default();
        Self {
            id: row.id,
            name: row.name,
            structure,
        }
    }
}
