use crate::database::models::{BinanceCandle, CalculatedIndicatorBatch, CandleData, IndicatorConfig};
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use deadpool_postgres::{Config, Pool, Runtime};
use tokio_postgres::{NoTls, types::Type};
use tracing::{error, info};

pub struct PostgresManager {
    pool: Pool,
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
        let mut cfg = Config::new();
        cfg.host = Some(host.to_string());
        cfg.port = Some(port);
        cfg.user = Some(user.to_string());
        cfg.password = Some(password.to_string());
        cfg.dbname = Some(dbname.to_string());
        
        let mut pool_cfg = deadpool_postgres::PoolConfig::new(max_connections);
        // Set config to Some(pool_cfg) to fix type mismatch
        cfg.pool = Some(pool_cfg);

        let pool = cfg
            .create_pool(Some(Runtime::Tokio1), NoTls)
            .context("Failed to create database connection pool")?;

        Ok(Self { pool })
    }

    // Create tables if they don't exist
    pub async fn init_tables(&self) -> Result<()> {
        let client = self.pool.get().await?;

        // Create indicator_config table if it doesn't exist
        client
            .execute(
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
                )",
                &[],
            )
            .await?;

        // Create calculated_indicators hypertable
        client
            .execute(
                "CREATE TABLE IF NOT EXISTS calculated_indicators (
                    id SERIAL PRIMARY KEY,
                    symbol VARCHAR NOT NULL,
                    interval VARCHAR NOT NULL,
                    indicator_type VARCHAR NOT NULL,
                    indicator_name VARCHAR NOT NULL,
                    parameters JSONB NOT NULL,
                    time TIMESTAMPTZ NOT NULL,
                    value JSONB NOT NULL,
                    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                    UNIQUE(symbol, interval, indicator_name, parameters, time)
                )",
                &[],
            )
            .await?;

        // Check if the extension is available
        let res = client
            .query_one(
                "SELECT COUNT(*) FROM pg_extension WHERE extname = 'timescaledb'",
                &[],
            )
            .await?;
        
        let count: i64 = res.get(0);
        
        if count > 0 {
            // Convert to hypertable if timescaledb is available
            let res = client
                .execute(
                    "SELECT create_hypertable('calculated_indicators', 'time', if_not_exists => TRUE)",
                    &[],
                )
                .await;
            
            if let Err(e) = res {
                // If it fails because the table is already a hypertable, that's fine
                if !e.to_string().contains("already a hypertable") {
                    return Err(e.into());
                }
            }
            
            // Set up compression policy for calculated_indicators
            client
                .execute(
                    "ALTER TABLE calculated_indicators SET (
                        timescaledb.compress,
                        timescaledb.compress_segmentby = 'symbol,interval,indicator_name'
                    )",
                    &[],
                )
                .await?;
            
            // Add compression policy
            let res = client
                .execute(
                    "SELECT add_compression_policy('calculated_indicators', INTERVAL '7 days', if_not_exists => TRUE)",
                    &[],
                )
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
        client
            .execute(
                "CREATE INDEX IF NOT EXISTS idx_calculated_indicators_symbol_interval ON calculated_indicators(symbol, interval)",
                &[],
            )
            .await?;

        client
            .execute(
                "CREATE INDEX IF NOT EXISTS idx_calculated_indicators_time ON calculated_indicators(time DESC)",
                &[],
            )
            .await?;

        info!("Database tables initialized successfully");
        Ok(())
    }

    // Get all enabled indicator configurations
    pub async fn get_enabled_indicator_configs(&self) -> Result<Vec<IndicatorConfig>> {
        let client = self.pool.get().await?;
        
        // Using query_as instead of manual mapping to leverage the FromRow trait
        let configs = sqlx::query_as::<_, IndicatorConfig>(
            "SELECT id, symbol, interval, indicator_type, indicator_name, parameters, enabled, created_at, updated_at 
            FROM indicator_config 
            WHERE enabled = TRUE"
        )
        .fetch_all(&*client)
        .await?;

        Ok(configs)
    }

    // Get unique symbol-interval pairs from the configuration
    pub async fn get_unique_symbol_intervals(&self) -> Result<Vec<(String, String)>> {
        let client = self.pool.get().await?;
        
        let rows = client
            .query(
                "SELECT DISTINCT symbol, interval 
                FROM indicator_config 
                WHERE enabled = TRUE",
                &[],
            )
            .await?;

        let pairs = rows
            .into_iter()
            .map(|row| (row.get(0), row.get(1)))
            .collect();

        Ok(pairs)
    }

    // Get candle data for a specific symbol and interval
    pub async fn get_candle_data(&self, symbol: &str, interval: &str) -> Result<CandleData> {
        let client = self.pool.get().await?;
        
        // Use sqlx to get the data with proper type handling
        let candles = sqlx::query_as::<_, BinanceCandle>(
            "SELECT id, symbol, interval, open_time, open_price, high_price, low_price, close_price, volume, 
            close_time, quote_asset_volume, number_of_trades 
            FROM binance_candles 
            WHERE symbol = $1 AND interval = $2 
            ORDER BY open_time ASC"
        )
        .bind(symbol)
        .bind(interval)
        .fetch_all(&*client)
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
        let client = self.pool.get().await?;
        
        // Using a parameterized query with proper parameter handling
        let row = sqlx::query(
            "SELECT MAX(time) 
            FROM calculated_indicators 
            WHERE symbol = $1 AND interval = $2 AND indicator_name = $3 AND parameters = $4"
        )
        .bind(symbol)
        .bind(interval)
        .bind(indicator_name)
        .bind(parameters)
        .fetch_optional(&*client)
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

        let client = self.pool.get().await?;
        
        for indicator in batch {
            // Use sqlx's query builder for proper parameter handling
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
            .execute(&*client)
            .await;
            
            if let Err(e) = result {
                error!("Error inserting indicator: {}", e);
                // Continue with the rest of the batch
            }
        }

        Ok(())
    }
}
