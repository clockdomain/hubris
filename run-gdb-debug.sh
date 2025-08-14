#!/bin/bash

# GDB Debug Script for Hubris
# This script sets up a proper GDB debugging session for Hubris firmware

set -e

# Configuration  
APP_NAME="ast1060-starter"
IMAGE_NAME="default"
BUILD_DIR="target/${APP_NAME}/dist/${IMAGE_NAME}"
GDB_SCRIPT_PATH="${BUILD_DIR}/script.gdb"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

echo -e "${BLUE}=== Hubris GDB Debug Session ===${NC}"

# Check if GDB script exists
if [ ! -f "$GDB_SCRIPT_PATH" ]; then
    echo -e "${RED}Error: GDB script not found at $GDB_SCRIPT_PATH${NC}"
    echo -e "${YELLOW}Please build the firmware first with:${NC}"
    echo "  cargo xtask dist app/${APP_NAME}/app.toml"
    exit 1
fi

echo -e "${GREEN}Using GDB script: $GDB_SCRIPT_PATH${NC}"
echo -e "${YELLOW}Make sure QEMU is running with debug flags (-s -S)${NC}"
echo ""

# Create a temporary GDB initialization file
TEMP_GDB_INIT=$(mktemp)
cat > "$TEMP_GDB_INIT" << 'EOFGDB'
# Connect to QEMU
target remote localhost:1234

# Set architecture
set architecture arm

# Load symbols and source paths
EOFGDB

# Append the generated script.gdb content
cat "$GDB_SCRIPT_PATH" >> "$TEMP_GDB_INIT"

cat >> "$TEMP_GDB_INIT" << 'EOFGDB'

# Useful settings
set confirm off
set verbose off
set pagination off

# Show what we loaded
info files

# Print current status
echo \n=== GDB Connected Successfully ===\n
echo Use 'continue' to start execution\n
echo Use 'break main' to set a breakpoint\n
echo Use 'info registers' to see CPU state\n
echo Use 'backtrace' to see call stack\n
echo =================================\n
EOFGDB

echo -e "${BLUE}Starting GDB with auto-configuration...${NC}"
echo -e "${YELLOW}GDB will automatically:${NC}"
echo "  1. Connect to QEMU (localhost:1234)"
echo "  2. Load all symbol files"
echo "  3. Set up source path remapping"
echo ""
echo -e "${GREEN}Ready to debug! Type 'continue' to start execution.${NC}"
echo ""

# Start GDB with our initialization script
exec gdb-multiarch -x "$TEMP_GDB_INIT"
