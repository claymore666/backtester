// src/database/postgres_ext.rs
//
// Extension methods for PostgresManager to support our strategy repository
//

use crate::database::postgres::PostgresManager;
use anyhow::Result;
use sqlx::{Row, postgres::{PgRow, PgQueryResult}};
use std::sync::Arc;

impl PostgresManager {
    /// Execute a simple query without parameters and return all rows
    pub async fn execute_query(&self, query: &str) -> Result<Vec<PgRow>> {
        let rows = sqlx::query(query)
            .fetch_all(&self.pool)
            .await?;
        Ok(rows)
    }
    
    /// Execute a query with parameters and return all rows
    pub async fn execute_query_with_params(&self, query: &str, params: &[&(dyn sqlx::types::Type<sqlx::Postgres> + Sync)]) -> Result<Vec<PgRow>> {
        let mut q = sqlx::query(query);
        for param in params {
            q = q.bind(*param);
        }
        let rows = q.fetch_all(&self.pool).await?;
        Ok(rows)
    }
    
    /// Execute a query and return an optional single row
    pub async fn execute_query_optional(&self, query: &str, params: &[&(dyn sqlx::types::Type<sqlx::Postgres> + Sync)]) -> Result<Option<PgRow>> {
        let mut q = sqlx::query(query);
        for param in params {
            q = q.bind(*param);
        }
        let row = q.fetch_optional(&self.pool).await?;
        Ok(row)
    }
    
    /// Execute a query that returns a single row
    pub async fn execute_query_one(&self, query: &str, params: &[&(dyn sqlx::types::Type<sqlx::Postgres> + Sync)]) -> Result<PgRow> {
        let mut q = sqlx::query(query);
        for param in params {
            q = q.bind(*param);
        }
        let row = q.fetch_one(&self.pool).await?;
        Ok(row)
    }
    
    /// Execute a query that doesn't return any rows, but just affects rows
    pub async fn execute_command(&self, query: &str, params: &[&(dyn sqlx::types::Type<sqlx::Postgres> + Sync)]) -> Result<u64> {
        let mut q = sqlx::query(query);
        for param in params {
            q = q.bind(*param);
        }
        let result = q.execute(&self.pool).await?;
        Ok(result.rows_affected())
    }
    
    /// Begin a transaction
    pub async fn begin_transaction(&self) -> Result<sqlx::Transaction<'_, sqlx::Postgres>> {
        let tx = self.pool.begin().await?;
        Ok(tx)
    }
    
    /// Execute a query within a transaction
    pub async fn execute_query_with_transaction<'a>(
        &self,
        tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
        query: &str,
        params: &[&(dyn sqlx::types::Type<sqlx::Postgres> + Sync)]
    ) -> Result<u64> {
        let mut q = sqlx::query(query);
        for param in params {
            q = q.bind(*param);
        }
        let result = q.execute(&mut **tx).await?;
        Ok(result.rows_affected())
    }
    
    /// Commit a transaction
    pub async fn commit_transaction(
        &self, 
        tx: sqlx::Transaction<'_, sqlx::Postgres>
    ) -> Result<()> {
        tx.commit().await?;
        Ok(())
    }
}
