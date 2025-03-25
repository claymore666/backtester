# Database Schema Documentation

## Core Tables

### binance_candles
Stores raw price data from Binance.

| Column | Type | Description |
|--------|------|-------------|
| id | SERIAL PRIMARY KEY | Unique identifier |
| symbol | VARCHAR NOT NULL | Trading pair (e.g., "BTCUSDT") |
| interval | VARCHAR NOT NULL | Timeframe (e.g., "1m", "1h", "1d") |
| open_time | TIMESTAMPTZ NOT NULL | Candle period start time |
| open_price | DOUBLE PRECISION | Price at period start |
| high_price | DOUBLE PRECISION | Highest price in period |
| low_price | DOUBLE PRECISION | Lowest price in period |
| close_price | DOUBLE PRECISION | Price at period end |
| volume | DOUBLE PRECISION | Trading volume in period |
| close_time | TIMESTAMPTZ | Candle period end time |
| quote_asset_volume | DOUBLE PRECISION | Volume in quote asset |
| number_of_trades | INTEGER | Count of trades in period |

**Indexes:**
- PRIMARY KEY on `id`
- Index on `symbol`
- Unique constraint on `(symbol, interval, open_time)`
- Combined index on `(symbol, interval)`

### indicator_config
Stores configuration for technical indicators to be calculated.

| Column | Type | Description |
|--------|------|-------------|
| id | SERIAL PRIMARY KEY | Unique identifier |
| symbol | VARCHAR NOT NULL | Trading pair (e.g., "BTCUSDT") |
| interval | VARCHAR NOT NULL | Timeframe (e.g., "1m", "1h", "1d") |
| indicator_type | VARCHAR NOT NULL | Category of indicator (e.g., "oscillator", "overlap") |
| indicator_name | VARCHAR NOT NULL | Name of indicator (e.g., "RSI", "MACD") |
| parameters | JSONB NOT NULL | Configuration parameters as JSON |
| enabled | BOOLEAN NOT NULL | Whether this indicator is active |
| created_at | TIMESTAMPTZ NOT NULL | Creation timestamp |
| updated_at | TIMESTAMPTZ NOT NULL | Last update timestamp |

**Indexes:**
- PRIMARY KEY on `id`
- Unique constraint on `(symbol, interval, indicator_name, parameters)`

### calculated_indicators
Stores calculated technical indicator values - implemented as a TimescaleDB hypertable.

| Column | Type | Description |
|--------|------|-------------|
| id | SERIAL PRIMARY KEY | Unique identifier |
| symbol | VARCHAR NOT NULL | Trading pair (e.g., "BTCUSDT") |
| interval | VARCHAR NOT NULL | Timeframe (e.g., "1m", "1h", "1d") |
| indicator_type | VARCHAR NOT NULL | Category of indicator (e.g., "oscillator", "overlap") |
| indicator_name | VARCHAR NOT NULL | Name of indicator (e.g., "RSI", "MACD") |
| parameters | JSONB NOT NULL | Configuration parameters as JSON |
| time | TIMESTAMPTZ NOT NULL | Timestamp for this indicator value |
| value | JSONB NOT NULL | Calculated value(s) as JSON |
| created_at | TIMESTAMPTZ NOT NULL | Creation timestamp |

**Indexes and TimescaleDB Configuration:**
- PRIMARY KEY on `id`
- Unique index on `(symbol, interval, indicator_name, parameters, time)`
- Index on `(symbol, interval)`
- Index on `time DESC`
- Hypertable partition key: `time`
- Compression enabled with segmentby: `symbol, interval, indicator_name`
- Compression policy: After 7 days

## Database Features

### TimescaleDB Optimizations

The system leverages TimescaleDB for time-series optimizations:

1. **Hypertable for calculated_indicators**: Partitions data by time automatically
2. **Compression**: Automatically compresses older data (after 7 days)
3. **Segment-by compression**: Improves compression by grouping similar data (same symbol, interval, indicator)

### Caching System

The application implements two layers of caching:

1. **Redis Cache**:
   - Prevents duplicate processing of the same indicators
   - Stores job status with TTL of 600 seconds
   
2. **Completeness Cache**:
   - In-memory cache of indicator completion status
   - Prevents recalculation of completed indicators
   - Refreshed every 30 minutes (configurable)
   - Tracks coverage percentage and completeness status

## Data Relationships

- Each **calculated_indicators** record relates to a specific configuration in **indicator_config**
- Each **calculated_indicators** record relates to specific candle data in **binance_candles**
- The completeness cache tracks the relationship between candle data range and calculated indicators

This schema is designed for high-performance technical analysis with optimizations for time-series data, efficient querying, and automatic maintenance through TimescaleDB's features.
