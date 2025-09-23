//! Database operations and connection management

use anyhow::{Context, Result};
use config::DatabaseConfig;
use sqlx::{sqlite::SqlitePool, Pool, Sqlite};

/// Database connection manager
#[derive(Debug, Clone)]
pub struct Database {
    pool: Pool<Sqlite>,
}

impl Database {
    /// Create a new database connection
    pub async fn new(config: &DatabaseConfig) -> Result<Self> {
        let pool = SqlitePool::connect_with(
            sqlx::sqlite::SqliteConnectOptions::new()
                .filename(&config.url.strip_prefix("sqlite:").unwrap_or(&config.url))
                .create_if_missing(true)
                .journal_mode(if config.wal_mode {
                    sqlx::sqlite::SqliteJournalMode::Wal
                } else {
                    sqlx::sqlite::SqliteJournalMode::Delete
                })
        )
        .await
        .context("Failed to connect to database")?;

        Ok(Self { pool })
    }

    /// Create an in-memory database for testing
    #[cfg(test)]
    pub async fn new_in_memory() -> Result<Self> {
        let pool = SqlitePool::connect(":memory:")
            .await
            .context("Failed to create in-memory database")?;
        
        let db = Self { pool };
        db.migrate().await?;
        Ok(db)
    }

    /// Run database migrations
    pub async fn migrate(&self) -> Result<()> {
        // TODO: Implement proper migrations
        // For now, create basic tables
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS bundles (
                id TEXT PRIMARY KEY,
                tx1_hash TEXT NOT NULL,
                tx2_hash TEXT,
                state TEXT NOT NULL DEFAULT 'queued',
                payment_amount_wei TEXT NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                expires_at DATETIME,
                block_hash TEXT,
                block_number INTEGER,
                gas_used INTEGER
            )
            "#,
        )
        .execute(&self.pool)
        .await
        .context("Failed to create bundles table")?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS relay_submissions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                bundle_id TEXT NOT NULL,
                relay_name TEXT NOT NULL,
                submitted_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                status TEXT NOT NULL DEFAULT 'pending',
                response_data TEXT,
                error_message TEXT,
                retry_count INTEGER DEFAULT 0,
                FOREIGN KEY (bundle_id) REFERENCES bundles(id)
            )
            "#,
        )
        .execute(&self.pool)
        .await
        .context("Failed to create relay_submissions table")?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS daily_spending (
                date DATE PRIMARY KEY,
                total_amount_wei TEXT NOT NULL DEFAULT '0',
                bundle_count INTEGER DEFAULT 0,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )
            "#,
        )
        .execute(&self.pool)
        .await
        .context("Failed to create daily_spending table")?;

        Ok(())
    }

    /// Perform a health check on the database
    pub async fn health_check(&self) -> Result<()> {
        sqlx::query("SELECT 1")
            .execute(&self.pool)
            .await
            .context("Database health check failed")?;
        Ok(())
    }

    /// Close the database connection
    pub async fn close(&self) -> Result<()> {
        self.pool.close().await;
        Ok(())
    }

    /// Get the database pool
    pub fn pool(&self) -> &Pool<Sqlite> {
        &self.pool
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_in_memory_database() {
        let db = Database::new_in_memory().await.unwrap();
        assert!(db.health_check().await.is_ok());
    }

    #[tokio::test]
    async fn test_database_migration() {
        let db = Database::new_in_memory().await.unwrap();
        
        // Check that tables were created
        let result = sqlx::query("SELECT name FROM sqlite_master WHERE type='table'")
            .fetch_all(db.pool())
            .await
            .unwrap();
        
        let table_names: Vec<String> = result
            .iter()
            .map(|row| sqlx::Row::get::<String, _>(row, "name"))
            .collect();
        
        assert!(table_names.contains(&"bundles".to_string()));
        assert!(table_names.contains(&"relay_submissions".to_string()));
        assert!(table_names.contains(&"daily_spending".to_string()));
    }
}
