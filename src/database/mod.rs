use anyhow::Result;
use sqlx::Row;
use tracing::info;

#[derive(Debug, Clone)]
pub struct Database {
    inner: DatabaseInner,
}

#[derive(Debug, Clone)]
enum DatabaseInner {
    Sqlite(sqlx::SqlitePool),
    Postgres(sqlx::PgPool),
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Portal {
    pub chat_type: String,
    pub chat_id: String,
    pub room_id: String,
    pub name: String,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct MessageMap {
    pub source: String,
    pub source_msg_id: String,
    pub room_id: String,
    pub chat_type: String,
    pub chat_id: String,
    pub matrix_event_id: Option<String>,
    pub qq_message_id: Option<String>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct QQUser {
    pub qq_user_id: String,
    pub mxid: String,
    pub displayname: String,
    pub avatar_url: Option<String>,
}

impl Database {
    pub async fn connect(db_type: &str, uri: &str, max_open: u32, max_idle: u32) -> Result<Self> {
        match db_type {
            "sqlite" | "sqlite3" => {
                info!("connecting to sqlite");
                let pool = sqlx::sqlite::SqlitePoolOptions::new()
                    .max_connections(max_open)
                    .min_connections(max_idle)
                    .connect(uri)
                    .await?;
                Ok(Self {
                    inner: DatabaseInner::Sqlite(pool),
                })
            }
            "postgres" => {
                info!("connecting to postgres");
                let pool = sqlx::postgres::PgPoolOptions::new()
                    .max_connections(max_open)
                    .min_connections(max_idle)
                    .connect(uri)
                    .await?;
                Ok(Self {
                    inner: DatabaseInner::Postgres(pool),
                })
            }
            _ => anyhow::bail!("unsupported database type: {db_type}"),
        }
    }

    pub async fn run_migrations(&self) -> Result<()> {
        let migration_sql = include_str!("../../migrations/001_initial.sql");
        match &self.inner {
            DatabaseInner::Sqlite(pool) => {
                for stmt in migration_sql.split(';') {
                    let trimmed = stmt.trim();
                    if !trimmed.is_empty() {
                        sqlx::query(trimmed).execute(pool).await?;
                    }
                }
            }
            DatabaseInner::Postgres(pool) => {
                for stmt in migration_sql.split(';') {
                    let trimmed = stmt.trim();
                    if !trimmed.is_empty() {
                        sqlx::query(trimmed).execute(pool).await?;
                    }
                }
            }
        }
        Ok(())
    }

    pub async fn get_portal_by_chat(
        &self,
        chat_type: &str,
        chat_id: &str,
    ) -> Result<Option<Portal>> {
        match &self.inner {
            DatabaseInner::Sqlite(pool) => {
                let row = sqlx::query_as::<_, Portal>(
                    "SELECT chat_type, chat_id, room_id, name FROM portal WHERE chat_type = ? AND chat_id = ?",
                )
                .bind(chat_type)
                .bind(chat_id)
                .fetch_optional(pool)
                .await?;
                Ok(row)
            }
            DatabaseInner::Postgres(pool) => {
                let row = sqlx::query_as::<_, Portal>(
                    "SELECT chat_type, chat_id, room_id, name FROM portal WHERE chat_type = $1 AND chat_id = $2",
                )
                .bind(chat_type)
                .bind(chat_id)
                .fetch_optional(pool)
                .await?;
                Ok(row)
            }
        }
    }

    pub async fn get_portal_by_room(&self, room_id: &str) -> Result<Option<Portal>> {
        match &self.inner {
            DatabaseInner::Sqlite(pool) => {
                let row = sqlx::query_as::<_, Portal>(
                    "SELECT chat_type, chat_id, room_id, name FROM portal WHERE room_id = ?",
                )
                .bind(room_id)
                .fetch_optional(pool)
                .await?;
                Ok(row)
            }
            DatabaseInner::Postgres(pool) => {
                let row = sqlx::query_as::<_, Portal>(
                    "SELECT chat_type, chat_id, room_id, name FROM portal WHERE room_id = $1",
                )
                .bind(room_id)
                .fetch_optional(pool)
                .await?;
                Ok(row)
            }
        }
    }

    pub async fn upsert_portal(&self, portal: &Portal) -> Result<()> {
        let now = chrono::Utc::now().timestamp_millis();
        match &self.inner {
            DatabaseInner::Sqlite(pool) => {
                sqlx::query(
                    "INSERT INTO portal (chat_type, chat_id, room_id, name, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?)\
                     ON CONFLICT(chat_type, chat_id) DO UPDATE SET room_id = excluded.room_id, name = excluded.name, updated_at = excluded.updated_at",
                )
                .bind(&portal.chat_type)
                .bind(&portal.chat_id)
                .bind(&portal.room_id)
                .bind(&portal.name)
                .bind(now)
                .bind(now)
                .execute(pool)
                .await?;
            }
            DatabaseInner::Postgres(pool) => {
                sqlx::query(
                    "INSERT INTO portal (chat_type, chat_id, room_id, name, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6)\
                     ON CONFLICT(chat_type, chat_id) DO UPDATE SET room_id = EXCLUDED.room_id, name = EXCLUDED.name, updated_at = EXCLUDED.updated_at",
                )
                .bind(&portal.chat_type)
                .bind(&portal.chat_id)
                .bind(&portal.room_id)
                .bind(&portal.name)
                .bind(now)
                .bind(now)
                .execute(pool)
                .await?;
            }
        }
        Ok(())
    }

    pub async fn insert_message_if_absent(
        &self,
        source: &str,
        source_msg_id: &str,
        room_id: &str,
        chat_type: &str,
        chat_id: &str,
    ) -> Result<bool> {
        let now = chrono::Utc::now().timestamp_millis();
        let rows = match &self.inner {
            DatabaseInner::Sqlite(pool) => {
                sqlx::query(
                    "INSERT INTO message_map (source, source_msg_id, room_id, chat_type, chat_id, created_at) VALUES (?, ?, ?, ?, ?, ?)\
                     ON CONFLICT(source, source_msg_id) DO NOTHING",
                )
                .bind(source)
                .bind(source_msg_id)
                .bind(room_id)
                .bind(chat_type)
                .bind(chat_id)
                .bind(now)
                .execute(pool)
                .await?
                .rows_affected()
            }
            DatabaseInner::Postgres(pool) => {
                sqlx::query(
                    "INSERT INTO message_map (source, source_msg_id, room_id, chat_type, chat_id, created_at) VALUES ($1, $2, $3, $4, $5, $6)\
                     ON CONFLICT(source, source_msg_id) DO NOTHING",
                )
                .bind(source)
                .bind(source_msg_id)
                .bind(room_id)
                .bind(chat_type)
                .bind(chat_id)
                .bind(now)
                .execute(pool)
                .await?
                .rows_affected()
            }
        };

        Ok(rows > 0)
    }

    pub async fn update_matrix_event_id(
        &self,
        source: &str,
        source_msg_id: &str,
        matrix_event_id: &str,
    ) -> Result<()> {
        match &self.inner {
            DatabaseInner::Sqlite(pool) => {
                sqlx::query("UPDATE message_map SET matrix_event_id = ? WHERE source = ? AND source_msg_id = ?")
                    .bind(matrix_event_id)
                    .bind(source)
                    .bind(source_msg_id)
                    .execute(pool)
                    .await?;
            }
            DatabaseInner::Postgres(pool) => {
                sqlx::query("UPDATE message_map SET matrix_event_id = $1 WHERE source = $2 AND source_msg_id = $3")
                    .bind(matrix_event_id)
                    .bind(source)
                    .bind(source_msg_id)
                    .execute(pool)
                    .await?;
            }
        }
        Ok(())
    }

    pub async fn update_qq_message_id(
        &self,
        source: &str,
        source_msg_id: &str,
        qq_message_id: &str,
    ) -> Result<()> {
        match &self.inner {
            DatabaseInner::Sqlite(pool) => {
                sqlx::query("UPDATE message_map SET qq_message_id = ? WHERE source = ? AND source_msg_id = ?")
                    .bind(qq_message_id)
                    .bind(source)
                    .bind(source_msg_id)
                    .execute(pool)
                    .await?;
            }
            DatabaseInner::Postgres(pool) => {
                sqlx::query("UPDATE message_map SET qq_message_id = $1 WHERE source = $2 AND source_msg_id = $3")
                    .bind(qq_message_id)
                    .bind(source)
                    .bind(source_msg_id)
                    .execute(pool)
                    .await?;
            }
        }
        Ok(())
    }

    pub async fn mark_transaction_processed(&self, txn_id: &str) -> Result<bool> {
        let now = chrono::Utc::now().timestamp_millis();
        let rows = match &self.inner {
            DatabaseInner::Sqlite(pool) => sqlx::query(
                "INSERT INTO processed_txn (txn_id, processed_at) VALUES (?, ?)\
                     ON CONFLICT(txn_id) DO NOTHING",
            )
            .bind(txn_id)
            .bind(now)
            .execute(pool)
            .await?
            .rows_affected(),
            DatabaseInner::Postgres(pool) => sqlx::query(
                "INSERT INTO processed_txn (txn_id, processed_at) VALUES ($1, $2)\
                     ON CONFLICT(txn_id) DO NOTHING",
            )
            .bind(txn_id)
            .bind(now)
            .execute(pool)
            .await?
            .rows_affected(),
        };
        Ok(rows > 0)
    }

    pub async fn upsert_qq_user(&self, user: &QQUser) -> Result<()> {
        let now = chrono::Utc::now().timestamp_millis();
        match &self.inner {
            DatabaseInner::Sqlite(pool) => {
                sqlx::query(
                    "INSERT INTO qq_user (qq_user_id, mxid, displayname, avatar_url, updated_at) VALUES (?, ?, ?, ?, ?)\
                     ON CONFLICT(qq_user_id) DO UPDATE SET mxid = excluded.mxid, displayname = excluded.displayname, avatar_url = excluded.avatar_url, updated_at = excluded.updated_at",
                )
                .bind(&user.qq_user_id)
                .bind(&user.mxid)
                .bind(&user.displayname)
                .bind(&user.avatar_url)
                .bind(now)
                .execute(pool)
                .await?;
            }
            DatabaseInner::Postgres(pool) => {
                sqlx::query(
                    "INSERT INTO qq_user (qq_user_id, mxid, displayname, avatar_url, updated_at) VALUES ($1, $2, $3, $4, $5)\
                     ON CONFLICT(qq_user_id) DO UPDATE SET mxid = EXCLUDED.mxid, displayname = EXCLUDED.displayname, avatar_url = EXCLUDED.avatar_url, updated_at = EXCLUDED.updated_at",
                )
                .bind(&user.qq_user_id)
                .bind(&user.mxid)
                .bind(&user.displayname)
                .bind(&user.avatar_url)
                .bind(now)
                .execute(pool)
                .await?;
            }
        }
        Ok(())
    }

    pub async fn get_qq_user(&self, qq_user_id: &str) -> Result<Option<QQUser>> {
        match &self.inner {
            DatabaseInner::Sqlite(pool) => {
                let user = sqlx::query_as::<_, QQUser>(
                    "SELECT qq_user_id, mxid, displayname, avatar_url FROM qq_user WHERE qq_user_id = ?",
                )
                .bind(qq_user_id)
                .fetch_optional(pool)
                .await?;
                Ok(user)
            }
            DatabaseInner::Postgres(pool) => {
                let user = sqlx::query_as::<_, QQUser>(
                    "SELECT qq_user_id, mxid, displayname, avatar_url FROM qq_user WHERE qq_user_id = $1",
                )
                .bind(qq_user_id)
                .fetch_optional(pool)
                .await?;
                Ok(user)
            }
        }
    }

    pub async fn count_portals(&self) -> Result<i64> {
        let value = match &self.inner {
            DatabaseInner::Sqlite(pool) => {
                let row = sqlx::query("SELECT COUNT(*) AS cnt FROM portal")
                    .fetch_one(pool)
                    .await?;
                row.try_get::<i64, _>("cnt")?
            }
            DatabaseInner::Postgres(pool) => {
                let row = sqlx::query("SELECT COUNT(*) AS cnt FROM portal")
                    .fetch_one(pool)
                    .await?;
                row.try_get::<i64, _>("cnt")?
            }
        };
        Ok(value)
    }
}
