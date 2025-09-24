#!/bin/bash

# Hubris QEMU I2C Scaffold App Runner
# This script runs the AST1060 I2C scaffold app in QEMU with GDB debugging enabled
# The app includes a mock I2C server, I2C client test task, and UART driver
#
# Usage:
#   ./run-qemu-i2c-scaffold-app.sh [--build] [--toml=<path>]
#
#   --build      Build the firmware before running
#   --toml=<path> Specify app.toml file (default: app/ast1060-i2c-scaffold/app.toml)

set -e

# Default configuration
APP_NAME="ast1060-i2c-scaffold"
IMAGE_NAME="default"
BUILD_FLAG=false
TOML_FILE="app/ast1060-i2c-scaffold/app.toml"

# Parse command line arguments
for arg in "$@"; do
    case $arg in
        --build)
            BUILD_FLAG=true
            shift
            ;;
        --toml=*)
            TOML_FILE="${arg#*=}"
            shift
            ;;
        -h|--help)
            echo "Usage: $0 [--build] [--toml=<path>]"
            echo ""
            echo "Options:"
            echo "  --build         Build the firmware before running"
            echo "  --toml=<path>   Specify app.toml file (default: app/ast1060-i2c-scaffold/app.toml)"
            echo "  -h, --help      Show this help message"
            echo ""
            echo "Examples:"
            echo "  $0                                              # Run with existing firmware"
            echo "  $0 --build                                     # Build and run default app.toml"
            echo "  $0 --build --toml=app/ast1060-i2c-scaffold/app-mock.toml  # Build and run mock configuration"
            exit 0
            ;;
        *)
            echo "Unknown argument: $arg"
            echo "Use -h or --help for usage information"
            exit 1
            ;;
    esac
done

BUILD_DIR="target/${APP_NAME}/dist/${IMAGE_NAME}"
FIRMWARE_PATH="${BUILD_DIR}/final.bin"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}=== Hubris AST1060 I2C Scaffold App Runner ===${NC}"
echo -e "${GREEN}This app includes:${NC}"
echo -e "  • Mock I2C Server (for testing without hardware)"
echo -e "  • I2C Client Test Task (exercises all I2C operations)"
echo -e "  • UART Driver (for debug output)"
echo -e "  • System Tasks (jefe, idle)"
echo ""

# Build firmware if requested
if [ "$BUILD_FLAG" = true ]; then
    echo -e "${BLUE}Building firmware...${NC}"
    echo -e "${GREEN}Using TOML file: $TOML_FILE${NC}"
    echo ""

    if [ ! -f "$TOML_FILE" ]; then
        echo -e "${RED}Error: TOML file not found: $TOML_FILE${NC}"
        exit 1
    fi

    echo "Running: cargo xtask dist $TOML_FILE"
    cargo xtask dist "$TOML_FILE"
    echo ""
fi

# Check if firmware exists
if [ ! -f "$FIRMWARE_PATH" ]; then
    echo -e "${RED}Error: Firmware not found at $FIRMWARE_PATH${NC}"
    echo -e "${YELLOW}Please build the firmware first with:${NC}"
    echo "  cargo xtask dist $TOML_FILE"
    echo -e "${YELLOW}Or use the --build flag:${NC}"
    echo "  $0 --build --toml=$TOML_FILE"
    exit 1
fi

echo -e "${GREEN}Found firmware: $FIRMWARE_PATH${NC}"
echo -e "${GREEN}Build directory: $BUILD_DIR${NC}"
echo -e "${GREEN}TOML file used: $TOML_FILE${NC}"

# Check if script.gdb exists
GDB_SCRIPT_PATH="${BUILD_DIR}/script.gdb"
if [ ! -f "$GDB_SCRIPT_PATH" ]; then
    echo -e "${YELLOW}Warning: GDB script not found at $GDB_SCRIPT_PATH${NC}"
else
    echo -e "${GREEN}Found GDB script: $GDB_SCRIPT_PATH${NC}"
fi

echo ""
echo -e "${BLUE}Starting QEMU with debugging enabled...${NC}"
echo -e "${YELLOW}QEMU will pause at startup waiting for GDB connection${NC}"
echo -e "${YELLOW}Connect with GDB using: gdb-multiarch${NC}"
echo -e "${YELLOW}In another terminal, run:${NC}"
echo "  cd $(pwd)"
echo "  gdb-multiarch"
echo "  (gdb) target remote localhost:1234"
echo "  (gdb) source ${GDB_SCRIPT_PATH}"
echo "  (gdb) continue"
echo ""
echo -e "${BLUE}Press Ctrl+C to stop QEMU${NC}"
echo ""

# Run QEMU with debugging
exec qemu-system-arm \
    -M ast1030-evb \
    -nographic \
    -serial mon:stdio \
    -kernel "$FIRMWARE_PATH" \
    -s \
    -S