use crate::database::models::{BinanceCandle, CalculatedIndicatorBatch, CandleData, IndicatorConfig};
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use deadpool_postgres::{Config, Pool, Runtime};
use sqlx::{postgres::PgPoolOptions, PgPool, Row};
use tokio_postgres::NoTls;
use tracing::{error, info};

pub struct PostgresManager {
    pool: PgPool,
    pg_pool: Pool, // Keep the original pool for non-sqlx operations
}

impl PostgresManager {
    pub async fn new(
        host: &str,
        port: u16,
        user: &str,
        password: &str,
        dbname: &str,
        max_connections: usize,
    ) -> Result<Self> {
        // Create SQLx pool
        let connection_string = format!(
            "postgres://{}:{}@{}:{}/{}",
            user, password, host, port, dbname
        );
        
        let pool = PgPoolOptions::new()
            .max_connections(max_connections as u32)
            .connect(&connection_string)
            .await
            .context("Failed to create database connection pool")?;
            
        // Also create deadpool-postgres pool for compatibility
        let mut cfg = Config::new();
        cfg.host = Some(host.to_string());
        cfg.port = Some(port);
        cfg.user = Some(user.to_string());
        cfg.password = Some(password.to_string());
        cfg.dbname = Some(dbname.to_string());
        
        let pool_cfg = deadpool_postgres::PoolConfig::new(max_connections);
        cfg.pool = Some(pool_cfg);

        let pg_pool = cfg
            .create_pool(Some(Runtime::Tokio1), NoTls)
            .context("Failed to create database connection pool")?;

        Ok(Self { pool, pg_pool })
    }

    // Create tables if they don't exist
    pub async fn init_tables(&self) -> Result<()> {
        // Create indicator_config table if it doesn't exist
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS indicator_config (
                id SERIAL PRIMARY KEY,
                symbol VARCHAR NOT NULL,
                interval VARCHAR NOT NULL,
                indicator_type VARCHAR NOT NULL,
                indicator_name VARCHAR NOT NULL,
                parameters JSONB NOT NULL,
                enabled BOOLEAN NOT NULL DEFAULT TRUE,
                created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                UNIQUE(symbol, interval, indicator_name, parameters)
            )"
        )
        .execute(&self.pool)
        .await?;

	// Create calculated_indicators hypertable
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS calculated_indicators (
                id SERIAL PRIMARY KEY,
                symbol VARCHAR NOT NULL,
                interval VARCHAR NOT NULL,
                indicator_type VARCHAR NOT NULL,
                indicator_name VARCHAR NOT NULL,
                parameters JSONB NOT NULL,
                time TIMESTAMPTZ NOT NULL,
                value JSONB NOT NULL,
                created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
            )"
        )
        .execute(&self.pool)
        .await?;
        
        // Create the unique index that includes the time column (for TimescaleDB)
        sqlx::query(
            "CREATE UNIQUE INDEX IF NOT EXISTS idx_calculated_indicators_unique 
             ON calculated_indicators(symbol, interval, indicator_name, parameters, time)"
        )
        .execute(&self.pool)
        .await?;

        // Check if the extension is available
        let res = sqlx::query("SELECT COUNT(*) FROM pg_extension WHERE extname = 'timescaledb'")
            .fetch_one(&self.pool)
            .await?;
        
        let count: i64 = res.get(0);
        
        if count > 0 {
            // Convert to hypertable if timescaledb is available
            let res = sqlx::query(
                "SELECT create_hypertable('calculated_indicators', 'time', if_not_exists => TRUE)"
            )
            .execute(&self.pool)
            .await;
            
            if let Err(e) = res {
                // If it fails because the table is already a hypertable, that's fine
                if !e.to_string().contains("already a hypertable") {
                    return Err(e.into());
                }
            }
            
            // Set up compression policy for calculated_indicators
            sqlx::query(
                "ALTER TABLE calculated_indicators SET (
                    timescaledb.compress,
                    timescaledb.compress_segmentby = 'symbol,interval,indicator_name'
                )"
            )
            .execute(&self.pool)
            .await?;
            
            // Add compression policy
            let res = sqlx::query(
                "SELECT add_compression_policy('calculated_indicators', INTERVAL '7 days', if_not_exists => TRUE)"
            )
            .execute(&self.pool)
            .await;
            
            if let Err(e) = res {
                // It's OK if the policy already exists
                if !e.to_string().contains("already exists") {
                    return Err(e.into());
                }
            }
        } else {
            info!("TimescaleDB extension not available, skipping hypertable creation");
        }

        // Create indices for better query performance
        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_calculated_indicators_symbol_interval ON calculated_indicators(symbol, interval)"
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_calculated_indicators_time ON calculated_indicators(time DESC)"
        )
        .execute(&self.pool)
        .await?;

        info!("Database tables initialized successfully");
        Ok(())
    }

    // Get all enabled indicator configurations
    pub async fn get_enabled_indicator_configs(&self) -> Result<Vec<IndicatorConfig>> {
        let configs = sqlx::query_as::<_, IndicatorConfig>(
            "SELECT id, symbol, interval, indicator_type, indicator_name, parameters, enabled, created_at, updated_at 
            FROM indicator_config 
            WHERE enabled = TRUE"
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(configs)
    }

    // Get unique symbol-interval pairs from the configuration
    pub async fn get_unique_symbol_intervals(&self) -> Result<Vec<(String, String)>> {
        let rows = sqlx::query(
            "SELECT DISTINCT symbol, interval 
            FROM indicator_config 
            WHERE enabled = TRUE"
        )
        .fetch_all(&self.pool)
        .await?;

        let pairs = rows
            .into_iter()
            .map(|row| (row.get(0), row.get(1)))
            .collect();

        Ok(pairs)
    }

    // Get candle data for a specific symbol and interval
    pub async fn get_candle_data(&self, symbol: &str, interval: &str) -> Result<CandleData> {
        let candles = sqlx::query_as::<_, BinanceCandle>(
            "SELECT id, symbol, interval, open_time, open_price, high_price, low_price, close_price, volume, 
            close_time, quote_asset_volume, number_of_trades 
            FROM binance_candles 
            WHERE symbol = $1 AND interval = $2 
            ORDER BY open_time ASC"
        )
        .bind(symbol)
        .bind(interval)
        .fetch_all(&self.pool)
        .await?;

        if candles.is_empty() {
            return Ok(CandleData::new(symbol.to_string(), interval.to_string()));
        }

        Ok(CandleData::from_candles(candles))
    }

    // Get the last calculated time for a specific indicator
    pub async fn get_last_calculated_time(
        &self, 
        symbol: &str, 
        interval: &str, 
        indicator_name: &str, 
        parameters: &serde_json::Value
    ) -> Result<Option<DateTime<Utc>>> {
        let row = sqlx::query(
            "SELECT MAX(time) 
            FROM calculated_indicators 
            WHERE symbol = $1 AND interval = $2 AND indicator_name = $3 AND parameters = $4"
        )
        .bind(symbol)
        .bind(interval)
        .bind(indicator_name)
        .bind(parameters)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            let time: Option<DateTime<Utc>> = row.get(0);
            Ok(time)
        } else {
            Ok(None)
        }
    }

    // Batch insert calculated indicators
    pub async fn insert_calculated_indicators_batch(
        &self,
        batch: Vec<CalculatedIndicatorBatch>,
    ) -> Result<()> {
        if batch.is_empty() {
            return Ok(());
        }

        // Use a transaction for batch inserts
        let mut tx = self.pool.begin().await?;
        
        for indicator in batch {
            let result = sqlx::query(
                "INSERT INTO calculated_indicators 
                (symbol, interval, indicator_type, indicator_name, parameters, time, value) 
                VALUES ($1, $2, $3, $4, $5, $6, $7)
                ON CONFLICT (symbol, interval, indicator_name, parameters, time) 
                DO UPDATE SET value = EXCLUDED.value"
            )
            .bind(&indicator.symbol)
            .bind(&indicator.interval)
            .bind(&indicator.indicator_type)
            .bind(&indicator.indicator_name)
            .bind(&indicator.parameters)
            .bind(&indicator.time)
            .bind(&indicator.value)
            .execute(&mut *tx)
            .await;
            
            if let Err(e) = result {
                error!("Error inserting indicator: {}", e);
                // Continue with the rest of the batch
            }
        }
        
        // Commit the transaction
        tx.commit().await?;

        Ok(())
    }
}
