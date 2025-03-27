#!/usr/bin/env python3
"""
LLM Strategy Optimizer

This module connects to an LLM running on Ollama to optimize trading strategy parameters.
It runs backtests with different parameter configurations and uses the LLM to suggest 
improvements based on performance metrics.
"""

import json
import argparse
import requests
import psycopg2
import psycopg2.extras
import subprocess
import uuid
from datetime import datetime, timezone, timedelta
import time
import random
import logging
import sys

# Set up logging
logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(levelname)s - %(message)s',
    handlers=[
        logging.FileHandler("strategy_optimizer.log"),
        logging.StreamHandler()
    ]
)
logger = logging.getLogger(__name__)

# Database connection parameters - adjust these to match your environment
DB_PARAMS = {
    "host": "localhost",
    "port": 5432,
    "database": "binancedb",
    "user": "binanceuser",
    "password": "binancepass"
}

# Ollama API configuration
OLLAMA_API_URL = "http://localhost:11434/api/generate"
OLLAMA_MODEL = "llama3.2"

class StrategyOptimizer:
    """
    Class for optimizing trading strategies using LLM suggestions.
    """
    
    def __init__(self, strategy_id, symbol, interval, start_date=None, end_date=None, 
                 initial_capital=10000.0, max_iterations=10):
        """
        Initialize the optimizer.
        
        Args:
            strategy_id: UUID of the strategy to optimize
            symbol: Trading pair (e.g., "BTCUSDT")
            interval: Timeframe (e.g., "1h", "4h", "1d")
            start_date: Start date for backtest (optional)
            end_date: End date for backtest (optional)
            initial_capital: Initial capital for backtesting
            max_iterations: Maximum number of optimization iterations
        """
        self.strategy_id = strategy_id
        self.symbol = symbol
        self.interval = interval
        self.start_date = start_date
        self.end_date = end_date
        self.initial_capital = initial_capital
        self.max_iterations = max_iterations
        self.current_iteration = 0
        self.optimization_history = []
        
        # Connect to database
        self.conn = psycopg2.connect(**DB_PARAMS)
        
        # Fetch the initial strategy configuration
        self.strategy = self.get_strategy()
        if not self.strategy:
            raise ValueError(f"Strategy with ID {strategy_id} not found")
        
        logger.info(f"Optimizing strategy: {self.strategy['name']} ({self.strategy_id}) on {symbol}:{interval}")
    
    def get_strategy(self):
        """
        Fetch the strategy from database.
        """
        try:
            cursor = self.conn.cursor(cursor_factory=psycopg2.extras.DictCursor)
            
            # Get strategy general info
            cursor.execute("""
                SELECT id, name, description, version, author, created_at, updated_at, 
                       enabled, assets, timeframes, parameters, risk_management, metadata
                FROM strategies
                WHERE id = %s
            """, (self.strategy_id,))
            
            strategy = cursor.fetchone()
            if not strategy:
                return None
            
            strategy_dict = dict(strategy)
            
            # Convert JSON fields
            for field in ['assets', 'timeframes', 'parameters', 'risk_management', 'metadata']:
                if strategy_dict[field]:
                    strategy_dict[field] = json.loads(strategy_dict[field])
            
            # Get indicators
            cursor.execute("""
                SELECT indicator_id, indicator_type, indicator_name, parameters, description
                FROM strategy_indicators
                WHERE strategy_id = %s
            """, (self.strategy_id,))
            
            indicators = cursor.fetchall()
            strategy_dict['indicators'] = []
            
            for indicator in indicators:
                ind_dict = dict(indicator)
                if ind_dict['parameters']:
                    ind_dict['parameters'] = json.loads(ind_dict['parameters'])
                strategy_dict['indicators'].append(ind_dict)
            
            # Get rules
            cursor.execute("""
                SELECT rule_id, name, condition, action, priority, description
                FROM strategy_rules
                WHERE strategy_id = %s
                ORDER BY priority
            """, (self.strategy_id,))
            
            rules = cursor.fetchall()
            strategy_dict['rules'] = []
            
            for rule in rules:
                rule_dict = dict(rule)
                if rule_dict['condition']:
                    rule_dict['condition'] = json.loads(rule_dict['condition'])
                if rule_dict['action']:
                    rule_dict['action'] = json.loads(rule_dict['action'])
                strategy_dict['rules'].append(rule_dict)
            
            return strategy_dict
            
        except Exception as e:
            logger.error(f"Error fetching strategy: {e}")
            return None
    
    def run_backtest(self, strategy):
        """
        Run a backtest using the Rust backtester.
        This is a placeholder - in a real implementation, you would call the Rust backtester
        via a command line interface or API.
        
        Args:
            strategy: Strategy configuration with parameters to test
            
        Returns:
            dict: Performance metrics
        """
        try:
            # In a real implementation, you would call your Rust backtester here
            # For now, we'll simulate this with a database operation
            
            # First, update the strategy parameters in the database
            cursor = self.conn.cursor()
            
            # Update parameters
            cursor.execute("""
                UPDATE strategies
                SET parameters = %s
                WHERE id = %s
            """, (json.dumps(strategy['parameters']), self.strategy_id))
            
            # Update risk management settings
            cursor.execute("""
                UPDATE strategies
                SET risk_management = %s
                WHERE id = %s
            """, (json.dumps(strategy['risk_management']), self.strategy_id))
            
            # Update indicators
            for indicator in strategy['indicators']:
                cursor.execute("""
                    UPDATE strategy_indicators
                    SET parameters = %s
                    WHERE strategy_id = %s AND indicator_id = %s
                """, (json.dumps(indicator['parameters']), self.strategy_id, indicator['indicator_id']))
            
            self.conn.commit()
            
            # Run the backtest 
            # In a real implementation, you would call a Rust CLI or API to execute the backtest
            # For this example, we'll generate simulated results
            
            backtest_id = str(uuid.uuid4())
            
            # Simulate some random performance based on previous iterations
            # This would be replaced with actual backtesting logic
            baseline = 0.5 if not self.optimization_history else self.optimization_history[-1]['performance']['win_rate'] / 100
            random_factor = random.uniform(0.8, 1.2)
            
            performance = {
                'total_trades': random.randint(50, 200),
                'winning_trades': 0,
                'losing_trades': 0,
                'win_rate': max(30, min(70, baseline * 100 * random_factor)),
                'max_drawdown': random.uniform(5, 30),
                'profit_factor': random.uniform(0.8, 2.0),
                'sharpe_ratio': random.uniform(0.5, 2.5),
                'total_return': random.uniform(-10, 50),
                'annualized_return': random.uniform(-5, 30),
                'max_consecutive_wins': random.randint(3, 10),
                'max_consecutive_losses': random.randint(3, 10),
                'avg_profit_per_win': random.uniform(1, 5),
                'avg_loss_per_loss': random.uniform(1, 3),
                'avg_win_holding_period': random.uniform(5, 48),
                'avg_loss_holding_period': random.uniform(2, 24),
                'expectancy': random.uniform(-0.5, 1.5),
            }
            
            # Set dependent values
            performance['winning_trades'] = int(performance['total_trades'] * (performance['win_rate'] / 100))
            performance['losing_trades'] = performance['total_trades'] - performance['winning_trades']
            
            # Store the backtest result in database
            cursor.execute("""
                INSERT INTO strategy_backtest_results
                (strategy_id, symbol, interval, start_date, end_date, initial_capital, 
                 final_capital, total_trades, winning_trades, losing_trades, win_rate,
                 max_drawdown, profit_factor, sharpe_ratio, total_return, annualized_return,
                 max_consecutive_wins, max_consecutive_losses, avg_profit_per_win, 
                 avg_loss_per_loss, avg_win_holding_period, avg_loss_holding_period,
                 expectancy, parameters_snapshot, created_at)
                VALUES
                (%s, %s, %s, %s, %s, %s, %s, %s, %s, %s, %s, %s, %s, %s, %s, %s, %s, %s, %s, %s, %s, %s, %s, %s, %s)
                RETURNING id
            """, (
                self.strategy_id,
                self.symbol,
                self.interval,
                self.start_date or datetime.now(timezone.utc) - timedelta(days=90),
                self.end_date or datetime.now(timezone.utc),
                self.initial_capital,
                self.initial_capital * (1 + performance['total_return'] / 100),
                performance['total_trades'],
                performance['winning_trades'],
                performance['losing_trades'],
                performance['win_rate'],
                performance['max_drawdown'],
                performance['profit_factor'],
                performance['sharpe_ratio'],
                performance['total_return'],
                performance['annualized_return'],
                performance['max_consecutive_wins'],
                performance['max_consecutive_losses'],
                performance['avg_profit_per_win'],
                performance['avg_loss_per_loss'],
                performance['avg_win_holding_period'],
                performance['avg_loss_holding_period'],
                performance['expectancy'],
                json.dumps(strategy['parameters']),
                datetime.now(timezone.utc)
            ))
            
            backtest_id = cursor.fetchone()[0]
            self.conn.commit()
            
            logger.info(f"Backtest completed with ID {backtest_id}")
            logger.info(f"Performance: win_rate={performance['win_rate']}%, total_return={performance['total_return']}%, sharpe={performance['sharpe_ratio']}")
            
            return performance
            
        except Exception as e:
            logger.error(f"Error running backtest: {e}")
            self.conn.rollback()
            return None
    
    def get_llm_suggestion(self, strategy, performance, optimization_history):
        """
        Get parameter improvement suggestions from the LLM.
        
        Args:
            strategy: Current strategy configuration
            performance: Current performance metrics
            optimization_history: History of previous optimizations
        
        Returns:
            dict: Updated strategy parameters
        """
        try:
            # Prepare the context for the LLM
            prompt = self._build_llm_prompt(strategy, performance, optimization_history)
            
            # Call the LLM API
            response = requests.post(
                OLLAMA_API_URL,
                json={
                    "model": OLLAMA_MODEL,
                    "prompt": prompt,
                    "stream": False
                }
            )
            
            if response.status_code != 200:
                logger.error(f"Error from LLM API: {response.status_code} - {response.text}")
                return None
            
            # Parse the response
            llm_response = response.json()
            suggestion_text = llm_response.get('response', '')
            
            logger.info(f"LLM suggestion received: {len(suggestion_text)} characters")
            
            # Extract the JSON part from the response
            return self._extract_parameters_from_llm_response(suggestion_text, strategy)
            
        except Exception as e:
            logger.error(f"Error getting LLM suggestion: {e}")
            return None
    
    def _build_llm_prompt(self, strategy, performance, optimization_history):
        """
        Build the prompt for the LLM with context about the strategy and performance.
        """
        # Create a simplified version of the strategy for the prompt
        strategy_overview = {
            'name': strategy['name'],
            'description': strategy['description'],
            'indicators': [{'id': i['indicator_id'], 'name': i['indicator_name'], 'parameters': i['parameters']} 
                          for i in strategy['indicators']],
            'parameters': strategy['parameters'],
            'risk_management': strategy['risk_management']
        }
        
        # Format optimization history for the prompt
        history_summary = []
        for i, iteration in enumerate(optimization_history):
            history_summary.append({
                'iteration': i + 1,
                'parameters': iteration['parameters'],
                'performance': {
                    'win_rate': iteration['performance']['win_rate'],
                    'total_return': iteration['performance']['total_return'],
                    'max_drawdown': iteration['performance']['max_drawdown'],
                    'sharpe_ratio': iteration['performance']['sharpe_ratio'],
                    'profit_factor': iteration['performance']['profit_factor'],
                    'expectancy': iteration['performance']['expectancy']
                }
            })
        
        prompt = f"""
You are a trading strategy optimizer for a cryptocurrency algorithmic trading system. Your task is to suggest improvements to the trading strategy parameters based on backtesting results.

## STRATEGY INFORMATION
Name: {strategy['name']}
Description: {strategy['description']}

## INDICATORS
{json.dumps(strategy_overview['indicators'], indent=2)}

## CURRENT PARAMETERS
{json.dumps(strategy['parameters'], indent=2)}

## RISK MANAGEMENT SETTINGS
{json.dumps(strategy['risk_management'], indent=2)}

## CURRENT PERFORMANCE METRICS
- Total Trades: {performance['total_trades']}
- Win Rate: {performance['win_rate']}%
- Total Return: {performance['total_return']}%
- Max Drawdown: {performance['max_drawdown']}%
- Sharpe Ratio: {performance['sharpe_ratio']}
- Profit Factor: {performance['profit_factor']}
- Expectancy: {performance['expectancy']}
- Avg Profit Per Win: {performance['avg_profit_per_win']}%
- Avg Loss Per Loss: {performance['avg_loss_per_loss']}%

## OPTIMIZATION HISTORY
{json.dumps(history_summary, indent=2)}

## OPTIMIZATION GOALS
1. Increase win rate
2. Increase total return
3. Reduce max drawdown
4. Improve Sharpe ratio and profit factor

## YOUR TASK
Based on the strategy details and performance metrics, suggest improvements to the strategy parameters to improve performance.

1. Analyze the current performance to identify weaknesses
2. Suggest specific parameter changes
3. Explain your reasoning for each change
4. Return an updated parameter configuration in JSON format

Requirements:
- Provide parameter values within the min/max constraints defined in the current parameters
- Don't change parameter types (integer, float, etc.)

## ANSWER FORMAT
First provide your analysis, then return the updated parameters in this format:

```json
{
  "parameters": {
    // Updated strategy parameters
  },
  "risk_management": {
    // Updated risk management parameters
  },
  "indicators": [
    {
      "id": "indicator_id",
      "parameters": {
        // Updated indicator parameters
      }
    }
  ]
}
```

Remember, for each parameter you modify, explain your reasoning and how you expect it to improve the strategy performance.
"""
        
        return prompt
    
    def _extract_parameters_from_llm_response(self, response_text, current_strategy):
        """
        Extract the JSON part from the LLM response and parse it.
        """
        try:
            # Look for JSON code block in markdown format
            json_start = response_text.find('```json')
            json_end = response_text.find('```', json_start + 7)
            
            if json_start == -1 or json_end == -1:
                # Try without specifying json
                json_start = response_text.find('```')
                json_end = response_text.find('```', json_start + 3)
            
            if json_start != -1 and json_end != -1:
                # Extract the JSON content
                json_content = response_text[json_start:json_end].strip()
                json_content = json_content.replace('```json', '').replace('```', '').strip()
                
                # Parse the JSON
                suggestions = json.loads(json_content)
                
                # Create a deep copy of the current strategy to modify
                updated_strategy = {
                    'parameters': current_strategy['parameters'].copy(),
                    'risk_management': current_strategy['risk_management'].copy(),
                    'indicators': [ind.copy() for ind in current_strategy['indicators']]
                }
                
                # Update the strategy with the suggestions
                if 'parameters' in suggestions:
                    for param, value in suggestions['parameters'].items():
                        if param in updated_strategy['parameters']:
                            updated_strategy['parameters'][param]['value'] = value
                
                if 'risk_management' in suggestions:
                    for param, value in suggestions['risk_management'].items():
                        if param in updated_strategy['risk_management']:
                            updated_strategy['risk_management'][param] = value
                
                if 'indicators' in suggestions:
                    for suggested_ind in suggestions['indicators']:
                        ind_id = suggested_ind.get('id')
                        if not ind_id:
                            continue
                            
                        # Find the matching indicator
                        for i, indicator in enumerate(updated_strategy['indicators']):
                            if indicator['indicator_id'] == ind_id:
                                if 'parameters' in suggested_ind:
                                    for param, value in suggested_ind['parameters'].items():
                                        if param in indicator['parameters']:
                                            updated_strategy['indicators'][i]['parameters'][param] = value
                
                return updated_strategy
            else:
                # If no JSON found, look for key-value pairs in the text
                logger.warning("No JSON found in LLM response, trying to extract key-value pairs")
                
                # Create a deep copy of the current strategy to modify
                updated_strategy = {
                    'parameters': current_strategy['parameters'].copy(),
                    'risk_management': current_strategy['risk_management'].copy(),
                    'indicators': [ind.copy() for ind in current_strategy['indicators']]
                }
                
                # This is a very basic parser - in a real system you would need more robust parsing
                lines = response_text.split('\n')
                for line in lines:
                    if ':' in line:
                        parts = line.split(':', 1)
                        key = parts[0].strip().lower().replace(' ', '_')
                        value_str = parts[1].strip()
                        
                        # Try to convert to number if it looks like one
                        try:
                            if '.' in value_str:
                                value = float(value_str)
                            else:
                                value = int(value_str)
                                
                            # Check if this is a known parameter
                            for param_id, param in updated_strategy['parameters'].items():
                                if param_id.lower() == key:
                                    updated_strategy['parameters'][param_id]['value'] = value
                        except ValueError:
                            pass
                
                return updated_strategy
                
        except Exception as e:
            logger.error(f"Error extracting parameters from LLM response: {e}")
            return current_strategy
    
    def optimize(self):
        """
        Run the optimization process.
        """
        logger.info(f"Starting optimization process for strategy {self.strategy_id}")
        
        # Run initial backtest with current configuration
        initial_performance = self.run_backtest(self.strategy)
        if not initial_performance:
            logger.error("Failed to run initial backtest")
            return False
        
        # Record the initial configuration and performance
        self.optimization_history.append({
            'iteration': 0,
            'parameters': {param_id: param['value'] for param_id, param in self.strategy['parameters'].items()},
            'risk_management': self.strategy['risk_management'],
            'indicators': [{
                'id': ind['indicator_id'],
                'parameters': ind['parameters']
            } for ind in self.strategy['indicators']],
            'performance': initial_performance
        })
        
        logger.info(f"Initial backtest complete. Win rate: {initial_performance['win_rate']}%, Return: {initial_performance['total_return']}%")
        
        # Start the optimization loop
        best_performance = initial_performance
        best_strategy = self.strategy
        
        for i in range(1, self.max_iterations + 1):
            self.current_iteration = i
            logger.info(f"Starting optimization iteration {i}")
            
            # Get suggestions from LLM
            updated_strategy = self.get_llm_suggestion(best_strategy, best_performance, self.optimization_history)
            if not updated_strategy:
                logger.error("Failed to get LLM suggestions")
                continue
            
            # Combine the updated parameters with the original strategy
            strategy_to_test = self.strategy.copy()
            strategy_to_test['parameters'] = updated_strategy['parameters']
            strategy_to_test['risk_management'] = updated_strategy['risk_management']
            strategy_to_test['indicators'] = updated_strategy['indicators']
            
            # Run backtest with updated parameters
            new_performance = self.run_backtest(strategy_to_test)
            if not new_performance:
                logger.error("Failed to run backtest with updated parameters")
                continue
            
            # Record this iteration
            self.optimization_history.append({
                'iteration': i,
                'parameters': {param_id: param['value'] for param_id, param in strategy_to_test['parameters'].items()},
                'risk_management': strategy_to_test['risk_management'],
                'indicators': [{
                    'id': ind['indicator_id'],
                    'parameters': ind['parameters']
                } for ind in strategy_to_test['indicators']],
                'performance': new_performance
            })
            
            # Check if this is better than the best so far
            if new_performance['expectancy'] > best_performance['expectancy']:
                logger.info(f"New best configuration found in iteration {i}")
                best_performance = new_performance
                best_strategy = strategy_to_test
        
        # Final update with the best configuration
        self.strategy = best_strategy
        self.update_strategy_in_db(best_strategy)
        
        logger.info(f"Optimization complete. Best win rate: {best_performance['win_rate']}%, Return: {best_performance['total_return']}%")
        
        return True
    
    def update_strategy_in_db(self, strategy):
        """
        Update the strategy in the database with optimized parameters.
        """
        try:
            cursor = self.conn.cursor()
            
            # Update parameters
            cursor.execute("""
                UPDATE strategies
                SET parameters = %s, risk_management = %s, updated_at = %s
                WHERE id = %s
            """, (
                json.dumps(strategy['parameters']), 
                json.dumps(strategy['risk_management']),
                datetime.now(timezone.utc),
                self.strategy_id
            ))
            
            # Update indicators
            for indicator in strategy['indicators']:
                cursor.execute("""
                    UPDATE strategy_indicators
                    SET parameters = %s
                    WHERE strategy_id = %s AND indicator_id = %s
                """, (
                    json.dumps(indicator['parameters']), 
                    self.strategy_id, 
                    indicator['indicator_id']
                ))
            
            self.conn.commit()
            logger.info(f"Strategy {self.strategy_id} updated with optimized parameters")
            
        except Exception as e:
            logger.error(f"Error updating strategy: {e}")
            self.conn.rollback()
    
    def generate_optimization_report(self):
        """
        Generate a report of the optimization process.
        """
        if not self.optimization_history:
            return "No optimization history available."
        
        report = "# Strategy Optimization Report\n\n"
        report += f"Strategy: {self.strategy['name']} ({self.strategy_id})\n"
        report += f"Symbol: {self.symbol}\n"
        report += f"Interval: {self.interval}\n"
        report += f"Iterations: {self.current_iteration}\n\n"
        
        report += "## Performance Summary\n\n"
        report += "| Iteration | Win Rate | Return | Drawdown | Sharpe | Profit Factor | Expectancy |\n"
        report += "|-----------|----------|--------|----------|--------|--------------|------------|\n"
        
        for entry in self.optimization_history:
            report += f"| {entry['iteration']} | "
            report += f"{entry['performance']['win_rate']:.2f}% | "
            report += f"{entry['performance']['total_return']:.2f}% | "
            report += f"{entry['performance']['max_drawdown']:.2f}% | "
            report += f"{entry['performance']['sharpe_ratio']:.2f} | "
            report += f"{entry['performance']['profit_factor']:.2f} | "
            report += f"{entry['performance']['expectancy']:.2f} |\n"
        
        report += "\n## Parameter Evolution\n\n"
        
        # Get all parameter IDs from the final iteration
        param_ids = list(self.optimization_history[-1]['parameters'].keys())
        
        report += "| Iteration | " + " | ".join(param_ids) + " |\n"
        report += "|-----------|" + "|".join(["-" * len(pid) for pid in param_ids]) + "|\n"
        
        for entry in self.optimization_history:
            report += f"| {entry['iteration']} | "
            report += " | ".join([str(entry['parameters'].get(pid, "N/A")) for pid in param_ids])
            report += " |\n"
        
        report += "\n## Risk Management Evolution\n\n"
        
        # Get all risk management parameter IDs
        risk_param_ids = list(self.optimization_history[-1]['risk_management'].keys())
        
        report += "| Iteration | " + " | ".join(risk_param_ids) + " |\n"
        report += "|-----------|" + "|".join(["-" * len(pid) for pid in risk_param_ids]) + "|\n"
        
        for entry in self.optimization_history:
            report += f"| {entry['iteration']} | "
            report += " | ".join([str(entry['risk_management'].get(pid, "N/A")) for pid in risk_param_ids])
            report += " |\n"
        
        # For each indicator, show its parameter evolution
        report += "\n## Indicator Parameter Evolution\n\n"
        
        for indicator in self.optimization_history[-1]['indicators']:
            ind_id = indicator['id']
            report += f"### Indicator: {ind_id}\n\n"
            
            # Get parameter IDs for this indicator
            ind_param_ids = list(indicator['parameters'].keys())
            
            report += "| Iteration | " + " | ".join(ind_param_ids) + " |\n"
            report += "|-----------|" + "|".join(["-" * len(pid) for pid in ind_param_ids]) + "|\n"
            
            for entry in self.optimization_history:
                report += f"| {entry['iteration']} | "
                
                # Find this indicator in the entry
                ind_entry = next((ind for ind in entry['indicators'] if ind['id'] == ind_id), None)
                if ind_entry:
                    report += " | ".join([str(ind_entry['parameters'].get(pid, "N/A")) for pid in ind_param_ids])
                else:
                    report += " | ".join(["N/A"] * len(ind_param_ids))
                    
                report += " |\n"
        
        return report
    
    def close(self):
        """
        Clean up resources.
        """
        if hasattr(self, 'conn') and self.conn:
            self.conn.close()
            logger.info("Database connection closed")

def main():
    """
    Main function to run the optimizer.
    """
    parser = argparse.ArgumentParser(description='Optimize trading strategies using LLM')
    parser.add_argument('--strategy_id', required=True, help='UUID of the strategy to optimize')
    parser.add_argument('--symbol', required=True, help='Trading pair (e.g., "BTCUSDT")')
    parser.add_argument('--interval', required=True, help='Timeframe (e.g., "1h", "4h", "1d")')
    parser.add_argument('--start_date', help='Start date for backtest (YYYY-MM-DD)')
    parser.add_argument('--end_date', help='End date for backtest (YYYY-MM-DD)')
    parser.add_argument('--initial_capital', type=float, default=10000.0, help='Initial capital for backtesting')
    parser.add_argument('--max_iterations', type=int, default=10, help='Maximum number of optimization iterations')
    parser.add_argument('--output', default='optimization_report.md', help='Output file for the optimization report')
    
    args = parser.parse_args()
    
    try:
        # Parse dates if provided
        start_date = None
        end_date = None
        
        if args.start_date:
            start_date = datetime.fromisoformat(args.start_date)
        if args.end_date:
            end_date = datetime.fromisoformat(args.end_date)
        
        # Create and run optimizer
        optimizer = StrategyOptimizer(
            args.strategy_id, 
            args.symbol, 
            args.interval,
            start_date, 
            end_date, 
            args.initial_capital,
            args.max_iterations
        )
        
        success = optimizer.optimize()
        
        if success:
            # Generate and save report
            report = optimizer.generate_optimization_report()
            with open(args.output, 'w') as f:
                f.write(report)
            
            logger.info(f"Optimization report saved to {args.output}")
        
        optimizer.close()
        
    except Exception as e:
        logger.error(f"Error: {e}")
        sys.exit(1)

if __name__ == "__main__":
    main()
