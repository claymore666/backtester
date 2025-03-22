// This file defines the database schema for SQL generation or migration tools
// For TimescaleDB, we're primarily using SQL directly rather than ORM-based schema

// SQL schema definitions for reference
pub const CREATE_CANDLES_TABLE: &str = r#"
CREATE TABLE IF NOT EXISTS binance_candles (
    id SERIAL PRIMARY KEY,
    symbol VARCHAR NOT NULL,
    interval VARCHAR NOT NULL,
    open_time TIMESTAMPTZ NOT NULL,
    open_price DOUBLE PRECISION NOT NULL,
    high_price DOUBLE PRECISION NOT NULL,
    low_price DOUBLE PRECISION NOT NULL,
    close_price DOUBLE PRECISION NOT NULL,
    volume DOUBLE PRECISION NOT NULL,
    close_time TIMESTAMPTZ NOT NULL,
    quote_asset_volume DOUBLE PRECISION NOT NULL,
    number_of_trades INTEGER NOT NULL,
    UNIQUE(symbol, interval, open_time)
);
"#;

pub const CREATE_INDICATOR_CONFIG_TABLE: &str = r#"
CREATE TABLE IF NOT EXISTS indicator_config (
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
);
"#;

pub const CREATE_CALCULATED_INDICATORS_TABLE: &str = r#"
CREATE TABLE IF NOT EXISTS calculated_indicators (
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
);
"#;

pub const CREATE_HYPERTABLE: &str = r#"
SELECT create_hypertable('calculated_indicators', 'time', if_not_exists => TRUE);
"#;

pub const SETUP_COMPRESSION: &str = r#"
ALTER TABLE calculated_indicators SET (
    timescaledb.compress,
    timescaledb.compress_segmentby = 'symbol,interval,indicator_name'
);
"#;

pub const ADD_COMPRESSION_POLICY: &str = r#"
SELECT add_compression_policy('calculated_indicators', INTERVAL '7 days', if_not_exists => TRUE);
"#;

pub const CREATE_INDICES: &str = r#"
CREATE INDEX IF NOT EXISTS idx_calculated_indicators_symbol_interval ON calculated_indicators(symbol, interval);
CREATE INDEX IF NOT EXISTS idx_calculated_indicators_time ON calculated_indicators(time DESC);
"#;
