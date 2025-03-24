#!/usr/bin/env python3
import psycopg2
import argparse
import logging
import json
from psycopg2.extensions import ISOLATION_LEVEL_AUTOCOMMIT

# Set up logging
logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(levelname)s - %(message)s',
)
logger = logging.getLogger(__name__)

# Database connection parameters
DB_PARAMS = {
    "host": "localhost",
    "port": 5432,
    "database": "binancedb",
    "user": "binanceuser",
    "password": "binancepass"
}

# Commonly used trading pairs
COMMON_PAIRS = [
    "BTCUSDT",
    "ETHUSDT",
    "BNBUSDT",
    "ADAUSDT",
    "XRPUSDT",
    "SOLUSDT",
    "DOGEUSDT",
    "DOTUSDT",
    "MATICUSDT",
    "LINKUSDT"
]

# Commonly used timeframes
COMMON_TIMEFRAMES = [
    "1m",
    "5m",
    "15m",
    "30m",
    "1h",
    "4h",
    "1d",
    "1w"
]

# Commonly used technical indicators with their configurations
TECHNICAL_INDICATORS = [
    # Oscillators
    {
        "type": "oscillator",
        "name": "RSI",
        "parameters": {"period": 14}
    },
    {
        "type": "oscillator",
        "name": "MACD",
        "parameters": {"fast_period": 12, "slow_period": 26, "signal_period": 9}
    },
    {
        "type": "oscillator",
        "name": "CCI",
        "parameters": {"period": 20}
    },
    {
        "type": "oscillator",
        "name": "STOCH",
        "parameters": {"k_period": 14, "d_period": 3, "slowing": 3}
    },
    {
        "type": "oscillator",
        "name": "STOCHRSI",
        "parameters": {"period": 14, "k_period": 3, "d_period": 3}
    },
    {
        "type": "oscillator",
        "name": "MOM",
        "parameters": {"period": 10}
    },
    {
        "type": "oscillator",
        "name": "MFI",
        "parameters": {"period": 14}
    },
    
    # Overlap studies (Moving Averages, etc.)
    {
        "type": "overlap",
        "name": "SMA",
        "parameters": {"period": 20}
    },
    {
        "type": "overlap",
        "name": "EMA",
        "parameters": {"period": 20}
    },
    {
        "type": "overlap",
        "name": "BBANDS",
        "parameters": {"period": 20, "deviation_up": 2, "deviation_down": 2}
    },
    {
        "type": "overlap",
        "name": "TEMA",
        "parameters": {"period": 20}
    },
    {
        "type": "overlap",
        "name": "WMA",
        "parameters": {"period": 20}
    },
    
    # Volatility indicators
    {
        "type": "volatility",
        "name": "ATR",
        "parameters": {"period": 14}
    },
    {
        "type": "volatility",
        "name": "NATR",
        "parameters": {"period": 14}
    },
    
    # Volume indicators
    {
        "type": "volume",
        "name": "OBV",
        "parameters": {}
    },
    {
        "type": "volume",
        "name": "AD",
        "parameters": {}
    },
    
    # Pattern recognition
    {
        "type": "pattern",
        "name": "CDLENGULFING",
        "parameters": {}
    },
    {
        "type": "pattern",
        "name": "CDLHAMMER",
        "parameters": {}
    },
    {
        "type": "pattern",
        "name": "CDLMORNINGSTAR",
        "parameters": {}
    }
]

def create_database_if_not_exists(args):
    """
    Create the database if it doesn't exist
    """
    # Connect to PostgreSQL server without specifying a database
    connection_params = {
        "host": args.host,
        "port": args.port,
        "user": args.user,
        "password": args.password
    }
    
    try:
        # Use a default database like postgres
        conn = psycopg2.connect(**connection_params, database="postgres")
        conn.set_isolation_level(ISOLATION_LEVEL_AUTOCOMMIT)
        cursor = conn.cursor()
        
        # Check if database exists
        cursor.execute(f"SELECT 1 FROM pg_database WHERE datname = '{args.dbname}'")
        exists = cursor.fetchone()
        
        if not exists:
            logger.info(f"Creating database '{args.dbname}'...")
            cursor.execute(f"CREATE DATABASE {args.dbname}")
            logger.info(f"Database '{args.dbname}' created successfully.")
        else:
            logger.info(f"Database '{args.dbname}' already exists.")
        
        cursor.close()
        conn.close()
        return True
    except Exception as e:
        logger.error(f"Error creating database: {e}")
        return False

def setup_extensions(conn):
    """
    Set up required PostgreSQL extensions
    """
    cursor = conn.cursor()
    
    try:
        # Check if TimescaleDB extension is installed
        cursor.execute("SELECT COUNT(*) FROM pg_extension WHERE extname = 'timescaledb'")
        has_timescaledb = cursor.fetchone()[0] > 0
        
        if not has_timescaledb:
            logger.warning("TimescaleDB extension is not installed. Attempting to create it...")
            try:
                cursor.execute("CREATE EXTENSION IF NOT EXISTS timescaledb CASCADE")
                conn.commit()
                logger.info("TimescaleDB extension created successfully.")
            except Exception as e:
                logger.error(f"Failed to create TimescaleDB extension: {e}")
                logger.error("Please install TimescaleDB on your PostgreSQL server.")
                conn.rollback()
        else:
            logger.info("TimescaleDB extension is already installed.")
    except Exception as e:
        logger.error(f"Error checking extensions: {e}")
        conn.rollback()
    finally:
        cursor.close()

def drop_and_recreate_schema(conn):
    """
    Drop all tables and recreate the schema from scratch
    """
    cursor = conn.cursor()
    
    try:
        logger.warning("Dropping all existing tables...")
        cursor.execute("DROP TABLE IF EXISTS calculated_indicators CASCADE")
        cursor.execute("DROP TABLE IF EXISTS indicator_config CASCADE")
        cursor.execute("DROP TABLE IF EXISTS binance_candles CASCADE")
        conn.commit()
        logger.info("All tables dropped successfully.")
        
        # Create binance_candles table
        logger.info("Creating binance_candles table...")
        cursor.execute("""
            CREATE TABLE binance_candles (
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
            )
        """)
        conn.commit()
        
        # Create indicator_config table
        logger.info("Creating indicator_config table...")
        cursor.execute("""
            CREATE TABLE indicator_config (
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
            )
        """)
        conn.commit()
        
        # Create calculated_indicators table
        logger.info("Creating calculated_indicators table...")
        cursor.execute("""
            CREATE TABLE calculated_indicators (
                id SERIAL PRIMARY KEY,
                symbol VARCHAR NOT NULL,
                interval VARCHAR NOT NULL,
                indicator_type VARCHAR NOT NULL,
                indicator_name VARCHAR NOT NULL,
                parameters JSONB NOT NULL,
                time TIMESTAMPTZ NOT NULL,
                value JSONB NOT NULL,
                created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
            )
        """)
        conn.commit()
        
        # Convert to hypertable
        logger.info("Converting to hypertable...")
        cursor.execute("""
            SELECT create_hypertable('calculated_indicators', 'time',
                                    if_not_exists => TRUE)
        """)
        conn.commit()
        
        # Create unique index
        logger.info("Creating unique index...")
        cursor.execute("""
            CREATE UNIQUE INDEX idx_calculated_indicators_unique 
            ON calculated_indicators(symbol, interval, indicator_name, parameters, time)
        """)
        conn.commit()
        
        # Set up compression
        logger.info("Setting up compression...")
        cursor.execute("""
            ALTER TABLE calculated_indicators SET (
                timescaledb.compress,
                timescaledb.compress_segmentby = 'symbol,interval,indicator_name'
            )
        """)
        conn.commit()
        
        # Add compression policy
        logger.info("Adding compression policy...")
        cursor.execute("""
            SELECT add_compression_policy('calculated_indicators', INTERVAL '7 days', if_not_exists => TRUE)
        """)
        conn.commit()
        
        # Create indices
        logger.info("Creating additional indices...")
        cursor.execute("""
            CREATE INDEX idx_binance_candles_symbol_interval
            ON binance_candles(symbol, interval)
        """)
        conn.commit()
        
        cursor.execute("""
            CREATE INDEX idx_calculated_indicators_symbol_interval 
            ON calculated_indicators(symbol, interval)
        """)
        conn.commit()
        
        cursor.execute("""
            CREATE INDEX idx_calculated_indicators_time 
            ON calculated_indicators(time DESC)
        """)
        conn.commit()
        
        logger.info("Schema setup complete!")
    except Exception as e:
        logger.error(f"An error occurred: {e}")
        conn.rollback()
    finally:
        cursor.close()

def create_indicator_configs(conn, pairs=None, timeframes=None, limit=None):
    """
    Create indicator configurations for specified pairs and timeframes
    """
    if pairs is None:
        pairs = COMMON_PAIRS
    
    if timeframes is None:
        timeframes = COMMON_TIMEFRAMES
    
    cursor = conn.cursor()
    
    try:
        # Count how many pairs we'll create configurations for
        total_configs = len(pairs) * len(timeframes) * len(TECHNICAL_INDICATORS)
        
        if limit and total_configs > limit:
            logger.warning(f"Would create {total_configs} configurations, limiting to {limit}")
            
            # Calculate how many pairs to use to stay within limit
            configs_per_pair = len(timeframes) * len(TECHNICAL_INDICATORS)
            pairs_to_use = min(len(pairs), limit // configs_per_pair)
            pairs = pairs[:pairs_to_use]
            
            # Recalculate total configs
            total_configs = len(pairs) * len(timeframes) * len(TECHNICAL_INDICATORS)
        
        logger.info(f"Creating {total_configs} indicator configurations...")
        
        count = 0
        for symbol in pairs:
            for interval in timeframes:
                for indicator in TECHNICAL_INDICATORS:
                    # Convert parameters to JSON string
                    params_json = json.dumps(indicator["parameters"])
                    
                    cursor.execute("""
                        INSERT INTO indicator_config 
                        (symbol, interval, indicator_type, indicator_name, parameters, enabled)
                        VALUES (%s, %s, %s, %s, %s, %s)
                        ON CONFLICT (symbol, interval, indicator_name, parameters) DO NOTHING
                    """, (
                        symbol,
                        interval,
                        indicator["type"],
                        indicator["name"],
                        params_json,
                        True
                    ))
                    
                    count += 1
                    
                    # Commit every 1000 inserts to avoid long transactions
                    if count % 1000 == 0:
                        conn.commit()
                        logger.info(f"Inserted {count}/{total_configs} configurations...")
        
        conn.commit()
        logger.info(f"Successfully created {count} indicator configurations.")
    except Exception as e:
        logger.error(f"An error occurred: {e}")
        conn.rollback()
    finally:
        cursor.close()

def main():
    parser = argparse.ArgumentParser(description="Set up database schema for technical indicator calculator")
    parser.add_argument("--host", default=DB_PARAMS["host"], help="Database host")
    parser.add_argument("--port", default=DB_PARAMS["port"], type=int, help="Database port")
    parser.add_argument("--dbname", default=DB_PARAMS["database"], help="Database name")
    parser.add_argument("--user", default=DB_PARAMS["user"], help="Database user")
    parser.add_argument("--password", default=DB_PARAMS["password"], help="Database password")
    parser.add_argument("--skip-drop", action="store_true", help="Skip dropping and recreating tables")
    parser.add_argument("--btc-only", action="store_true", help="Only create configs for BTC")
    parser.add_argument("--all-pairs", action="store_true", help="Create configs for all common pairs")
    parser.add_argument("--limit", type=int, help="Limit the number of configurations to create")
    
    args = parser.parse_args()
    
    # Update DB params with command line arguments
    connection_params = {
        "host": args.host,
        "port": args.port,
        "database": args.dbname,
        "user": args.user,
        "password": args.password
    }
    
    # First make sure the database exists
    if not create_database_if_not_exists(args):
        logger.error("Failed to ensure database exists. Exiting.")
        return
    
    try:
        # Connect to database
        logger.info(f"Connecting to database at {args.host}:{args.port}/{args.dbname}...")
        conn = psycopg2.connect(**connection_params)
        
        # Set up extensions
        setup_extensions(conn)
        
        # Drop and recreate schema if not skipped
        if not args.skip_drop:
            drop_and_recreate_schema(conn)
        
        # Determine which pairs to use
        if args.btc_only:
            pairs = ["BTCUSDT"]
        elif args.all_pairs:
            pairs = COMMON_PAIRS
        else:
            pairs = ["BTCUSDT"]  # Default to BTC only
        
        # Create indicator configurations
        create_indicator_configs(conn, pairs, COMMON_TIMEFRAMES, args.limit)
        
        logger.info("Database setup completed successfully.")
        
    except Exception as e:
        logger.error(f"Failed to connect to database: {e}")
    finally:
        if 'conn' in locals():
            conn.close()

if __name__ == "__main__":
    main()
