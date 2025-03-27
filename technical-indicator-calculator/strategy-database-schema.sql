-- Database schema for trading strategies
-- To be executed on the existing binancedb database

-- Create strategies table
CREATE TABLE IF NOT EXISTS strategies (
    id UUID PRIMARY KEY,
    name VARCHAR NOT NULL,
    description TEXT,
    version VARCHAR NOT NULL,
    author VARCHAR,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    assets JSONB NOT NULL, -- Array of symbols
    timeframes JSONB NOT NULL, -- Array of intervals
    parameters JSONB NOT NULL, -- Strategy parameters
    risk_management JSONB NOT NULL, -- Risk settings
    metadata JSONB -- Optional metadata
);

-- Create strategy_indicators table
CREATE TABLE IF NOT EXISTS strategy_indicators (
    id SERIAL PRIMARY KEY,
    strategy_id UUID NOT NULL REFERENCES strategies(id) ON DELETE CASCADE,
    indicator_id VARCHAR NOT NULL, -- Unique ID within a strategy
    indicator_type VARCHAR NOT NULL, -- Type of indicator (oscillator, overlap, etc.)
    indicator_name VARCHAR NOT NULL, -- Name of indicator (RSI, MACD, etc.)
    parameters JSONB NOT NULL, -- Parameters for the indicator
    description TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (strategy_id, indicator_id)
);

-- Create strategy_rules table
CREATE TABLE IF NOT EXISTS strategy_rules (
    id SERIAL PRIMARY KEY,
    strategy_id UUID NOT NULL REFERENCES strategies(id) ON DELETE CASCADE,
    rule_id VARCHAR NOT NULL, -- Unique ID within a strategy
    name VARCHAR NOT NULL,
    condition JSONB NOT NULL, -- Condition definition
    action JSONB NOT NULL, -- Action to take
    priority INTEGER NOT NULL DEFAULT 0,
    description TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (strategy_id, rule_id)
);

-- Create strategy_backtest_results table
CREATE TABLE IF NOT EXISTS strategy_backtest_results (
    id SERIAL PRIMARY KEY,
    strategy_id UUID NOT NULL REFERENCES strategies(id),
    symbol VARCHAR NOT NULL,
    interval VARCHAR NOT NULL,
    start_date TIMESTAMPTZ NOT NULL,
    end_date TIMESTAMPTZ NOT NULL,
    initial_capital NUMERIC NOT NULL,
    final_capital NUMERIC NOT NULL,
    total_trades INTEGER NOT NULL DEFAULT 0,
    winning_trades INTEGER NOT NULL DEFAULT 0,
    losing_trades INTEGER NOT NULL DEFAULT 0,
    win_rate NUMERIC,
    max_drawdown NUMERIC,
    profit_factor NUMERIC,
    sharpe_ratio NUMERIC,
    total_return NUMERIC,
    annualized_return NUMERIC,
    max_consecutive_wins INTEGER,
    max_consecutive_losses INTEGER,
    avg_profit_per_win NUMERIC,
    avg_loss_per_loss NUMERIC,
    avg_win_holding_period NUMERIC, -- in hours
    avg_loss_holding_period NUMERIC, -- in hours
    expectancy NUMERIC,
    parameters_snapshot JSONB, -- Parameters used in this backtest
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Create trades table for detailed backtest trade history
CREATE TABLE IF NOT EXISTS strategy_backtest_trades (
    id SERIAL PRIMARY KEY,
    backtest_id INTEGER NOT NULL REFERENCES strategy_backtest_results(id) ON DELETE CASCADE,
    is_long BOOLEAN NOT NULL,
    entry_price NUMERIC NOT NULL,
    exit_price NUMERIC NOT NULL,
    size_percent NUMERIC NOT NULL,
    entry_time TIMESTAMPTZ NOT NULL,
    exit_time TIMESTAMPTZ NOT NULL,
    exit_reason TEXT NOT NULL,  -- Changed from VARCHAR to TEXT as suggested
    profit_loss_percent NUMERIC NOT NULL,
    profit_loss_amount NUMERIC NOT NULL
);

-- Create index for faster lookup
CREATE INDEX IF NOT EXISTS idx_strategies_enabled ON strategies(enabled);
CREATE INDEX IF NOT EXISTS idx_strategy_backtest_results_strategy_id ON strategy_backtest_results(strategy_id);
CREATE INDEX IF NOT EXISTS idx_strategy_backtest_trades_backtest_id ON strategy_backtest_trades(backtest_id);
CREATE INDEX IF NOT EXISTS idx_strategy_indicators_strategy_id ON strategy_indicators(strategy_id);
CREATE INDEX IF NOT EXISTS idx_strategy_rules_strategy_id ON strategy_rules(strategy_id);

-- For TimescaleDB, convert the trade history to hypertable
-- This is optional, but recommended for large backtests
SELECT create_hypertable('strategy_backtest_trades', 'entry_time', if_not_exists => TRUE);

-- Add a unique index that includes the partitioning column
CREATE UNIQUE INDEX IF NOT EXISTS idx_strategy_backtest_trades_unique 
ON strategy_backtest_trades(backtest_id, entry_time, exit_time);
