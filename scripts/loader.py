#!/usr/bin/env python3
import requests
import sys
import time
import argparse
from datetime import datetime, timedelta
import concurrent.futures
import sqlalchemy as sa
from sqlalchemy.orm import declarative_base, Session
from sqlalchemy.dialects.postgresql import insert
from sqlalchemy.exc import OperationalError, ProgrammingError
from pick import pick
import math
from tqdm import tqdm
import logging
import threading
from collections import deque

# Set up argument parser
parser = argparse.ArgumentParser(description='Binance historical data loader')
parser.add_argument('--asset', help='Asset symbol to load (e.g., BTCUSDT)')
parser.add_argument('--debug', action='store_true', help='Enable debug logging')
parser.add_argument('--interval', help='Specific interval to load (e.g., 1d, 1h, 15m)')
args = parser.parse_args()

# Configure logging level based on debug flag
log_level = logging.DEBUG if args.debug else logging.INFO

# Set up logging
logging.basicConfig(
    level=log_level,
    format='%(asctime)s - %(levelname)s - %(message)s',
    handlers=[
        logging.FileHandler("binance_loader.log"),
        logging.StreamHandler()
    ]
)
logger = logging.getLogger(__name__)

# Configuration Parameters
API_CONFIG = {
    # Binance API Limits
    'WEIGHT_PER_MINUTE': 6000,        # Maximum request weight per minute
    'MAX_ORDERS_PER_10SEC': 100,      # Maximum orders per 10 seconds
    'MAX_ORDERS_PER_24H': 200000,     # Maximum orders per 24 hours
    'SAFETY_MARGIN': 0.8,             # Safety margin (80% of max)
    
    # Candle Interval Options
    'CANDLE_INTERVALS': {
        '1m': '1 minute',
        '3m': '3 minutes',
        '5m': '5 minutes',
        '15m': '15 minutes',
        '30m': '30 minutes',
        '1h': '1 hour',
        '2h': '2 hours',
        '4h': '4 hours',
        '6h': '6 hours',
        '8h': '8 hours',
        '12h': '12 hours',
        '1d': '1 day',
        '3d': '3 days',
        '1w': '1 week',
        '1M': '1 month'
    }
}

# Get only the specified interval if provided
if args.interval and args.interval in API_CONFIG['CANDLE_INTERVALS']:
    selected_intervals = {args.interval: API_CONFIG['CANDLE_INTERVALS'][args.interval]}
    logger.info(f"Loading only the {args.interval} interval")
    API_CONFIG['CANDLE_INTERVALS'] = selected_intervals

# Database Configuration
DB_CONNECTION = 'postgresql://binanceuser:binancepass@localhost:5432/binancedb'

# SQLAlchemy Setup
Base = declarative_base()

class BinanceCandle(Base):
    """
    SQLAlchemy model for storing Binance candle data
    """
    __tablename__ = 'binance_candles'

    id = sa.Column(sa.Integer, primary_key=True, autoincrement=True)
    symbol = sa.Column(sa.String, nullable=False, index=True)
    interval = sa.Column(sa.String, nullable=False)
    open_time = sa.Column(sa.DateTime, nullable=False)
    open_price = sa.Column(sa.Float)
    high_price = sa.Column(sa.Float)
    low_price = sa.Column(sa.Float)
    close_price = sa.Column(sa.Float)
    volume = sa.Column(sa.Float)
    close_time = sa.Column(sa.DateTime)
    quote_asset_volume = sa.Column(sa.Float)
    number_of_trades = sa.Column(sa.Integer)

    # Unique constraint to prevent duplicates
    __table_args__ = (
        sa.UniqueConstraint('symbol', 'interval', 'open_time'),
    )

class WeightBasedRateLimiter:
    """
    Rate limiter that respects Binance's weight-based rate limiting system
    """
    def __init__(self, max_weight_per_minute, safety_margin=0.8):
        self.max_weight = int(max_weight_per_minute * safety_margin)
        self.lock = threading.Lock()
        self.request_times = deque()  # Timestamps of requests
        self.request_weights = deque()  # Weights of requests
        self.total_weight_used = 0  # Total weight used since start
        logger.info(f"Rate limiter initialized with max weight of {self.max_weight} per minute")

    def wait_if_needed(self, weight):
        """
        Wait if necessary to respect the rate limit before making a request
        Returns the waiting time in seconds, or 0 if no wait was needed
        """
        with self.lock:
            current_time = time.time()
            minute_ago = current_time - 60
            
            # Remove entries older than 1 minute
            while self.request_times and self.request_times[0] < minute_ago:
                self.request_times.popleft()
                removed_weight = self.request_weights.popleft()
            
            # Calculate current weight sum in the last minute
            current_weight_sum = sum(self.request_weights)
            
            # Check if adding this request would exceed the limit
            if current_weight_sum + weight > self.max_weight:
                # Need to wait for some older requests to expire
                if not self.request_times:
                    # This shouldn't happen normally, but just in case
                    wait_time = 0
                else:
                    # Wait until enough weight is freed up
                    wait_needed = 60 - (current_time - self.request_times[0])
                    wait_time = max(0, wait_needed)
                
                if wait_time > 0:
                    logger.debug(f"Rate limit approaching. Waiting {wait_time:.2f}s. Current weight: {current_weight_sum}, Adding: {weight}")
                    time.sleep(wait_time)
            else:
                wait_time = 0
            
            # Record this request
            self.request_times.append(time.time())
            self.request_weights.append(weight)
            self.total_weight_used += weight
            
            return wait_time

# Create global rate limiter instance
rate_limiter = WeightBasedRateLimiter(
    max_weight_per_minute=API_CONFIG['WEIGHT_PER_MINUTE'],
    safety_margin=API_CONFIG['SAFETY_MARGIN']
)

def calculate_request_weight(interval, limit):
    """
    Calculate the weight of a klines request based on the number of candles
    
    Weight rules for klines:
    - Default weight: 1
    - 100-499 candles: weight = 2
    - 500-999 candles: weight = 5
    - 1000-1499 candles: weight = 10
    - 1500-1999 candles: weight = 20
    - 2000-2999 candles: weight = 50
    - 3000-5000 candles: weight = 100
    """
    if limit <= 100:
        return 1
    elif limit <= 499:
        return 2
    elif limit <= 999:
        return 5
    elif limit <= 1499:
        return 10
    elif limit <= 1999:
        return 20
    elif limit <= 2999:
        return 50
    else:  # 3000-5000
        return 100

def create_database_tables():
    """
    Ensure database tables are created
    """
    try:
        engine = sa.create_engine(DB_CONNECTION)
        logger.debug(f"Connecting to database with connection string: {DB_CONNECTION}")
        Base.metadata.create_all(engine)
        logger.info("Database tables created successfully.")
        
        # Check if we can actually connect and query
        with engine.connect() as conn:
            result = conn.execute(sa.text("SELECT 1"))
            if result:
                logger.debug("Successfully executed test query on database.")
            
    except (OperationalError, ProgrammingError) as e:
        logger.error(f"Error creating database tables: {e}")
        logger.error("Please ensure:")
        logger.error("1. The database exists")
        logger.error("2. The user has sufficient permissions")
        logger.error("3. The connection details are correct")
        sys.exit(1)

def get_asset_description(asset):
    """
    Predefined descriptions for common assets
    """
    descriptions = {
        'BTC': 'Bitcoin - The first and most famous decentralized cryptocurrency',
        'ETH': 'Ethereum - Blockchain platform for smart contracts and decentralized applications',
        'BNB': 'Binance Coin - Native cryptocurrency of the Binance ecosystem',
        'USDT': 'Tether - Stablecoin pegged to the US Dollar',
        'DEFAULT': 'No specific description available'
    }
    return descriptions.get(asset, descriptions['DEFAULT'])

def get_date_range_for_symbol_interval(symbol, interval):
    """
    Get the date range of existing data for a symbol and interval
    Returns (min_date, max_date) or (None, None) if no data
    """
    try:
        engine = sa.create_engine(DB_CONNECTION)
        with Session(engine) as session:
            result = session.query(
                sa.func.min(BinanceCandle.open_time),
                sa.func.max(BinanceCandle.open_time)
            ).filter(
                BinanceCandle.symbol == symbol,
                BinanceCandle.interval == interval
            ).first()
            
            min_date, max_date = result
            
            if min_date and max_date:
                logger.info(f"Existing data for {symbol} ({interval}): {min_date} to {max_date}")
                return min_date, max_date
            
            logger.debug(f"No existing data found for {symbol} ({interval})")
            return None, None
    except Exception as e:
        logger.error(f"Error getting date range: {e}")
        return None, None

def check_asset_loaded_intervals(symbol):
    """
    Check which intervals are already loaded for a given symbol
    """
    try:
        engine = sa.create_engine(DB_CONNECTION)
        with Session(engine) as session:
            loaded_intervals = session.query(
                sa.distinct(BinanceCandle.interval)
            ).filter(
                BinanceCandle.symbol == symbol
            ).all()
        
        result = [interval[0] for interval in loaded_intervals]
        logger.info(f"Loaded intervals for {symbol}: {result}")
        
        # Debug: verify content in the database
        if args.debug:
            with Session(engine) as session:
                for interval in result:
                    count = session.query(sa.func.count(BinanceCandle.id)).filter(
                        BinanceCandle.symbol == symbol,
                        BinanceCandle.interval == interval
                    ).scalar()
                    logger.debug(f"Found {count} rows for {symbol} {interval}")
        
        return result
    except Exception as e:
        logger.error(f"Error checking loaded intervals: {e}")
        return []

def fetch_binance_assets():
    """
    Fetch all available trading symbols from Binance
    """
    try:
        logger.info("Fetching available trading pairs from Binance...")
        
        # Weight for this endpoint is 10
        wait_time = rate_limiter.wait_if_needed(10)
        if wait_time > 0:
            logger.debug(f"Waited {wait_time:.2f}s for rate limiting before fetching assets")
            
        response = requests.get("https://api.binance.com/api/v3/exchangeInfo")
        response.raise_for_status()
        exchange_info = response.json()
        
        assets = [
            {
                'symbol': symbol['symbol'], 
                'baseAsset': symbol['baseAsset'], 
                'quoteAsset': symbol['quoteAsset']
            } 
            for symbol in exchange_info.get('symbols', [])
        ]
        
        logger.info(f"Found {len(assets)} trading pairs on Binance")
        
        # Debug: Show all found assets that match our target
        if args.debug and args.asset:
            matching_assets = [asset for asset in assets if asset['symbol'] == args.asset]
            if matching_assets:
                logger.debug(f"Found requested asset {args.asset} in available assets")
            else:
                logger.debug(f"Could not find requested asset {args.asset} in available assets")
                logger.debug(f"Available assets starting with same prefix: {[a['symbol'] for a in assets if a['symbol'].startswith(args.asset[:3])][:10]}")
        
        return assets
    except requests.RequestException as e:
        logger.error(f"Error fetching Binance assets: {e}")
        return []

def fetch_earliest_tradable_date(symbol, interval):
    """
    Fetch the earliest available candle data for a symbol
    """
    try:
        logger.info(f"Fetching earliest tradable date for {symbol} ({interval})...")
        
        # Weight for klines with limit=1 is 1
        wait_time = rate_limiter.wait_if_needed(1)
        if wait_time > 0:
            logger.debug(f"Waited {wait_time:.2f}s for rate limiting")
            
        # First try with startTime=0 to get the earliest possible candle
        params = {
            'symbol': symbol,
            'interval': interval,
            'startTime': 0,  # Start from UNIX epoch
            'limit': 1
        }
        
        logger.debug(f"Requesting earliest candle with params: {params}")
        response = requests.get("https://api.binance.com/api/v3/klines", params=params)
        response.raise_for_status()
        first_candle = response.json()
        
        if not first_candle:
            logger.warning(f"No candle data found for {symbol} with interval {interval}")
            return None
        
        # Convert timestamp to datetime (Binance provides millisecond timestamps)
        first_candle_time = datetime.fromtimestamp(int(first_candle[0][0]) / 1000)
        logger.info(f"Earliest tradable date for {symbol} ({interval}): {first_candle_time}")
        return first_candle_time
    except Exception as e:
        logger.error(f"Error fetching earliest date for {symbol}: {e}")
        return None

def fetch_candles_batch(symbol, interval, start_time, end_time, max_limit=1000):
    """
    Fetch a batch of candles for a specific time range
    """
    try:
        limit = min(max_limit, 1000)  # Binance max limit is 1000
        
        params = {
            'symbol': symbol,
            'interval': interval,
            'startTime': int(start_time.timestamp() * 1000),
            'endTime': int(end_time.timestamp() * 1000),
            'limit': limit
        }
        
        # Calculate weight based on limit
        weight = calculate_request_weight(interval, limit)
        
        # Wait if needed for rate limiting
        wait_time = rate_limiter.wait_if_needed(weight)
        if wait_time > 0:
            logger.debug(f"Waited {wait_time:.2f}s for rate limiting. Request weight: {weight}")
        
        logger.debug(f"Fetching candles for {symbol} ({interval}) from {start_time} to {end_time}, weight: {weight}")
        response = requests.get("https://api.binance.com/api/v3/klines", params=params)
        response.raise_for_status()
        
        candles_json = response.json()
        logger.debug(f"Received {len(candles_json)} candles")
        
        candles = [
            {
                'symbol': symbol,
                'interval': interval,
                'open_time': datetime.fromtimestamp(candle[0] / 1000),
                'open_price': float(candle[1]),
                'high_price': float(candle[2]),
                'low_price': float(candle[3]),
                'close_price': float(candle[4]),
                'volume': float(candle[5]),
                'close_time': datetime.fromtimestamp(candle[6] / 1000),
                'quote_asset_volume': float(candle[7]),
                'number_of_trades': int(candle[8])
            }
            for candle in candles_json
        ]
        
        return candles
    except Exception as e:
        logger.error(f"Error fetching candles: {e}")
        return []

def calculate_batch_size_for_interval(interval):
    """
    Calculate appropriate batch size for different intervals to avoid timeouts and optimize API usage
    """
    # Mapping of interval to appropriate minute range
    interval_to_minutes = {
        '1m': 1000,   # 1000 minutes (16.6 hours) per batch
        '3m': 3000,   # 3000 minutes (50 hours) per batch
        '5m': 5000,   # 5000 minutes (83.3 hours) per batch
        '15m': 15000, # 15000 minutes (10.4 days) per batch
        '30m': 30000, # 30000 minutes (20.8 days) per batch
        '1h': 60000,  # 60000 minutes (41.6 days) per batch
        '2h': 120000, # 120000 minutes (83.3 days) per batch
        '4h': 240000, # 240000 minutes (166.6 days) per batch
        '6h': 360000, # 360000 minutes (250 days) per batch
        '8h': 480000, # 480000 minutes (333.3 days) per batch
        '12h': 720000, # 720000 minutes (500 days) per batch
        '1d': 1440000, # 1440000 minutes (1000 days) per batch
        '3d': 4320000, # 4320000 minutes (3000 days) per batch
        '1w': 10080000, # 10080000 minutes (7000 days) per batch
        '1M': 43200000, # 43200000 minutes (30000 days) per batch
    }
    
    return interval_to_minutes.get(interval, 1000)  # Default to 1000 minutes

def save_candles_to_db(candles):
    """
    Save candles to the database using bulk insert
    """
    if not candles:
        return 0
    
    engine = sa.create_engine(DB_CONNECTION)
    
    with Session(engine) as session:
        try:
            # Use insert with on conflict do nothing to handle duplicates
            stmt = insert(BinanceCandle).values(candles)
            stmt = stmt.on_conflict_do_nothing(
                index_elements=['symbol', 'interval', 'open_time']
            )
            
            # In debug mode, show the SQL (truncated for readability)
            if args.debug:
                sql_str = str(stmt.compile(compile_kwargs={"literal_binds": True}))
                logger.debug(f"Insert SQL (truncated): {sql_str[:200]}...")
            
            result = session.execute(stmt)
            session.commit()
            
            # Log some information about what was actually inserted
            if args.debug:
                symbol = candles[0]['symbol'] if candles else 'unknown'
                interval = candles[0]['interval'] if candles else 'unknown'
                min_time = min(c['open_time'] for c in candles) if candles else None
                max_time = max(c['open_time'] for c in candles) if candles else None
                logger.debug(f"Saved {len(candles)} candles for {symbol} {interval} from {min_time} to {max_time}")
            
            # Return approximate number of rows inserted
            return len(candles)
        except Exception as e:
            logger.error(f"Database insertion error: {e}")
            logger.error(f"Error details: {str(e)}")
            session.rollback()
            return 0

def load_historical_candles(symbol):
    """
    Load historical candles for a given symbol across all intervals
    """
    # Fetch earliest available dates for each interval
    logger.info(f"Loading historical candles for {symbol}")
    
    # Check which intervals are already loaded
    loaded_intervals = check_asset_loaded_intervals(symbol)
    logger.info(f"Already loaded intervals: {', '.join(loaded_intervals) if loaded_intervals else 'None'}")
    
    total_candles_loaded = 0
    
    # Iterate through all intervals
    for interval in API_CONFIG['CANDLE_INTERVALS']:
        logger.info(f"Processing interval: {interval}")
        
        # Determine start and end dates
        min_date, max_date = get_date_range_for_symbol_interval(symbol, interval)
        
        # Get earliest available date from Binance
        earliest_available_date = fetch_earliest_tradable_date(symbol, interval)
        if not earliest_available_date:
            logger.warning(f"No data available for {symbol} with interval {interval}")
            continue
        
        # Determine start date based on existing data
        if max_date:
            # If we have existing data, start from the last data point + 1 candle period
            start_date = max_date + timedelta(seconds=1)
            logger.info(f"Continuing from existing data. Start date: {start_date}")
        else:
            # If no existing data, start from the earliest available
            start_date = earliest_available_date
            logger.info(f"No existing data. Starting from earliest available: {start_date}")
        
        # If we also want to fill gaps in the data (optional)
        if min_date and min_date > earliest_available_date:
            # We have a gap at the beginning - load that too
            logger.info(f"Detected gap at beginning of data. Loading from {earliest_available_date} to {min_date}")
            interval_candles = load_candle_range(symbol, interval, earliest_available_date, min_date)
            total_candles_loaded += interval_candles
        
        # Calculate end date (now)
        end_date = datetime.utcnow()
        
        # Skip if start date is in the future or after end date
        if start_date >= end_date:
            logger.info(f"Start date ({start_date}) is not before end date ({end_date}). Skipping.")
            continue
        
        # Load the main data range
        interval_candles = load_candle_range(symbol, interval, start_date, end_date)
        total_candles_loaded += interval_candles
        
    logger.info(f"Total candles loaded for {symbol}: {total_candles_loaded}")
    return total_candles_loaded

def load_candle_range(symbol, interval, start_date, end_date, max_concurrent=5):
    """
    Load candles for a specific date range
    """
    # Calculate appropriate batch size based on interval
    batch_minutes = calculate_batch_size_for_interval(interval)
    
    # Calculate total batches
    time_delta = end_date - start_date
    total_minutes = time_delta.total_seconds() / 60
    total_batches = math.ceil(total_minutes / batch_minutes)
    
    logger.info(f"Loading {symbol} - {interval} candles from {start_date} to {end_date}")
    logger.info(f"Using batch size of {batch_minutes} minutes, total batches: {total_batches}")
    
    candles_loaded = 0
    
    # Use a thread pool to handle concurrent requests, but limit concurrency
    # to avoid exceeding rate limits too quickly
    max_concurrent = min(max_concurrent, total_batches)
    
    # Progress tracking
    with tqdm(total=total_batches, desc=f"Loading {symbol} - {interval} Candles", unit="batch") as pbar:
        with concurrent.futures.ThreadPoolExecutor(max_workers=max_concurrent) as executor:
            futures = []
            current_time = start_date
            
            while current_time < end_date:
                # Calculate batch end time
                batch_end_time = min(
                    current_time + timedelta(minutes=batch_minutes),
                    end_date
                )
                
                # Submit batch fetch task
                future = executor.submit(
                    fetch_candles_batch, 
                    symbol, 
                    interval, 
                    current_time, 
                    batch_end_time
                )
                futures.append(future)
                
                # Move to next batch
                current_time = batch_end_time
            
            # Process results as they complete
            for future in concurrent.futures.as_completed(futures):
                try:
                    candles = future.result()
                    inserted = save_candles_to_db(candles)
                    candles_loaded += inserted
                    pbar.update(1)
                    pbar.set_postfix({
                        "candles": candles_loaded, 
                        "weight": rate_limiter.total_weight_used
                    })
                except Exception as e:
                    logger.error(f"Error processing batch: {e}")
    
    logger.info(f"Completed loading {interval} interval for {symbol}. Total candles: {candles_loaded}")
    return candles_loaded

def verify_loaded_data(symbol=None):
    """Verify what data was actually loaded"""
    try:
        engine = sa.create_engine(DB_CONNECTION)
        with Session(engine) as session:
            # Get count of all records
            total_query = session.query(sa.func.count(BinanceCandle.id))
            if symbol:
                total_query = total_query.filter(BinanceCandle.symbol == symbol)
                
            total_count = total_query.scalar()
            logger.info(f"Total records in database: {total_count}")
            
            # Get counts by symbol
            symbol_query = session.query(
                BinanceCandle.symbol, 
                sa.func.count(BinanceCandle.id)
            ).group_by(BinanceCandle.symbol)
            
            if symbol:
                symbol_query = symbol_query.filter(BinanceCandle.symbol == symbol)
                
            symbol_counts = symbol_query.all()
            
            logger.info("Records by symbol:")
            for sym, count in symbol_counts:
                logger.info(f"  {sym}: {count}")
                
                # Get counts by interval for this symbol
                interval_counts = session.query(
                    BinanceCandle.interval, 
                    sa.func.count(BinanceCandle.id)
                ).filter(
                    BinanceCandle.symbol == sym
                ).group_by(BinanceCandle.interval).all()
                
                logger.info(f"  Intervals for {sym}:")
                for intv, count in interval_counts:
                    logger.info(f"    {intv}: {count}")
                    
                    # For debugging, show a sample record
                    if args.debug:
                        sample = session.query(BinanceCandle).filter(
                            BinanceCandle.symbol == sym,
                            BinanceCandle.interval == intv
                        ).first()
                        
                        if sample:
                            logger.debug(f"    Sample record: id={sample.id}, "
                                       f"time={sample.open_time}, "
                                       f"open={sample.open_price}, "
                                       f"close={sample.close_price}")
    except Exception as e:
        logger.error(f"Error verifying loaded data: {e}")

def main():
    try:
        logger.info("Starting Binance data loader with weight-based rate limiting")
        logger.info(f"Using max weight of {API_CONFIG['WEIGHT_PER_MINUTE'] * API_CONFIG['SAFETY_MARGIN']:.0f} per minute")
        
        # Create database tables first
        create_database_tables()
        
        # If asset is provided directly as argument, use it
        if args.asset:
            logger.info(f"Asset symbol provided as argument: {args.asset}")
            # Fetch assets for validation
            assets = fetch_binance_assets()
            
            # Find matching asset
            matching_assets = [asset for asset in assets if asset['symbol'] == args.asset]
            
            if matching_assets:
                selected_asset = matching_assets[0]
                
                # Print selected asset details
                logger.info("\nSelected Asset:")
                logger.info(f"Symbol: {selected_asset['symbol']}")
                logger.info(f"Base Asset: {selected_asset['baseAsset']}")
                logger.info(f"Description: {get_asset_description(selected_asset['baseAsset'])}")
                
                # Load historical candles for all intervals
                load_historical_candles(selected_asset['symbol'])
            else:
                logger.error(f"Symbol {args.asset} not found among available trading pairs.")
                available_symbols = [a['symbol'] for a in assets if args.asset.lower() in a['symbol'].lower()][:10]
                if available_symbols:
                    logger.info(f"Did you mean one of these? {', '.join(available_symbols)}")
                sys.exit(1)
        else:
            # Fetch assets for interactive selection
            assets = fetch_binance_assets()
            
            if not assets:
                logger.error("No assets found.")
                sys.exit(1)
            
            # Create list of display strings for selection
            asset_options = [
                f"{asset['symbol']} - {asset['baseAsset']}/{asset['quoteAsset']} | {get_asset_description(asset['baseAsset'])}" 
                for asset in assets
            ]
            
            # Create interactive pick menu
            title = "Select Binance Trading Pairs (use space to select, enter to confirm):"
            selected_options = pick(asset_options, title, multiselect=True)
            
            # Process selected assets
            for option, index in selected_options:
                # Selected asset details
                selected_asset = assets[index]
                symbol = selected_asset['symbol']
                base_asset = selected_asset['baseAsset']
                
                # Print selected asset details
                logger.info("\nSelected Asset:")
                logger.info(f"Symbol: {symbol}")
                logger.info(f"Base Asset: {base_asset}")
                logger.info(f"Description: {get_asset_description(base_asset)}")
                
                # Load historical candles for all intervals
                load_historical_candles(symbol)
        
        # Verify the data was loaded correctly
        logger.info("\nVerifying loaded data:")
        verify_loaded_data(args.asset)
        
        logger.info("\nCandle data loading complete!")
        logger.info(f"Total weight used: {rate_limiter.total_weight_used}")
    
    except KeyboardInterrupt:
        logger.warning("\nOperation cancelled.")
        sys.exit(0)
    except Exception as e:
        logger.error(f"An unexpected error occurred: {e}")
        sys.exit(1)

if __name__ == "__main__":
    main()
