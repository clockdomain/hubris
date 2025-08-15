# AST1060 Digest Server Integration

This document describes the integration of the digest server into the AST1060 application.

## Changes Made

### 1. Application Configuration (`app/ast1060-starter/app.toml`)

Added the digest server task:

```toml
[tasks.digest_server]
name = "digest-server"
priority = 2
max-sizes = {flash = 16384, ram = 4096}
start = true
stacksize = 2048
```

- **Priority 2**: Same as UART driver, suitable for service tasks
- **Flash**: 16KB allocated for the digest server code
- **RAM**: 4KB allocated for session management and buffers
- **Auto-start**: Server starts automatically with the system

### 2. Task Dependencies

Updated the helloworld task to include digest server access:

```toml
[tasks.helloworld]
# ... existing config ...
task-slots = ["uart_driver", "digest_server"]
```

This gives the helloworld task IPC access to the digest server.

### 3. Kernel Resources

Updated kernel requirements to accommodate the new task:

```toml
[kernel]
name = "ast1060-starter"
requires = {flash = 25000, ram = 4096}  # Increased from 20000/3072
```

## Demonstration Features

The updated helloworld task now demonstrates:

### 1. One-Shot Digest Operations
- SHA-256 hashing of static data
- SHA-384 hashing (showing first 8 words)
- SHA-512 hashing (showing first 8 words)

### 2. Session-Based Digest Operations
- Multi-chunk hashing using sessions
- Proper session lifecycle management
- Demonstrates incremental data processing

### 3. Interactive Features
- Hashes any data received via UART
- Displays hash results in hexadecimal format
- Comprehensive error handling and reporting

## Expected Output

When the system boots, you should see:

```
Hello, world from AST1060!
Testing digest server...
Testing one-shot SHA-256...
SHA-256 result: 6A09E667BB67AE856A09E66873A5A6726A09E667...
Testing session-based SHA-256...
Session SHA-256 result: 6A09E667BB67AE856A09E66873A5A6726A09E667...
Testing SHA-384...
SHA-384 result: CBBB9D5DDC1C9D5DCBBB9D5D44A44A44CBBB9D5D...
Testing SHA-512...
SHA-512 result: 6A09E667BB67AE856A09E66773A5A6726A09E667...
Digest server testing complete!
```

When you send data via UART, you'll also see the hash of that data.

## System Architecture

```
┌─────────────────┐
│   HelloWorld    │
│     Task        │
└─────────┬───────┘
          │ IPC calls
          ▼
┌─────────────────┐    ┌─────────────────┐
│ Digest Server   │◄──►│   UART Driver   │
│     Task        │    │     Task        │
└─────────────────┘    └─────────────────┘
          │                      │
          │ Mock Hash            │ Hardware
          │ Operations           │ I/O
          ▼                      ▼
┌─────────────────┐    ┌─────────────────┐
│ Software Mock   │    │   AST1060 UART  │
│ Implementation  │    │   Hardware      │
└─────────────────┘    └─────────────────┘
```

## Resource Usage

- **Total Flash**: ~25KB (kernel + all tasks)
- **Total RAM**: ~4KB (kernel + all tasks)
- **Digest Server**: 16KB flash, 4KB RAM
- **Session Limit**: 8 concurrent digest sessions
- **Buffer Limit**: 1024 bytes per operation

## Building and Running

From the workspace root:

```bash
# Build the application
cargo xtask build --app ast1060-starter

# Flash to hardware (if available)
cargo xtask flash --app ast1060-starter

# Run in QEMU (if supported)
cargo xtask run --app ast1060-starter
```

## Testing the Digest Server

1. **Boot Test**: System should boot and show digest test results
2. **UART Test**: Send text via UART and observe hash output
3. **Algorithm Test**: Verify different hash algorithms work
4. **Session Test**: Verify session-based operations work
5. **Error Test**: Test error conditions (though limited in current mock)

## Future Enhancements

1. **Hardware Backend**: Replace mock with AST1060 crypto hardware
2. **Performance Testing**: Measure actual hash performance
3. **Stress Testing**: Test session limits and error conditions
4. **Security Features**: Add HMAC and authenticated operations
5. **Power Management**: Add sleep/wake support for the digest server
