# AST1060 Starter Application with Digest Server

This is a minimal Hubris application for the AST1060 platform that includes a comprehensive digest (cryptographic hash) server with hardware acceleration support.

## Overview

The application demonstrates:
- **Hardware-accelerated digest operations** using ASPEED HACE controller (on real hardware)
- **Mock digest device** for QEMU testing - stub implementation for development
- **Safe OpenPRoT HAL implementation** with zero unsafe code
- **Session-based streaming** for large data processing
- **One-shot operations** for simple hash calculations
- **Comprehensive test suite** validating all functionality

## Components

- **Kernel**: Core Hubris OS kernel
- **Digest Server**: Hardware-accelerated SHA-256/384/512 implementation
- **HelloWorld Task**: Comprehensive digest server test suite
- **UART Driver**: Serial communication for test output

## Prerequisites

### Development Environment
```bash
# Rust toolchain (nightly required)
rustup toolchain install nightly
rustup default nightly

# ARM cross-compilation target
rustup target add thumbv7em-none-eabihf

# Additional tools
sudo apt install gcc-arm-none-eabi gdb-multiarch qemu-system-arm
```

### Hardware Requirements
- **AST1060 EVB** (Evaluation Board) or compatible hardware
- **UART connection** for serial output (115200 baud, 8N1)

### QEMU Testing (Recommended)
- **qemu-system-arm** with AST1060 EVB support
- **Mock digest device** - Stub hash implementation for testing
- No additional hardware required

## Building

### Quick Build
```bash
./build-ast1060.sh
```

### Manual Build
```bash
cargo xtask dist app/ast1060-starter/app.toml
```

### Build Output
- **Firmware binary**: `target/ast1060-starter/dist/default/final.bin`
- **ELF file**: `target/ast1060-starter/dist/default/final.elf`
- **Task binaries**: Individual task ELF files for debugging

## Running Tests

### QEMU Emulation (Recommended)
```bash
# Terminal 1: Start QEMU with test firmware
./run-qemu-debug.sh

# Terminal 2: Connect GDB debugger (optional)
./run-gdb-debug.sh
```

**Note**: 
- QEMU will pause at startup waiting for GDB connection. Use `continue` in GDB to start execution.
- **QEMU uses mock digest device** - Stub implementation for testing digest server logic
- Mock device supports up to 8 concurrent sessions (vs 1 for real hardware)

### Hardware Testing
Hardware testing has not been validated yet. The firmware binary is available at `target/ast1060-starter/dist/default/final.bin` for those who wish to test on actual AST1060 hardware.

## Test Suite Overview

The HelloWorld task runs a comprehensive digest server test suite:

### Test 1: Server Connectivity
- Verifies digest server is responding
- Tests basic SHA-256 one-shot operation

### Test 2: One-shot Operations
- **SHA-256** with known test vector (`"abc"`)
- **SHA-384** algorithm verification
- **SHA-512** algorithm verification
- Validates against expected hash outputs

### Test 3: Session Operations (Streaming)
- **SHA-256 streaming**: `"Hello" + ", " + "World!"`
- **SHA-384 streaming**: `"Streaming" + " SHA-384" + " test!"`
- **SHA-512 streaming**: `"Streaming" + " SHA-512" + " works great!"`
- Demonstrates multi-chunk hash processing

### Test 4: Multiple Sessions
- Attempts to create multiple concurrent sessions
- **Expected behavior**: Only 1 session succeeds (hardware limitation)
- Demonstrates session management infrastructure

### Test 5: Error Conditions
- Invalid session ID handling
- Session limit testing
- Proper error reporting validation

### Test 6: One-shot Error Conditions
- Input validation testing
- Edge case handling

## Expected Output

```
=== Hubris Digest Server Test Suite ===
Running on AST1060 in QEMU (Mock/Stub Device)

Starting digest server tests...

[TEST 1] Server Connectivity Test
  [OK] Server responding
  [OK] Basic SHA-256 operation successful

[TEST 2] One-shot Operations
  Testing SHA-256 with known vector...
    Input: 'abc'
    SHA-256: ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad
    [OK] Known vector matches!
  Testing SHA-384...
    SHA-384: cb00753f45a35e8bb5a03d699ac65007272c32ab0eded1631a8b605a43ff5bed...
    [OK] SHA-384 operation successful
  Testing SHA-512...
    SHA-512: ddaf35a193617abacc417349ae20413112e6fa4e89a97ea20a9eeee64b55d39a...
    [OK] SHA-512 operation successful

[TEST 3] Session Operations
  [OK] SHA-256 stream
    ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad
  [OK] SHA-384 stream
    cb00753f45a35e8bb5a03d699ac65007272c32ab0eded1631a8b605a43ff5bed...
  [OK] SHA-512 stream
    ddaf35a193617abacc417349ae20413112e6fa4e89a97ea20a9eeee64b55d39a...

[TEST 4] Multiple Sessions
  Creating multiple sessions...
    [OK] Session 0 created (ID: 1)
    [FAIL] Session 1 failed: TooManySessions
    [FAIL] Session 2 failed: TooManySessions
    [FAIL] Session 3 failed: TooManySessions
  Created 1 sessions total

[TEST 5] Error Conditions
  Testing invalid session ID...
    [OK] Correctly rejected invalid session
  Testing session limit...
    Session limit reached at: 1 sessions

=== Test Suite Complete ===
All tests finished. System will now echo received data.
```

## Architecture Notes

### Hardware Acceleration
- **ASPEED HACE**: Hardware cryptographic accelerator (real hardware)
- **MockDigestController**: Stub hash implementation (QEMU testing)
- **Single controller**: Only one digest operation at a time (real hardware limitation)
- **Session management**: Infrastructure supports multiple sessions, but single hardware controller limits concurrency on real hardware

### Device Differences
- **Real Hardware (AST1060)**: ASPEED HACE controller, 1 concurrent session max
- **QEMU (Mock Device)**: Stub implementation, 8 concurrent sessions supported (but still limited by single controller architecture)

### Security Features
- **Zero unsafe code**: Complete migration to safe OpenPRoT HAL
- **Move semantics**: Controller ownership prevents resource conflicts
- **Error handling**: Comprehensive validation and error reporting

### Memory Usage
- **Flash**: ~51KB total firmware
- **RAM**: ~31% utilization
- **Session storage**: Up to 8 session contexts (limited by hardware to 1 active)

## Troubleshooting

### Build Issues
```bash
# Clean build
cargo clean
./build-ast1060.sh

# Check Rust toolchain
rustup show
```

### QEMU Issues
```bash
# Kill existing QEMU processes
pkill -f qemu-system-arm

# Check if debug port is in use
netstat -tlnp | grep 1234
```

### Serial Output Issues
- Hardware testing not yet validated
- For QEMU: output appears directly in terminal running `./run-qemu-debug.sh`

## Development

### Adding New Tests
1. Edit `task/helloworld/src/main.rs`
2. Add test function following existing pattern
3. Call from `run_digest_test_suite()`
4. Rebuild with `./build-ast1060.sh`

### Debugging
```bash
# Two-terminal debugging workflow:

# Terminal 1: Start QEMU (will wait for GDB)
./run-qemu-debug.sh

# Terminal 2: Connect GDB debugger  
./run-gdb-debug.sh

# In GDB, use standard commands:
# (gdb) continue          # Start execution
# (gdb) break main        # Set breakpoint
# (gdb) info threads      # Show all tasks
# (gdb) thread 2          # Switch to task
```

### Performance Tuning
- Monitor flash usage: Currently ~78% utilized
- Session limits configurable via `MAX_CONCURRENT_SESSIONS`
- Hardware capabilities defined in digest server traits

## TODO / Future Work

### Memory Optimization
- **Trim down memory footprint**: HelloWorld task currently at flash limit (11904/12288 bytes)
- **Optimize string literals**: Reduce test output verbosity to save flash space
- **Code size analysis**: Identify and eliminate unused code paths
- **Compiler optimizations**: Explore additional size optimization flags

### Testing & Validation
- **Hardware testing**: Validate on actual AST1060 hardware
- **Performance benchmarking**: Real hardware vs mock device comparison
- **Stress testing**: Extended session lifecycle and error recovery

### Feature Enhancements
- **Additional algorithms**: SHA3 support (currently stubbed)
- **Session timeouts**: Implement session cleanup and timeout handling
- **Multiple controller support**: Architecture changes for true concurrency (if hardware permits)
