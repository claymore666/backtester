# Technical Indicator Calculator v1.0 Release Notes

## Overview

The Technical Indicator Calculator is a high-performance system designed to calculate and store technical indicators for cryptocurrency market data. Built with Rust and utilizing TimescaleDB for time-series data storage, this application processes market data from Binance and applies various technical analysis algorithms using the TA-Lib library.

## Architecture

The system follows a modular architecture with clearly defined components:

1. **Core Engine** - A Rust-based calculation engine that processes market data and generates technical indicators
2. **Database Layer** - TimescaleDB for efficient time-series data storage with compression capabilities
3. **Cache Layer** - Redis for temporary storage and job coordination
4. **Python Utilities** - Supporting scripts for database setup, data loading, and monitoring

## Database Schema

The system utilizes three primary tables:

### binance_candles
Stores raw market data with the following structure:
```
- id: Primary key, auto-incrementing integer
- symbol: String, indexed, represents trading pair (e.g., "BTCUSDT")
- interval: String, timeframe of candle (e.g., "1m", "1h", "1d")
- open_time: DateTime, when candle period started
- open_price: Float, price at period start
- high_price: Float, highest price in period
- low_price: Float, lowest price in period
- close_price: Float, price at period end
- volume: Float, trading volume in period
- close_time: DateTime, when candle period ended
- quote_asset_volume: Float, volume in quote asset
- number_of_trades: Integer, count of trades in period
```
Unique constraint: (symbol, interval, open_time)

### indicator_config
Stores configuration for enabled technical indicators:
```
- id: Serial PRIMARY KEY
- symbol: VARCHAR NOT NULL
- interval: VARCHAR NOT NULL
- indicator_type: VARCHAR NOT NULL
- indicator_name: VARCHAR NOT NULL
- parameters: JSONB NOT NULL
- enabled: BOOLEAN NOT NULL DEFAULT TRUE
- created_at: TIMESTAMPTZ NOT NULL DEFAULT NOW()
- updated_at: TIMESTAMPTZ NOT NULL DEFAULT NOW()
```
Unique constraint: (symbol, interval, indicator_name, parameters)

### calculated_indicators
Stores calculated indicator values:
```
- id: Serial PRIMARY KEY
- symbol: VARCHAR NOT NULL
- interval: VARCHAR NOT NULL
- indicator_type: VARCHAR NOT NULL
- indicator_name: VARCHAR NOT NULL
- parameters: JSONB NOT NULL
- time: TIMESTAMPTZ NOT NULL
- value: JSONB NOT NULL
- created_at: TIMESTAMPTZ NOT NULL DEFAULT NOW()
```
Unique constraint: (symbol, interval, indicator_name, parameters, time)

This table is configured as a TimescaleDB hypertable with compression enabled to optimize storage for time-series data.

## Components

### Core Rust Application

The Rust application is structured into several modules:

1. **Main Module** (`src/main.rs`)
   - Application entry point, initializes components and starts the worker process

2. **Configuration** (`src/config.rs`)
   - Handles application configuration from environment variables

3. **Database** (`src/database/`)
   - `models.rs`: Defines data structures representing database entities
   - `postgres.rs`: Manages PostgreSQL connections and database operations
   - `schema.rs`: Contains database schema definitions

4. **Cache** (`src/cache/`)
   - `redis.rs`: Manages Redis connections and caching operations

5. **Processor** (`src/processor/`)
   - `job.rs`: Defines job structures for indicator calculations
   - `worker.rs`: Implements the main worker logic for processing calculation jobs

6. **Indicators** (`src/indicators/`)
   - `calculator.rs`: Provides the interface for calculating technical indicators

7. **TA-Lib Bindings** (`src/talib_bindings/`)
   - `ffi.rs`: Foreign Function Interface to TA-Lib C library
   - `common.rs`: Common functionality for TA-Lib operations
   - `oscillators.rs`: Implementations for oscillator indicators (RSI, MACD, etc.)
   - `overlaps.rs`: Implementations for overlay indicators (SMA, EMA, BBANDS, etc.)
   - `patterns.rs`: Implementations for pattern recognition indicators
   - `volatility.rs`: Implementations for volatility indicators (ATR, etc.)
   - `volume.rs`: Implementations for volume indicators (OBV, etc.)

8. **Utilities** (`src/utils/`)
   - `log_utils.rs`: Logging utilities
   - `utils.rs`: General utility functions

### Python Scripts

The system includes several Python scripts for setup and maintenance:

1. **setup_database.py**
   - Creates the database schema and initializes configurations

2. **loader.py**
   - Loads historical market data from Binance API

3. **list_configured_indicators.py**
   - Lists all configured indicators in the system

4. **show_asset_data.py**
   - Displays information about loaded market data

## Supported Technical Indicators

The system supports a wide range of technical indicators categorized as follows:

### Oscillators
- RSI (Relative Strength Index)
- MACD (Moving Average Convergence Divergence)
- CCI (Commodity Channel Index)
- STOCH (Stochastic)
- STOCHRSI (Stochastic RSI)
- MOM (Momentum)
- MFI (Money Flow Index)

### Overlaps
- SMA (Simple Moving Average)
- EMA (Exponential Moving Average)
- BBANDS (Bollinger Bands)
- TEMA (Triple Exponential Moving Average)
- WMA (Weighted Moving Average)

### Volatility
- ATR (Average True Range)
- NATR (Normalized Average True Range)

### Volume
- OBV (On Balance Volume)
- AD (Accumulation/Distribution)

### Pattern Recognition
- CDLENGULFING (Engulfing Pattern)
- CDLHAMMER (Hammer Pattern)
- CDLMORNINGSTAR (Morning Star Pattern)

## Deployment

The application is containerized using Docker and can be deployed using docker-compose. The following services are defined:

1. **database** - TimescaleDB instance for data storage
2. **redis** - Redis instance for caching and job coordination
3. **indicator-calculator** - The main Rust application

## Configuration

Configuration is managed through environment variables, which can be set in the `.env` file:

```
# Database configuration
DB_HOST=localhost
DB_PORT=5432
DB_USER=binanceuser
DB_PASSWORD=binancepass
DB_NAME=binancedb

# Redis configuration
REDIS_URL=redis://localhost:6379

# Application configuration
RUST_LOG=info
CONCURRENCY=4
CACHE_TTL_SECONDS=3600
```

## Development Roadmap

For future development, consider the following areas:

1. **Indicator Extensions**
   - Implement additional technical indicators beyond those provided by TA-Lib
   - Add custom indicators that combine multiple signals

2. **API Layer**
   - Develop a REST API for accessing calculated indicators
   - Implement WebSocket support for real-time updates

3. **Performance Optimization**
   - Explore further parallelization opportunities
   - Implement batch calculation strategies for related indicators

4. **Visualization Tools**
   - Create a web-based dashboard for visualizing indicators
   - Develop reports for technical analysis summaries

5. **Backtesting Framework**
   - Build a framework for testing trading strategies using calculated indicators
   - Implement performance metrics for strategy evaluation

## Technical Requirements

- Rust 1.85.1 or later
- PostgreSQL 13 or later with TimescaleDB extension
- Redis 6 or later
- TA-Lib 0.6.4
- Python 3.8 or later (for utility scripts)

## Conclusion

The Technical Indicator Calculator provides a robust foundation for analyzing cryptocurrency market data. Its modular design and efficient data processing capabilities make it suitable for both real-time analysis and historical backtesting. The combination of Rust's performance with TimescaleDB's time-series optimizations creates a powerful platform for technical analysis in the cryptocurrency domain.
