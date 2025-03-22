#!/usr/bin/env python3
import psycopg2
import psycopg2.extras
import json
from collections import defaultdict
from tabulate import tabulate

# Database connection parameters - adjust these to match your environment
DB_PARAMS = {
    "host": "localhost",
    "port": 5432,
    "database": "binancedb",
    "user": "binanceuser",
    "password": "binancepass"
}

def main():
    try:
        # Connect to the database
        conn = psycopg2.connect(**DB_PARAMS)
        cursor = conn.cursor(cursor_factory=psycopg2.extras.DictCursor)
        
        # Query for all enabled indicators
        cursor.execute("""
            SELECT 
                symbol, 
                interval, 
                indicator_type, 
                indicator_name, 
                parameters,
                created_at,
                updated_at
            FROM 
                indicator_config 
            WHERE 
                enabled = TRUE
            ORDER BY 
                symbol, 
                interval, 
                indicator_type, 
                indicator_name
        """)
        
        indicators = cursor.fetchall()
        
        if not indicators:
            print("No enabled indicators found in the configuration.")
            return
        
        print(f"Found {len(indicators)} enabled indicator configurations\n")
        
        # Group by indicator type
        by_type = defaultdict(list)
        for indicator in indicators:
            by_type[indicator['indicator_type']].append(indicator)
        
        # Print summary by type
        print("=== SUMMARY BY TYPE ===")
        type_counts = [(t, len(indicators)) for t, indicators in by_type.items()]
        print(tabulate(type_counts, headers=["Indicator Type", "Count"]))
        print("\n")
        
        # Print summary by name
        by_name = defaultdict(list)
        for indicator in indicators:
            by_name[indicator['indicator_name']].append(indicator)
        
        print("=== SUMMARY BY NAME ===")
        name_counts = [(n, len(indicators)) for n, indicators in sorted(by_name.items())]
        print(tabulate(name_counts, headers=["Indicator Name", "Count"]))
        print("\n")
        
        # Detailed list of indicators
        print("=== DETAILED INDICATORS LIST ===")
        table_data = []
        for indicator in indicators:
            # Format the parameters as a more readable string
            params_str = json.dumps(indicator['parameters'], indent=2)
            # Limit length for display
            if len(params_str) > 30:
                params_str = params_str[:27] + "..."
            
            table_data.append([
                indicator['symbol'],
                indicator['interval'],
                indicator['indicator_type'],
                indicator['indicator_name'],
                params_str,
                indicator['created_at'].strftime('%Y-%m-%d %H:%M:%S')
            ])
        
        print(tabulate(table_data, headers=[
            "Symbol", "Interval", "Type", "Name", "Parameters", "Created At"
        ]))
        
        # Symbol and interval combinations
        symbol_intervals = set((ind['symbol'], ind['interval']) for ind in indicators)
        print(f"\n=== SYMBOL/INTERVAL COMBINATIONS ({len(symbol_intervals)}) ===")
        si_table = sorted([[symbol, interval] for symbol, interval in symbol_intervals])
        print(tabulate(si_table, headers=["Symbol", "Interval"]))
        
    except Exception as e:
        print(f"Error: {e}")
    finally:
        if 'conn' in locals():
            conn.close()

if __name__ == "__main__":
    main()
