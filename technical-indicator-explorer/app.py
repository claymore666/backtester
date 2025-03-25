from flask import Flask, render_template, request, jsonify, abort
from flask_sqlalchemy import SQLAlchemy
from sqlalchemy import distinct, func, desc
import os
from datetime import datetime
import json

app = Flask(__name__)

# Database configuration
DB_HOST = os.environ.get('DB_HOST', 'localhost')
DB_PORT = os.environ.get('DB_PORT', '5432')
DB_USER = os.environ.get('DB_USER', 'binanceuser')
DB_PASSWORD = os.environ.get('DB_PASSWORD', 'binancepass')
DB_NAME = os.environ.get('DB_NAME', 'binancedb')

app.config['SQLALCHEMY_DATABASE_URI'] = f'postgresql://{DB_USER}:{DB_PASSWORD}@{DB_HOST}:{DB_PORT}/{DB_NAME}'
app.config['SQLALCHEMY_TRACK_MODIFICATIONS'] = False

db = SQLAlchemy(app)

# Models
class BinanceCandle(db.Model):
    __tablename__ = 'binance_candles'
    
    id = db.Column(db.Integer, primary_key=True)
    symbol = db.Column(db.String, nullable=False, index=True)
    interval = db.Column(db.String, nullable=False)
    open_time = db.Column(db.DateTime, nullable=False)
    open_price = db.Column(db.Float)
    high_price = db.Column(db.Float)
    low_price = db.Column(db.Float)
    close_price = db.Column(db.Float)
    volume = db.Column(db.Float)
    close_time = db.Column(db.DateTime)
    quote_asset_volume = db.Column(db.Float)
    number_of_trades = db.Column(db.Integer)
    
    # Unique constraint is handled at the database level

class IndicatorConfig(db.Model):
    __tablename__ = 'indicator_config'
    
    id = db.Column(db.Integer, primary_key=True)
    symbol = db.Column(db.String, nullable=False)
    interval = db.Column(db.String, nullable=False)
    indicator_type = db.Column(db.String, nullable=False)
    indicator_name = db.Column(db.String, nullable=False)
    parameters = db.Column(db.JSON, nullable=False)
    enabled = db.Column(db.Boolean, default=True)
    created_at = db.Column(db.DateTime, default=datetime.utcnow)
    updated_at = db.Column(db.DateTime, default=datetime.utcnow, onupdate=datetime.utcnow)

class CalculatedIndicator(db.Model):
    __tablename__ = 'calculated_indicators'
    
    id = db.Column(db.Integer, primary_key=True)
    symbol = db.Column(db.String, nullable=False)
    interval = db.Column(db.String, nullable=False)
    indicator_type = db.Column(db.String, nullable=False)
    indicator_name = db.Column(db.String, nullable=False)
    parameters = db.Column(db.JSON, nullable=False)
    time = db.Column(db.DateTime, nullable=False)
    value = db.Column(db.JSON, nullable=False)
    created_at = db.Column(db.DateTime, default=datetime.utcnow)

# Routes
@app.route('/')
def index():
    return render_template('index.html')

@app.route('/api/assets')
def get_assets():
    """Get list of available assets with their intervals"""
    assets = db.session.query(
        BinanceCandle.symbol,
        func.array_agg(distinct(BinanceCandle.interval)).label('intervals'),
        func.min(BinanceCandle.open_time).label('first_candle'),
        func.max(BinanceCandle.open_time).label('last_candle'),
        func.count(BinanceCandle.id).label('candle_count')
    ).group_by(
        BinanceCandle.symbol
    ).all()
    
    result = []
    for asset in assets:
        result.append({
            'symbol': asset.symbol,
            'intervals': asset.intervals,
            'first_candle': asset.first_candle.isoformat() if asset.first_candle else None,
            'last_candle': asset.last_candle.isoformat() if asset.last_candle else None,
            'candle_count': asset.candle_count
        })
    
    return jsonify(result)

@app.route('/api/asset/<symbol>')
def get_asset_details(symbol):
    """Get detailed information about a specific asset"""
    # Get intervals for this asset
    intervals = db.session.query(
        BinanceCandle.interval,
        func.min(BinanceCandle.open_time).label('first_candle'),
        func.max(BinanceCandle.open_time).label('last_candle'),
        func.count(BinanceCandle.id).label('candle_count')
    ).filter(
        BinanceCandle.symbol == symbol
    ).group_by(
        BinanceCandle.interval
    ).all()
    
    if not intervals:
        abort(404, description=f"No data found for symbol {symbol}")
    
    intervals_data = []
    for interval in intervals:
        intervals_data.append({
            'interval': interval.interval,
            'first_candle': interval.first_candle.isoformat() if interval.first_candle else None,
            'last_candle': interval.last_candle.isoformat() if interval.last_candle else None,
            'candle_count': interval.candle_count
        })
    
    # Get configured indicators for this asset
    indicators = db.session.query(
        IndicatorConfig.indicator_type,
        IndicatorConfig.indicator_name,
        IndicatorConfig.interval,
        IndicatorConfig.parameters
    ).filter(
        IndicatorConfig.symbol == symbol,
        IndicatorConfig.enabled == True
    ).all()
    
    indicators_data = []
    for indicator in indicators:
        indicators_data.append({
            'type': indicator.indicator_type,
            'name': indicator.indicator_name,
            'interval': indicator.interval,
            'parameters': indicator.parameters
        })
    
    result = {
        'symbol': symbol,
        'intervals': intervals_data,
        'configured_indicators': indicators_data
    }
    
    return jsonify(result)

@app.route('/api/indicators/<symbol>/<interval>')
def get_calculated_indicators(symbol, interval):
    """Get all calculated indicators for a specific symbol and interval"""
    
    # Get distinct indicator names and types
    indicators = db.session.query(
        CalculatedIndicator.indicator_name,
        CalculatedIndicator.indicator_type,
        func.count(CalculatedIndicator.id).label('data_points'),
        func.min(CalculatedIndicator.time).label('first_point'),
        func.max(CalculatedIndicator.time).label('last_point')
    ).filter(
        CalculatedIndicator.symbol == symbol,
        CalculatedIndicator.interval == interval
    ).group_by(
        CalculatedIndicator.indicator_name,
        CalculatedIndicator.indicator_type
    ).all()
    
    result = []
    for indicator in indicators:
        # Get parameters variations for this indicator
        params_variations = db.session.query(
            CalculatedIndicator.parameters,
            func.count(CalculatedIndicator.id).label('count')
        ).filter(
            CalculatedIndicator.symbol == symbol,
            CalculatedIndicator.interval == interval,
            CalculatedIndicator.indicator_name == indicator.indicator_name
        ).group_by(
            CalculatedIndicator.parameters
        ).all()
        
        params_list = []
        for params in params_variations:
            params_list.append({
                'parameters': params.parameters,
                'count': params.count
            })
        
        result.append({
            'name': indicator.indicator_name,
            'type': indicator.indicator_type,
            'data_points': indicator.data_points,
            'first_point': indicator.first_point.isoformat() if indicator.first_point else None,
            'last_point': indicator.last_point.isoformat() if indicator.last_point else None,
            'parameters_variations': params_list
        })
    
    return jsonify(result)

@app.route('/api/indicator-data/<symbol>/<interval>/<indicator_name>')
def get_indicator_data(symbol, interval, indicator_name):
    """Get recent calculated indicator data"""
    # Get parameters from query string
    params_str = request.args.get('parameters', '{}')
    try:
        parameters = json.loads(params_str)
    except json.JSONDecodeError:
        abort(400, description="Invalid parameters JSON")
    
    # Limit the number of data points (optional pagination)
    limit = request.args.get('limit', 100, type=int)
    offset = request.args.get('offset', 0, type=int)
    
    # Query for the indicator data
    data = db.session.query(
        CalculatedIndicator.time,
        CalculatedIndicator.value
    ).filter(
        CalculatedIndicator.symbol == symbol,
        CalculatedIndicator.interval == interval,
        CalculatedIndicator.indicator_name == indicator_name,
        CalculatedIndicator.parameters == parameters
    ).order_by(
        desc(CalculatedIndicator.time)
    ).limit(limit).offset(offset).all()
    
    result = []
    for point in data:
        result.append({
            'time': point.time.isoformat(),
            'value': point.value
        })
    
    return jsonify(result)

if __name__ == '__main__':
    app.run(debug=True, host='0.0.0.0', port=5000)
