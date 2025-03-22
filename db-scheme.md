Table: binance_candles
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

Unique constraint: (symbol, interval, open_time)
Database: PostgreSQL
