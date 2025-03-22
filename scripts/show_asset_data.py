import sqlalchemy as sa
from sqlalchemy.orm import declarative_base, Session
import pandas as pd
from tabulate import tabulate
from collections import defaultdict

# Database Configuration
DB_CONNECTION = 'postgresql://binanceuser:binancepass@localhost:5432/binancedb'

# SQLAlchemy Setup
Base = declarative_base()

class BinanceCandle(Base):
    """
    SQLAlchemy model for Binance candle data
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

    # Unique constraint
    __table_args__ = (
        sa.UniqueConstraint('symbol', 'interval', 'open_time'),
    )

def main():
    try:
        # Connect to the database
        engine = sa.create_engine(DB_CONNECTION)
        print("Connecting to database...")
        
        with Session(engine) as session:
            # Get all unique symbols in the database
            print("Fetching unique trading pairs...")
            symbols = session.query(sa.distinct(BinanceCandle.symbol)).all()
            symbols = [symbol[0] for symbol in symbols]
            
            if not symbols:
                print("No data found in the database.")
                return
            
            # Get all unique intervals in the database
            print("Fetching unique candle intervals...")
            intervals = session.query(sa.distinct(BinanceCandle.interval)).all()
            intervals = [interval[0] for interval in intervals]
            
            # Get data summary for each symbol and interval
            print("Generating summary statistics...")
            
            # Dictionary to store interval availability for each symbol
            symbol_interval_map = defaultdict(set)
            
            # Dictionary to store date ranges for each symbol and interval
            date_ranges = {}
            
            # Process each symbol
            for symbol in symbols:
                # Check which intervals are available for this symbol
                for interval in intervals:
                    # Query to check if data exists for this symbol and interval
                    exists = session.query(sa.exists().where(
                        sa.and_(
                            BinanceCandle.symbol == symbol,
                            BinanceCandle.interval == interval
                        )
                    )).scalar()
                    
                    if exists:
                        symbol_interval_map[symbol].add(interval)
                        
                        # Get the date range for this symbol and interval
                        first_candle = session.query(
                            BinanceCandle.open_time
                        ).filter(
                            BinanceCandle.symbol == symbol,
                            BinanceCandle.interval == interval
                        ).order_by(
                            BinanceCandle.open_time.asc()
                        ).first()
                        
                        last_candle = session.query(
                            BinanceCandle.open_time
                        ).filter(
                            BinanceCandle.symbol == symbol,
                            BinanceCandle.interval == interval
                        ).order_by(
                            BinanceCandle.open_time.desc()
                        ).first()
                        
                        if first_candle and last_candle:
                            date_ranges[(symbol, interval)] = (first_candle[0], last_candle[0])
            
            # Count the number of candles for each symbol and interval
            candle_counts = {}
            for symbol in symbols:
                for interval in symbol_interval_map[symbol]:
                    count = session.query(sa.func.count(BinanceCandle.id)).filter(
                        BinanceCandle.symbol == symbol,
                        BinanceCandle.interval == interval
                    ).scalar()
                    
                    candle_counts[(symbol, interval)] = count
            
            # Print summary
            print("\n===== AVAILABLE TRADING PAIRS =====")
            print(f"Total trading pairs: {len(symbols)}")
            
            # Create a table of trading pairs and intervals
            headers = ["Symbol"] + intervals
            table_data = []
            
            for symbol in sorted(symbols):
                row = [symbol]
                for interval in intervals:
                    if interval in symbol_interval_map[symbol]:
                        count = candle_counts.get((symbol, interval), 0)
                        date_range = date_ranges.get((symbol, interval), (None, None))
                        if date_range[0] and date_range[1]:
                            date_info = f"{date_range[0].strftime('%Y-%m-%d')} to {date_range[1].strftime('%Y-%m-%d')}"
                            row.append(f"✓ ({count}, {date_info})")
                        else:
                            row.append(f"✓ ({count})")
                    else:
                        row.append("✗")
                table_data.append(row)
            
            # Print the summary table
            print(tabulate(table_data, headers=headers, tablefmt="grid"))
            
            # Print interval summary
            print("\n===== INTERVAL SUMMARY =====")
            interval_counts = {interval: sum(1 for s in symbols if interval in symbol_interval_map[s]) for interval in intervals}
            for interval, count in sorted(interval_counts.items()):
                print(f"{interval}: Available for {count}/{len(symbols)} trading pairs ({count/len(symbols)*100:.2f}%)")
            
            # Summarize total data points
            total_candles = sum(candle_counts.values())
            print(f"\nTotal candles in database: {total_candles:,}")
            
    except Exception as e:
        print(f"Error: {e}")

if __name__ == "__main__":
    main()
