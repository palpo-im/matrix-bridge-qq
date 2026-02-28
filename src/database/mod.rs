use anyhow::{bail, Context, Result};
use diesel::connection::SimpleConnection;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use diesel::r2d2::{self, ConnectionManager};
use diesel::sql_types::{BigInt, Nullable, Text};
use tokio::task;
use tracing::info;

type SqlitePool = r2d2::Pool<ConnectionManager<SqliteConnection>>;
type PgPool = r2d2::Pool<ConnectionManager<PgConnection>>;

#[derive(Debug, Clone)]
pub struct Database {
    inner: DatabaseInner,
}

#[derive(Debug, Clone)]
enum DatabaseInner {
    Sqlite(SqlitePool),
    Postgres(PgPool),
}

#[derive(Debug, Clone)]
pub struct Portal {
    pub chat_type: String,
    pub chat_id: String,
    pub room_id: String,
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct MessageMap {
    pub source: String,
    pub source_msg_id: String,
    pub room_id: String,
    pub chat_type: String,
    pub chat_id: String,
    pub matrix_event_id: Option<String>,
    pub qq_message_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct QQUser {
    pub qq_user_id: String,
    pub mxid: String,
    pub displayname: String,
    pub avatar_url: Option<String>,
}

#[derive(Debug, QueryableByName)]
struct PortalRow {
    #[diesel(sql_type = Text)]
    chat_type: String,
    #[diesel(sql_type = Text)]
    chat_id: String,
    #[diesel(sql_type = Text)]
    room_id: String,
    #[diesel(sql_type = Text)]
    name: String,
}

impl From<PortalRow> for Portal {
    fn from(row: PortalRow) -> Self {
        Self {
            chat_type: row.chat_type,
            chat_id: row.chat_id,
            room_id: row.room_id,
            name: row.name,
        }
    }
}

#[derive(Debug, QueryableByName)]
struct QQUserRow {
    #[diesel(sql_type = Text)]
    qq_user_id: String,
    #[diesel(sql_type = Text)]
    mxid: String,
    #[diesel(sql_type = Text)]
    displayname: String,
    #[diesel(sql_type = Nullable<Text>)]
    avatar_url: Option<String>,
}

impl From<QQUserRow> for QQUser {
    fn from(row: QQUserRow) -> Self {
        Self {
            qq_user_id: row.qq_user_id,
            mxid: row.mxid,
            displayname: row.displayname,
            avatar_url: row.avatar_url,
        }
    }
}

#[derive(Debug, QueryableByName)]
struct CountRow {
    #[diesel(sql_type = BigInt)]
    cnt: i64,
}

impl Database {
    pub async fn connect(db_type: &str, uri: &str, max_open: u32, max_idle: u32) -> Result<Self> {
        match db_type {
            "sqlite" | "sqlite3" => {
                info!("connecting to sqlite via diesel");
                let sqlite_url = normalize_sqlite_uri(uri);
                let pool = task::spawn_blocking(move || -> Result<SqlitePool> {
                    let manager = ConnectionManager::<SqliteConnection>::new(sqlite_url);
                    let pool = r2d2::Pool::builder()
                        .max_size(max_open)
                        .min_idle(Some(max_idle))
                        .build(manager)
                        .context("failed to build sqlite pool")?;
                    Ok(pool)
                })
                .await??;
                Ok(Self {
                    inner: DatabaseInner::Sqlite(pool),
                })
            }
            "postgres" | "postgresql" | "pgsql" => {
                info!("connecting to postgres via diesel");
                let pg_url = uri.to_owned();
                let pool = task::spawn_blocking(move || -> Result<PgPool> {
                    let manager = ConnectionManager::<PgConnection>::new(pg_url);
                    let pool = r2d2::Pool::builder()
                        .max_size(max_open)
                        .min_idle(Some(max_idle))
                        .build(manager)
                        .context("failed to build postgres pool")?;
                    Ok(pool)
                })
                .await??;
                Ok(Self {
                    inner: DatabaseInner::Postgres(pool),
                })
            }
            _ => bail!(
                "unsupported database type: {db_type}, expected sqlite/sqlite3/postgres/postgresql/pgsql"
            ),
        }
    }

    pub async fn run_migrations(&self) -> Result<()> {
        let migration_sql: &'static str = include_str!("../../migrations/001_initial.sql");
        match &self.inner {
            DatabaseInner::Sqlite(pool) => {
                let pool = pool.clone();
                with_sqlite_conn(pool, move |conn| {
                    conn.batch_execute(migration_sql)?;
                    Ok(())
                })
                .await
            }
            DatabaseInner::Postgres(pool) => {
                let pool = pool.clone();
                with_pg_conn(pool, move |conn| {
                    conn.batch_execute(migration_sql)?;
                    Ok(())
                })
                .await
            }
        }
    }

    pub async fn get_portal_by_chat(
        &self,
        chat_type: &str,
        chat_id: &str,
    ) -> Result<Option<Portal>> {
        match &self.inner {
            DatabaseInner::Sqlite(pool) => {
                let pool = pool.clone();
                let chat_type = chat_type.to_owned();
                let chat_id = chat_id.to_owned();
                with_sqlite_conn(pool, move |conn| {
                    let mut rows = diesel::sql_query(
                        "SELECT chat_type, chat_id, room_id, name FROM portal WHERE chat_type = ? AND chat_id = ?",
                    )
                    .bind::<Text, _>(chat_type)
                    .bind::<Text, _>(chat_id)
                    .load::<PortalRow>(conn)?;
                    Ok(rows.pop().map(Into::into))
                })
                .await
            }
            DatabaseInner::Postgres(pool) => {
                let pool = pool.clone();
                let chat_type = chat_type.to_owned();
                let chat_id = chat_id.to_owned();
                with_pg_conn(pool, move |conn| {
                    let mut rows = diesel::sql_query(
                        "SELECT chat_type, chat_id, room_id, name FROM portal WHERE chat_type = $1 AND chat_id = $2",
                    )
                    .bind::<Text, _>(chat_type)
                    .bind::<Text, _>(chat_id)
                    .load::<PortalRow>(conn)?;
                    Ok(rows.pop().map(Into::into))
                })
                .await
            }
        }
    }

    pub async fn get_portal_by_room(&self, room_id: &str) -> Result<Option<Portal>> {
        match &self.inner {
            DatabaseInner::Sqlite(pool) => {
                let pool = pool.clone();
                let room_id = room_id.to_owned();
                with_sqlite_conn(pool, move |conn| {
                    let mut rows = diesel::sql_query(
                        "SELECT chat_type, chat_id, room_id, name FROM portal WHERE room_id = ?",
                    )
                    .bind::<Text, _>(room_id)
                    .load::<PortalRow>(conn)?;
                    Ok(rows.pop().map(Into::into))
                })
                .await
            }
            DatabaseInner::Postgres(pool) => {
                let pool = pool.clone();
                let room_id = room_id.to_owned();
                with_pg_conn(pool, move |conn| {
                    let mut rows = diesel::sql_query(
                        "SELECT chat_type, chat_id, room_id, name FROM portal WHERE room_id = $1",
                    )
                    .bind::<Text, _>(room_id)
                    .load::<PortalRow>(conn)?;
                    Ok(rows.pop().map(Into::into))
                })
                .await
            }
        }
    }

    pub async fn upsert_portal(&self, portal: &Portal) -> Result<()> {
        let portal = portal.clone();
        let now = chrono::Utc::now().timestamp_millis();
        match &self.inner {
            DatabaseInner::Sqlite(pool) => {
                let pool = pool.clone();
                with_sqlite_conn(pool, move |conn| {
                    diesel::sql_query(
                        "INSERT INTO portal (chat_type, chat_id, room_id, name, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?)\
                         ON CONFLICT(chat_type, chat_id) DO UPDATE SET room_id = excluded.room_id, name = excluded.name, updated_at = excluded.updated_at",
                    )
                    .bind::<Text, _>(portal.chat_type)
                    .bind::<Text, _>(portal.chat_id)
                    .bind::<Text, _>(portal.room_id)
                    .bind::<Text, _>(portal.name)
                    .bind::<BigInt, _>(now)
                    .bind::<BigInt, _>(now)
                    .execute(conn)?;
                    Ok(())
                })
                .await
            }
            DatabaseInner::Postgres(pool) => {
                let pool = pool.clone();
                with_pg_conn(pool, move |conn| {
                    diesel::sql_query(
                        "INSERT INTO portal (chat_type, chat_id, room_id, name, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6)\
                         ON CONFLICT(chat_type, chat_id) DO UPDATE SET room_id = EXCLUDED.room_id, name = EXCLUDED.name, updated_at = EXCLUDED.updated_at",
                    )
                    .bind::<Text, _>(portal.chat_type)
                    .bind::<Text, _>(portal.chat_id)
                    .bind::<Text, _>(portal.room_id)
                    .bind::<Text, _>(portal.name)
                    .bind::<BigInt, _>(now)
                    .bind::<BigInt, _>(now)
                    .execute(conn)?;
                    Ok(())
                })
                .await
            }
        }
    }

    pub async fn insert_message_if_absent(
        &self,
        source: &str,
        source_msg_id: &str,
        room_id: &str,
        chat_type: &str,
        chat_id: &str,
    ) -> Result<bool> {
        let source = source.to_owned();
        let source_msg_id = source_msg_id.to_owned();
        let room_id = room_id.to_owned();
        let chat_type = chat_type.to_owned();
        let chat_id = chat_id.to_owned();
        let now = chrono::Utc::now().timestamp_millis();
        match &self.inner {
            DatabaseInner::Sqlite(pool) => {
                let pool = pool.clone();
                with_sqlite_conn(pool, move |conn| {
                    let rows = diesel::sql_query(
                        "INSERT INTO message_map (source, source_msg_id, room_id, chat_type, chat_id, created_at) VALUES (?, ?, ?, ?, ?, ?)\
                         ON CONFLICT(source, source_msg_id) DO NOTHING",
                    )
                    .bind::<Text, _>(source)
                    .bind::<Text, _>(source_msg_id)
                    .bind::<Text, _>(room_id)
                    .bind::<Text, _>(chat_type)
                    .bind::<Text, _>(chat_id)
                    .bind::<BigInt, _>(now)
                    .execute(conn)?;
                    Ok(rows > 0)
                })
                .await
            }
            DatabaseInner::Postgres(pool) => {
                let pool = pool.clone();
                with_pg_conn(pool, move |conn| {
                    let rows = diesel::sql_query(
                        "INSERT INTO message_map (source, source_msg_id, room_id, chat_type, chat_id, created_at) VALUES ($1, $2, $3, $4, $5, $6)\
                         ON CONFLICT(source, source_msg_id) DO NOTHING",
                    )
                    .bind::<Text, _>(source)
                    .bind::<Text, _>(source_msg_id)
                    .bind::<Text, _>(room_id)
                    .bind::<Text, _>(chat_type)
                    .bind::<Text, _>(chat_id)
                    .bind::<BigInt, _>(now)
                    .execute(conn)?;
                    Ok(rows > 0)
                })
                .await
            }
        }
    }

    pub async fn update_matrix_event_id(
        &self,
        source: &str,
        source_msg_id: &str,
        matrix_event_id: &str,
    ) -> Result<()> {
        let source = source.to_owned();
        let source_msg_id = source_msg_id.to_owned();
        let matrix_event_id = matrix_event_id.to_owned();
        match &self.inner {
            DatabaseInner::Sqlite(pool) => {
                let pool = pool.clone();
                with_sqlite_conn(pool, move |conn| {
                    diesel::sql_query(
                        "UPDATE message_map SET matrix_event_id = ? WHERE source = ? AND source_msg_id = ?",
                    )
                    .bind::<Text, _>(matrix_event_id)
                    .bind::<Text, _>(source)
                    .bind::<Text, _>(source_msg_id)
                    .execute(conn)?;
                    Ok(())
                })
                .await
            }
            DatabaseInner::Postgres(pool) => {
                let pool = pool.clone();
                with_pg_conn(pool, move |conn| {
                    diesel::sql_query(
                        "UPDATE message_map SET matrix_event_id = $1 WHERE source = $2 AND source_msg_id = $3",
                    )
                    .bind::<Text, _>(matrix_event_id)
                    .bind::<Text, _>(source)
                    .bind::<Text, _>(source_msg_id)
                    .execute(conn)?;
                    Ok(())
                })
                .await
            }
        }
    }

    pub async fn update_qq_message_id(
        &self,
        source: &str,
        source_msg_id: &str,
        qq_message_id: &str,
    ) -> Result<()> {
        let source = source.to_owned();
        let source_msg_id = source_msg_id.to_owned();
        let qq_message_id = qq_message_id.to_owned();
        match &self.inner {
            DatabaseInner::Sqlite(pool) => {
                let pool = pool.clone();
                with_sqlite_conn(pool, move |conn| {
                    diesel::sql_query(
                        "UPDATE message_map SET qq_message_id = ? WHERE source = ? AND source_msg_id = ?",
                    )
                    .bind::<Text, _>(qq_message_id)
                    .bind::<Text, _>(source)
                    .bind::<Text, _>(source_msg_id)
                    .execute(conn)?;
                    Ok(())
                })
                .await
            }
            DatabaseInner::Postgres(pool) => {
                let pool = pool.clone();
                with_pg_conn(pool, move |conn| {
                    diesel::sql_query(
                        "UPDATE message_map SET qq_message_id = $1 WHERE source = $2 AND source_msg_id = $3",
                    )
                    .bind::<Text, _>(qq_message_id)
                    .bind::<Text, _>(source)
                    .bind::<Text, _>(source_msg_id)
                    .execute(conn)?;
                    Ok(())
                })
                .await
            }
        }
    }

    pub async fn mark_transaction_processed(&self, txn_id: &str) -> Result<bool> {
        let txn_id = txn_id.to_owned();
        let now = chrono::Utc::now().timestamp_millis();
        match &self.inner {
            DatabaseInner::Sqlite(pool) => {
                let pool = pool.clone();
                with_sqlite_conn(pool, move |conn| {
                    let rows = diesel::sql_query(
                        "INSERT INTO processed_txn (txn_id, processed_at) VALUES (?, ?)\
                         ON CONFLICT(txn_id) DO NOTHING",
                    )
                    .bind::<Text, _>(txn_id)
                    .bind::<BigInt, _>(now)
                    .execute(conn)?;
                    Ok(rows > 0)
                })
                .await
            }
            DatabaseInner::Postgres(pool) => {
                let pool = pool.clone();
                with_pg_conn(pool, move |conn| {
                    let rows = diesel::sql_query(
                        "INSERT INTO processed_txn (txn_id, processed_at) VALUES ($1, $2)\
                         ON CONFLICT(txn_id) DO NOTHING",
                    )
                    .bind::<Text, _>(txn_id)
                    .bind::<BigInt, _>(now)
                    .execute(conn)?;
                    Ok(rows > 0)
                })
                .await
            }
        }
    }

    pub async fn upsert_qq_user(&self, user: &QQUser) -> Result<()> {
        let user = user.clone();
        let now = chrono::Utc::now().timestamp_millis();
        match &self.inner {
            DatabaseInner::Sqlite(pool) => {
                let pool = pool.clone();
                with_sqlite_conn(pool, move |conn| {
                    diesel::sql_query(
                        "INSERT INTO qq_user (qq_user_id, mxid, displayname, avatar_url, updated_at) VALUES (?, ?, ?, ?, ?)\
                         ON CONFLICT(qq_user_id) DO UPDATE SET mxid = excluded.mxid, displayname = excluded.displayname, avatar_url = excluded.avatar_url, updated_at = excluded.updated_at",
                    )
                    .bind::<Text, _>(user.qq_user_id)
                    .bind::<Text, _>(user.mxid)
                    .bind::<Text, _>(user.displayname)
                    .bind::<Nullable<Text>, _>(user.avatar_url)
                    .bind::<BigInt, _>(now)
                    .execute(conn)?;
                    Ok(())
                })
                .await
            }
            DatabaseInner::Postgres(pool) => {
                let pool = pool.clone();
                with_pg_conn(pool, move |conn| {
                    diesel::sql_query(
                        "INSERT INTO qq_user (qq_user_id, mxid, displayname, avatar_url, updated_at) VALUES ($1, $2, $3, $4, $5)\
                         ON CONFLICT(qq_user_id) DO UPDATE SET mxid = EXCLUDED.mxid, displayname = EXCLUDED.displayname, avatar_url = EXCLUDED.avatar_url, updated_at = EXCLUDED.updated_at",
                    )
                    .bind::<Text, _>(user.qq_user_id)
                    .bind::<Text, _>(user.mxid)
                    .bind::<Text, _>(user.displayname)
                    .bind::<Nullable<Text>, _>(user.avatar_url)
                    .bind::<BigInt, _>(now)
                    .execute(conn)?;
                    Ok(())
                })
                .await
            }
        }
    }

    pub async fn get_qq_user(&self, qq_user_id: &str) -> Result<Option<QQUser>> {
        match &self.inner {
            DatabaseInner::Sqlite(pool) => {
                let pool = pool.clone();
                let qq_user_id = qq_user_id.to_owned();
                with_sqlite_conn(pool, move |conn| {
                    let mut rows = diesel::sql_query(
                        "SELECT qq_user_id, mxid, displayname, avatar_url FROM qq_user WHERE qq_user_id = ?",
                    )
                    .bind::<Text, _>(qq_user_id)
                    .load::<QQUserRow>(conn)?;
                    Ok(rows.pop().map(Into::into))
                })
                .await
            }
            DatabaseInner::Postgres(pool) => {
                let pool = pool.clone();
                let qq_user_id = qq_user_id.to_owned();
                with_pg_conn(pool, move |conn| {
                    let mut rows = diesel::sql_query(
                        "SELECT qq_user_id, mxid, displayname, avatar_url FROM qq_user WHERE qq_user_id = $1",
                    )
                    .bind::<Text, _>(qq_user_id)
                    .load::<QQUserRow>(conn)?;
                    Ok(rows.pop().map(Into::into))
                })
                .await
            }
        }
    }

    pub async fn count_portals(&self) -> Result<i64> {
        match &self.inner {
            DatabaseInner::Sqlite(pool) => {
                let pool = pool.clone();
                with_sqlite_conn(pool, move |conn| {
                    let mut rows = diesel::sql_query("SELECT COUNT(*) AS cnt FROM portal")
                        .load::<CountRow>(conn)?;
                    Ok(rows.pop().map(|row| row.cnt).unwrap_or(0))
                })
                .await
            }
            DatabaseInner::Postgres(pool) => {
                let pool = pool.clone();
                with_pg_conn(pool, move |conn| {
                    let mut rows = diesel::sql_query("SELECT COUNT(*) AS cnt FROM portal")
                        .load::<CountRow>(conn)?;
                    Ok(rows.pop().map(|row| row.cnt).unwrap_or(0))
                })
                .await
            }
        }
    }
}

async fn with_sqlite_conn<T, F>(pool: SqlitePool, op: F) -> Result<T>
where
    T: Send + 'static,
    F: FnOnce(&mut SqliteConnection) -> Result<T> + Send + 'static,
{
    task::spawn_blocking(move || {
        let mut conn = pool
            .get()
            .context("failed to get sqlite connection from pool")?;
        op(&mut conn)
    })
    .await?
}

async fn with_pg_conn<T, F>(pool: PgPool, op: F) -> Result<T>
where
    T: Send + 'static,
    F: FnOnce(&mut PgConnection) -> Result<T> + Send + 'static,
{
    task::spawn_blocking(move || {
        let mut conn = pool
            .get()
            .context("failed to get postgres connection from pool")?;
        op(&mut conn)
    })
    .await?
}

fn normalize_sqlite_uri(uri: &str) -> String {
    if uri == "sqlite::memory:" {
        return ":memory:".to_owned();
    }
    if let Some(stripped) = uri.strip_prefix("sqlite://") {
        return stripped.to_owned();
    }
    uri.to_owned()
}
