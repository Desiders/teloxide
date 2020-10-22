use super::{serializer::Serializer, Storage};
use futures::future::BoxFuture;
use serde::{de::DeserializeOwned, Serialize};
use sqlx::sqlite::SqlitePool;
use sqlx::Executor;
use std::{
    convert::Infallible,
    fmt::{Debug, Display},
    sync::Arc,
};
use thiserror::Error;

// An error returned from [`SqliteStorage`].
#[derive(Debug, Error)]
pub enum SqliteStorageError<SE>
where
    SE: Debug + Display,
{
    #[error("dialogue serialization error: {0}")]
    SerdeError(SE),
    #[error("sqlite error: {0}")]
    SqliteError(#[from] sqlx::Error),
}

// TODO: make JSON serializer to be default
pub struct SqliteStorage<S> {
    pool: SqlitePool,
    serializer: S,
}

#[derive(sqlx::FromRow)]
struct DialogueDBRow {
    dialogue: Vec<u8>,
}

impl<S> SqliteStorage<S> {
    pub async fn open(
        path: &str,
        serializer: S,
    ) -> Result<Arc<Self>, SqliteStorageError<Infallible>> {
        let pool = SqlitePool::connect(format!("sqlite:{}?mode=rwc", path).as_str()).await?;
        let mut conn = pool.acquire().await?;
        sqlx::query(
            r#"
CREATE TABLE IF NOT EXISTS teloxide_dialogues (
    chat_id BIGINT PRIMARY KEY,
    dialogue BLOB NOT NULL
);
        "#,
        )
        .execute(&mut conn)
        .await?;

        Ok(Arc::new(Self { pool, serializer }))
    }
}

async fn get_dialogue(
    pool: &SqlitePool,
    chat_id: i64,
) -> Result<Option<Box<Vec<u8>>>, sqlx::Error> {
    match sqlx::query_as::<_, DialogueDBRow>(
        "SELECT dialogue FROM teloxide_dialogues WHERE chat_id = ?",
    )
    .bind(chat_id)
    .fetch_optional(pool)
    .await?
    {
        Some(r) => Ok(Some(Box::new(r.dialogue))),
        _ => Ok(None),
    }
}

impl<S, D> Storage<D> for SqliteStorage<S>
where
    S: Send + Sync + Serializer<D> + 'static,
    D: Send + Serialize + DeserializeOwned + 'static,
    <S as Serializer<D>>::Error: Debug + Display,
{
    type Error = SqliteStorageError<<S as Serializer<D>>::Error>;

    fn remove_dialogue(
        self: Arc<Self>,
        chat_id: i64,
    ) -> BoxFuture<'static, Result<Option<D>, Self::Error>> {
        Box::pin(async move {
            match get_dialogue(&self.pool, chat_id).await? {
                None => Ok(None),
                Some(d) => {
                    let prev_dialogue =
                        self.serializer.deserialize(&d).map_err(SqliteStorageError::SerdeError)?;
                    sqlx::query("DELETE FROM teloxide_dialogues WHERE chat_id = ?")
                        .bind(chat_id)
                        .execute(&self.pool)
                        .await?;
                    Ok(Some(prev_dialogue))
                }
            }
        })
    }

    fn update_dialogue(
        self: Arc<Self>,
        chat_id: i64,
        dialogue: D,
    ) -> BoxFuture<'static, Result<Option<D>, Self::Error>> {
        Box::pin(async move {
            let serialized_dialogue =
                self.serializer.serialize(&dialogue).map_err(SqliteStorageError::SerdeError)?;
            let prev_dialogue = get_dialogue(&self.pool, chat_id).await?;

            self.pool
                .acquire()
                .await?
                .execute(
                    sqlx::query(
                        r#"
            INSERT INTO teloxide_dialogues VALUES (?, ?) WHERE chat_id = ?
            ON CONFLICT(chat_id) DO UPDATE SET dialogue=excluded.dialogue
                                "#,
                    )
                    .bind(chat_id)
                    .bind(serialized_dialogue),
                )
                .await?;

            Ok(match prev_dialogue {
                None => None,
                Some(d) => {
                    Some(self.serializer.deserialize(&d).map_err(SqliteStorageError::SerdeError)?)
                }
            })
        })
    }
}
