#!/bin/bash

# Script to generate test contracts in the database
# Fetches available options from /optionsTable and creates varied contracts

set -e

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

BASE_URL="http://localhost:8080"
DB_FILE="contracts.db"

echo -e "${BLUE}========================================${NC}"
echo -e "${BLUE}    Test Contract Generator${NC}"
echo -e "${BLUE}========================================${NC}"

# Check if server is running
echo -e "\n${YELLOW}Checking server status...${NC}"
if ! curl -s "${BASE_URL}/health" > /dev/null 2>&1; then
    echo -e "${RED}❌ Server is not running on ${BASE_URL}${NC}"
    echo "Please start the server with: cargo run --bin btc_options_api"
    exit 1
fi
echo -e "${GREEN}✓ Server is running${NC}"

# Backup existing database if it exists
if [ -f "$DB_FILE" ]; then
    BACKUP_FILE="${DB_FILE}.backup_$(date +%Y%m%d_%H%M%S)"
    echo -e "\n${YELLOW}Backing up existing database to ${BACKUP_FILE}${NC}"
    cp "$DB_FILE" "$BACKUP_FILE"
fi

# Fetch options table
echo -e "\n${YELLOW}Fetching available options from /optionsTable...${NC}"
OPTIONS_RESPONSE=$(curl -s "${BASE_URL}/optionsTable")

if [ -z "$OPTIONS_RESPONSE" ] || [ "$OPTIONS_RESPONSE" = "[]" ]; then
    echo -e "${RED}❌ Failed to fetch options or empty response${NC}"
    exit 1
fi

echo -e "${GREEN}✓ Fetched options table successfully${NC}"

# Create contracts using Python
echo -e "\n${BLUE}Creating 10 test contracts with varied parameters...${NC}"

python3 << 'PYTHON_SCRIPT'
import json
import subprocess
import random
import time
from datetime import datetime, timedelta

# Configuration
BASE_URL = "http://localhost:8080"
DB_FILE = "contracts.db"

# Get options data
try:
    options_json = subprocess.check_output(["curl", "-s", f"{BASE_URL}/optionsTable"])
    options = json.loads(options_json)
except Exception as e:
    print(f"Error fetching options: {e}")
    exit(1)

if not options:
    print("No options available")
    exit(1)

print(f"Found {len(options)} available options")

# Separate calls and puts
calls = [opt for opt in options if opt['side'] == 'Call']
puts = [opt for opt in options if opt['side'] == 'Put']

print(f"  - {len(calls)} Call options")
print(f"  - {len(puts)} Put options")

# Contract creation function
def create_contract(option, quantity_factor, days_ago):
    """Create a contract via API and optionally backdate it"""
    
    # Calculate quantity based on max_quantity
    max_qty = float(option.get('max_quantity', 1.0))
    quantity = round(max_qty * quantity_factor, 8)
    
    # Ensure minimum quantity
    if quantity < 0.001:
        quantity = 0.001
    
    # Convert expire string to timestamp
    expire_map = {
        "1d": 86400,
        "2d": 172800,
        "3d": 259200,
        "5d": 432000,
        "7d": 604800
    }
    
    expire_seconds = expire_map.get(option['expire'], 86400)
    expires_timestamp = int((datetime.now() + timedelta(seconds=expire_seconds)).timestamp())
    
    # Use the premium from options table
    premium = option['premium']
    
    # Create contract data
    contract_data = {
        "side": option['side'],
        "strike_price": option['strike_price'],
        "quantity": quantity,
        "expires": expires_timestamp,
        "premium": premium
    }
    
    print(f"\n  Creating {option['side']} - Strike: ${option['strike_price']}, Qty: {quantity:.8f}, Premium: {premium:.8f}")
    
    # Create contract via API
    try:
        response = subprocess.run([
            "curl", "-s", "-w", "\n%{http_code}", "-X", "POST",
            "-H", "Content-Type: application/json",
            "-d", json.dumps(contract_data),
            f"{BASE_URL}/contract"
        ], capture_output=True, text=True)
        
        lines = response.stdout.strip().split('\n')
        status_code = lines[-1] if lines else "000"
        body = '\n'.join(lines[:-1]) if len(lines) > 1 else ""
        
        if status_code == "200":
            print(f"    ✓ Created successfully")
            
            # Backdate if needed
            if days_ago > 0:
                created_at = int((datetime.now() - timedelta(days=days_ago)).timestamp())
                try:
                    subprocess.run([
                        "sqlite3", DB_FILE,
                        f"UPDATE contracts SET created_at = {created_at} WHERE id = (SELECT MAX(id) FROM contracts);"
                    ], capture_output=True)
                    
                    # Also add to premium_history with backdated timestamp
                    product_key = f"{option['side']}-{int(option['strike_price'] * 100)}-{expires_timestamp}"
                    subprocess.run([
                        "sqlite3", DB_FILE,
                        f"INSERT INTO premium_history (product_key, side, strike_price_cents, expires, premium_str, timestamp) VALUES ('{product_key}', '{option['side']}', {int(option['strike_price'] * 100)}, {expires_timestamp}, '{premium:.8f}', {created_at});"
                    ], capture_output=True)
                    
                    print(f"    ✓ Backdated to {days_ago} days ago")
                except:
                    pass
            
            return True
        else:
            print(f"    ✗ Failed: {body}")
            return False
            
    except Exception as e:
        print(f"    ✗ Error: {e}")
        return False

# Contract specifications with varied parameters
contract_specs = [
    # Recent contracts (created today)
    {"type": "put", "quantity_factor": 0.1, "days_ago": 0},
    {"type": "call", "quantity_factor": 0.05, "days_ago": 0},
    
    # Yesterday's contracts
    {"type": "put", "quantity_factor": 0.2, "days_ago": 1},
    {"type": "call", "quantity_factor": 0.15, "days_ago": 1},
    
    # 2-3 days old contracts
    {"type": "put", "quantity_factor": 0.25, "days_ago": 2},
    {"type": "call", "quantity_factor": 0.3, "days_ago": 3},
    
    # Older contracts (4-7 days)
    {"type": "put", "quantity_factor": 0.08, "days_ago": 4},
    {"type": "call", "quantity_factor": 0.12, "days_ago": 5},
    {"type": "put", "quantity_factor": 0.18, "days_ago": 6},
    {"type": "call", "quantity_factor": 0.07, "days_ago": 7}
]

# Create contracts
success_count = 0
for i, spec in enumerate(contract_specs):
    print(f"\nContract {i+1}/10:")
    
    # Select appropriate option
    if spec["type"] == "put":
        # Vary the put selection
        if i < 4:
            option = random.choice(puts[:10])  # Near the money
        else:
            option = random.choice(puts)  # Any put
    else:
        # Vary the call selection  
        if i < 4:
            option = random.choice(calls[:10])  # Near the money
        else:
            option = random.choice(calls)  # Any call
    
    if create_contract(option, spec["quantity_factor"], spec["days_ago"]):
        success_count += 1
    
    # Small delay to avoid overwhelming the server
    time.sleep(0.1)

print(f"\n✓ Successfully created {success_count}/10 contracts")

# Show summary
print("\nFetching created contracts...")
try:
    contracts_json = subprocess.check_output(["curl", "-s", f"{BASE_URL}/contracts"])
    contracts = json.loads(contracts_json)
    
    print(f"\nTotal contracts in database: {len(contracts)}")
    
    # Show last 10 contracts
    print("\nLast 10 contracts:")
    for contract in contracts[-10:]:
        created_date = datetime.fromtimestamp(contract['created_at']).strftime('%Y-%m-%d')
        print(f"  {contract['side']} | Strike: ${contract['strike_price']} | Qty: {contract['quantity']} | Premium: {contract['premium']} | Created: {created_date}")
        
except Exception as e:
    print(f"Error fetching contracts: {e}")

PYTHON_SCRIPT

# Show final summary
echo -e "\n${BLUE}========================================${NC}"
echo -e "${GREEN}✓ Contract generation complete!${NC}"
echo -e "${BLUE}========================================${NC}"

# Database statistics
if command -v sqlite3 &> /dev/null; then
    echo -e "\n${YELLOW}Database Statistics:${NC}"
    
    TOTAL=$(sqlite3 "$DB_FILE" "SELECT COUNT(*) FROM contracts;" 2>/dev/null || echo "0")
    echo -e "${BLUE}Total contracts:${NC} $TOTAL"
    
    PUTS=$(sqlite3 "$DB_FILE" "SELECT COUNT(*) FROM contracts WHERE side = 'Put';" 2>/dev/null || echo "0")
    CALLS=$(sqlite3 "$DB_FILE" "SELECT COUNT(*) FROM contracts WHERE side = 'Call';" 2>/dev/null || echo "0") 
    echo -e "${BLUE}Distribution:${NC} $PUTS Puts, $CALLS Calls"
    
    DATE_RANGE=$(sqlite3 "$DB_FILE" "SELECT datetime(MIN(created_at), 'unixepoch', 'localtime') || ' to ' || datetime(MAX(created_at), 'unixepoch', 'localtime') FROM contracts;" 2>/dev/null || echo "N/A")
    echo -e "${BLUE}Date range:${NC} $DATE_RANGE"
    
    echo -e "\n${YELLOW}Strike price distribution:${NC}"
    sqlite3 "$DB_FILE" "SELECT strike_price, COUNT(*) as count FROM contracts GROUP BY strike_price ORDER BY strike_price;" 2>/dev/null || echo "N/A"
fi

echo -e "\n${GREEN}✓ Done! Database '$DB_FILE' has been populated with test contracts.${NC}"
echo -e "${YELLOW}You can now test the market analytics endpoints.${NC}"