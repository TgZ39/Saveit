use std::fs::create_dir_all;

use directories::ProjectDirs;
use sqlx::migrate::MigrateDatabase;
use sqlx::{Sqlite, SqlitePool};
use tracing::*;

use crate::source::Source;
use crate::ui::Application;

#[macro_export]
macro_rules! db_version {
    () => {
        format!("sources-{}.db", &env!("CARGO_PKG_VERSION")[0..3])
    };
}

pub async fn establish_connection() -> Result<SqlitePool, sqlx::Error> {
    let db_path = ProjectDirs::from("com", "tgz39", "saveit")
        .unwrap()
        .data_dir()
        .to_owned();

    // create DB path if it doesn't exist
    if !&db_path.exists() {
        debug!("Creating database directories...");
        create_dir_all(&db_path).expect("Error creating database directories");
    }

    // DB path + DB name
    let db_loc = format!(
        "sqlite://{}/{}",
        &db_path.to_str().unwrap().to_owned(),
        db_version!()
    );

    // create DB file if it doesn't exist
    if !Sqlite::database_exists(&db_loc).await.unwrap_or(false) {
        debug!("Database doesn't exists. Creating database {}", &db_loc);

        match Sqlite::create_database(&db_loc).await {
            Ok(_) => {
                debug!("Successfully created database")
            }
            Err(e) => {
                error!("Error creating database: {}", e)
            }
        }
    }

    // connect to DB
    debug!("Establishing connection to database {}...", &db_loc);
    SqlitePool::connect(&db_loc).await
}

pub async fn insert_source(source: &Source, pool: &SqlitePool) -> Result<(), sqlx::Error> {
    debug!("Inserting source into database: {:#?}", &source);

    sqlx::query("INSERT INTO sources (title, url, author, published_date, viewed_date, published_date_unknown, comment) VALUES ($1, $2, $3, $4, $5, $6, $7)")
        .bind(&source.title)
        .bind(&source.url)
        .bind(&source.author)
        .bind(source.published_date)
        .bind(source.viewed_date)
        .bind(source.published_date_unknown)
        .bind(&source.comment)
        .execute(pool)
        .await?;

    Ok(())
}

pub async fn get_all_sources(pool: &SqlitePool) -> Result<Vec<Source>, sqlx::Error> {
    debug!("Fetching all sources");

    sqlx::query_as::<_, Source>("SELECT * FROM sources")
        .fetch_all(pool)
        .await
}

pub async fn delete_source(id: i64, pool: &SqlitePool) -> Result<(), sqlx::Error> {
    debug!("Deleting source: {}", id);

    sqlx::query("DELETE FROM sources WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await
        .map(|_| ())
}

pub async fn update_source(id: i64, source: &Source, pool: &SqlitePool) -> Result<(), sqlx::Error> {
    debug!("Updating source: {} to {:#?}", id, &source);

    sqlx::query("UPDATE sources SET title = $1, url = $2, author = $3, published_date = $4, viewed_date = $5, published_date_unknown = $6, comment = $7 WHERE id = $8")
        .bind(&source.title)
        .bind(&source.url)
        .bind(&source.author)
        .bind(source.published_date)
        .bind(source.viewed_date)
        .bind(source.published_date_unknown)
        .bind(&source.comment)
        .bind(id)
        .execute(pool)
        .await
        .map(|_| ())
}

// async delete source
pub fn handle_delete_source(id: i64, app: &Application) {
    let source_cache = app.sources_cache.clone();
    let pool = app.pool.clone();

    tokio::task::spawn(async move {
        delete_source(id, &pool)
            .await
            .expect("Error deleting source");

        // update source cache
        *source_cache.write().unwrap() =
            get_all_sources(&pool).await.expect("Error loading sources");
    });
}

// async update source
pub fn handle_update_source(id: i64, source: &Source, app: &Application) {
    let source = source.clone();
    let source_cache = app.sources_cache.clone();
    let pool = app.pool.clone();

    tokio::task::spawn(async move {
        update_source(id, &source, &pool)
            .await
            .expect("Error deleting source");

        // update source cache
        *source_cache.write().unwrap() =
            get_all_sources(&pool).await.expect("Error loading sources");
    });
}

// async save source
pub fn handle_source_save(app: &Application) {
    let source = app.get_source();
    let source_cache = app.sources_cache.clone();
    let pool = app.pool.clone();

    tokio::task::spawn(async move {
        insert_source(&source, &pool)
            .await
            .expect("Error inserting source in database");

        // update source cache
        *source_cache.write().unwrap() =
            get_all_sources(&pool).await.expect("Error loading sources");
    });
}
