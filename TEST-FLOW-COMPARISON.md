# Test Flow Comparison: Python Script vs AST1060 Client App

This document compares the test execution flow between the Python host script (`aardvark_test.py`) and the embedded AST1060 client application (`task/i2c-client/src/aardvark.rs`).

## Overview

| Aspect | Python Script (aardvark_test.py) | AST1060 Client (aardvark.rs) |
|--------|----------------------------------|------------------------------|
| **Role** | Host-side test orchestrator | Device under test (DUT) |
| **Hardware** | Aardvark adapter on PC | AST1060 on target board |
| **Language** | Python 3 | Rust (embedded, no_std) |
| **Aardvark Mode** | Configured as I2C Slave (0x50) | External device (slave) |
| **AST1060 Mode** | External device (master) | Tests both master and slave modes |
| **Execution** | Single run, then exit | Runs once at startup, then loops |
| **Timing** | Host controls with sleep() | Device initiates, no coordination |

## Key Difference: Perspective

```
Python Script Perspective (Host):
┌─────────────┐                    ┌──────────┐
│   Python    │  ← USB Cable →    │ Aardvark │  ← I2C Bus → AST1060
│   Script    │   (Control)        │ (Slave)  │   (Master tests)
└─────────────┘                    └──────────┘

AST1060 Client Perspective (Embedded):
┌──────────┐                    ┌──────────┐
│   PC     │  ← I2C Bus →      │ Aardvark │  ← I2C Bus → AST1060
│ (unused) │                    │ (Slave)  │   (Executes tests)
└──────────┘                    └──────────┘
```

**Critical Insight**: These are **complementary** programs, not alternatives:
- **Python script** = Test equipment operator (configures Aardvark, monitors results)
- **AST1060 client** = Device being tested (performs I2C operations)

---

## Test Sequence Comparison

### Test 1: Bus Scan

| Step | Python Script | AST1060 Client |
|------|--------------|----------------|
| **Setup** | Configure Aardvark as slave at 0x50 | Assume Aardvark already at 0x50 |
| **Action** | Wait for incoming transaction | Send zero-byte write to 0x50 |
| **Check** | `aa_i2c_slave_read()` - did we receive addr 0x50? | Check if ACK received |
| **Pass Criteria** | Received transaction at correct address | ACK from slave (device exists) |
| **Output** | `✓ PASS: AST1060 detected device at 0x50` | `✓ Device found at 0x50 (ACK received)` |

**Code Comparison**:

```python
# Python: Passive receiver
(count, addr, data) = aa_i2c_slave_read(handle, 100)
if count >= 0 and addr == 0x50:
    print("✓ PASS: AST1060 detected device")
```

```rust
// Rust: Active initiator
let scan_result = device.write(&[]);  // Zero-byte write
match scan_result {
    Ok(_) => uart_send(b"✓ Device found at 0x50\r\n"),
    Err(_) => uart_send(b"✗ No device found\r\n"),
}
```

**Synchronization**: Python waits 2 seconds (`self.wait_for_ast1060("bus scan", 2000)`) hoping AST1060 performs test during that window.

---

### Test 2: Single Byte Write

| Step | Python Script | AST1060 Client |
|------|--------------|----------------|
| **Setup** | Clear receive buffer | Prepare data byte 0x42 |
| **Action** | Monitor for incoming write | Write [0x42] to Aardvark |
| **Check** | Did we receive [0x42]? | Did write succeed (ACK)? |
| **Pass Criteria** | `count == 1 and data[0] == 0x42` | `Ok(_)` from write operation |
| **Output** | `✓ PASS: Received correct data: [0x42]` | `✓ Write successful` |

**Code Comparison**:

```python
# Python: Validate received data
aa_i2c_slave_read(handle, 0)  # Clear buffer
time.sleep(2)  # Wait for AST1060
(count, addr, data) = aa_i2c_slave_read(handle, 100)
if count == 1 and data[0] == 0x42:
    print("✓ PASS: Received correct data")
```

```rust
// Rust: Perform write operation
let write_data = [0x42u8];
let result = device.write(&write_data);
match result {
    Ok(_) => uart_send(b"✓ Write successful\r\n"),
    Err(e) => print_error(e),
}
```

**Asymmetry**: Python validates content (0x42), but Rust only validates success. Rust assumes Python will check the actual data received.

---

### Test 3: Single Byte Read

| Step | Python Script | AST1060 Client |
|------|--------------|----------------|
| **Setup** | Load slave response buffer: [0xAB] | Expect Aardvark has 0xAB loaded |
| **Action** | Provide data when master reads | Read from Aardvark |
| **Check** | (Can't verify what was read) | Did read return 0xAB? |
| **Pass Criteria** | Response loaded successfully | `value == 0xAB` |
| **Output** | `✓ PASS: Response buffer loaded` | `✓ Read successful: [0xAB]` / `✓ Data matches` |

**Code Comparison**:

```python
# Python: Pre-load response
response_data = [0xAB]
aa_i2c_slave_set_response(handle, response_data)
print("✓ Loaded response: [0xAB]")
time.sleep(2)
print("✓ PASS: Response buffer loaded")
print("  (Check AST1060 console to verify it received 0xAB)")
```

```rust
// Rust: Read and validate
let result: Result<u8, ResponseCode> = device.read_reg(0u8);
match result {
    Ok(value) => {
        if value == 0xAB {
            uart_send(b"✓ Data matches expected value (0xAB)\r\n");
        } else {
            uart_send(b"✗ Data mismatch\r\n");
        }
    }
    Err(e) => print_error(e),
}
```

**Limitation**: Aardvark API doesn't report what master actually read, so Python can't verify. Python trusts AST1060 console output.

---

### Test 4: Write-Read (Register Access)

| Step | Python Script | AST1060 Client |
|------|--------------|----------------|
| **Setup** | Load response: [0xDE, 0xAD, 0xBE, 0xEF] | Expect 4-byte response ready |
| **Action** | Receive write (reg addr), provide read data | Write 0x10, RESTART, read 4 bytes |
| **Check** | Did we receive write of 0x10? | Read data == [DE, AD, BE, EF]? |
| **Pass Criteria** | Write data[0] == 0x10 | All 4 bytes match |
| **Output** | `✓ PASS: Received register write: [0x10]` | `✓ Data matches expected sequence` |

**Code Comparison**:

```python
# Python: Pre-load multi-byte response
response_data = [0xDE, 0xAD, 0xBE, 0xEF]
aa_i2c_slave_set_response(handle, response_data)
aa_i2c_slave_read(handle, 0)  # Clear
time.sleep(2)
(count, addr, data) = aa_i2c_slave_read(handle, 100)
if count >= 1 and data[0] == 0x10:
    print("✓ PASS: Received register write: [0x10]")
```

```rust
// Rust: Combined write-read transaction
let mut read_buffer = [0u8; 4];
let result = device.read_reg_into(0x10u8, &mut read_buffer);
match result {
    Ok(_) => {
        if read_buffer == [0xDE, 0xAD, 0xBE, 0xEF] {
            uart_send(b"✓ Data matches expected sequence\r\n");
        }
    }
    Err(e) => print_error(e),
}
```

**Protocol Detail**: This uses I2C RESTART between write and read phases:
```
START → 0xA0 (addr+W) → 0x10 (reg) → RESTART → 0xA1 (addr+R) → [DE AD BE EF] → STOP
```

---

### Test 5: Buffer Mode (32 Bytes)

| Step | Python Script | AST1060 Client |
|------|--------------|----------------|
| **Setup** | Clear receive buffer | Create pattern [0x00-0x1F] |
| **Action** | Receive and validate 32-byte pattern | Write 32 bytes to Aardvark |
| **Check** | All 32 bytes match expected pattern? | Write succeeded? |
| **Pass Criteria** | `count == 32` and pattern matches | `Ok(_)` from write |
| **Output** | `✓ PASS: Received 32 bytes in single transaction` | `✓ Write successful` |

**Code Comparison**:

```python
# Python: Validate 32-byte pattern
time.sleep(2)
(count, addr, data) = aa_i2c_slave_read(handle, 100)
if count == 32:
    expected = list(range(32))  # [0x00..0x1F]
    actual = list(data[:32])
    if actual == expected:
        print("✓ PASS: Received 32 bytes")
```

```rust
// Rust: Send 32-byte pattern
let mut write_data = [0u8; 32];
for (i, byte) in write_data.iter_mut().enumerate() {
    *byte = i as u8;
}
let result = device.write(&write_data);
match result {
    Ok(_) => uart_send(b"✓ Write successful\r\n"),
    Err(e) => print_error(e),
}
```

**Important**: This tests AST1060 **buffer mode** (hardware feature that sends all 32 bytes in one I2C transaction vs byte-by-byte).

---

### Test 6: Error Recovery

| Step | Python Script | AST1060 Client |
|------|--------------|----------------|
| **Setup** | Disable Aardvark slave (causes NACK) | Prepare to write to invalid addr 0x7F |
| **Action** | Wait for error attempt, re-enable slave | Write to 0x7F, then retry valid addr |
| **Check** | Did retry transaction succeed? | First fails, second succeeds? |
| **Pass Criteria** | Received transaction after re-enable | Error detected AND recovery worked |
| **Output** | `✓ PASS: Bus recovery successful` | `✓ Error detected` + `✓ Bus recovery successful` |

**Code Comparison**:

```python
# Python: Simulate failure by disabling slave
aa_i2c_slave_disable(handle)
time.sleep(2)  # Wait for error attempt
aa_i2c_slave_enable(handle, 0x50, 1024, 1024)
time.sleep(2)  # Wait for retry
(count, addr, data) = aa_i2c_slave_read(handle, 100)
if count >= 0:
    print("✓ PASS: Bus recovery successful")
```

```rust
// Rust: Intentional failure then recovery
let invalid_device = I2cDevice::new(..., 0x7F, ...);
let result = invalid_device.write(&[0x00]);
match result {
    Err(e) => {
        uart_send(b"✓ Error detected (expected)\r\n");
        // Test recovery
        let recovery_result = device.write(&[0xFF]);
        match recovery_result {
            Ok(_) => uart_send(b"✓ Bus recovery successful\r\n"),
            Err(_) => uart_send(b"✗ Bus recovery failed\r\n"),
        }
    }
    Ok(_) => uart_send(b"✗ Unexpected success\r\n"),
}
```

**Purpose**: Validates SMBus timeout and bus recovery mechanism work correctly.

---

## Slave Mode Tests (AST1060 Only)

The Python script **does not** include Tests 7-9 (slave mode tests) because the Aardvark would need to be reconfigured as I2C master, which requires manual intervention.

### Test 7-9: AST1060 as Slave

| Test | AST1060 Action | Aardvark Required Mode |
|------|----------------|------------------------|
| Test 7 | Configure self as slave at 0x42 | Master (manual reconfiguration) |
| Test 8 | Receive message from Aardvark master | Master writes to 0x42 |
| Test 9 | Bidirectional: receive command, send response | Master read/write to 0x42 |

**Why not in Python?**
```python
# This would require Aardvark mode switching:
aa_i2c_slave_disable(handle)  # Stop being slave
aa_configure(handle, AA_CONFIG_I2C_MASTER)  # Become master
aa_i2c_master_write(handle, 0x42, data)  # Write to AST1060
# But then can't monitor results simultaneously!
```

**Manual Test Procedure** (from AARDVARK-BRINGUP.md):
1. Run Python script for Tests 1-6 (master mode)
2. Exit Python script
3. Use Aardvark GUI application in master mode
4. Monitor AST1060 UART console for Test 7-9 results
5. Manually send I2C commands to AST1060 slave address 0x42

---

## Timing and Synchronization

### Python Script Timing Strategy

```python
def wait_for_ast1060(self, test_name, timeout_ms=5000):
    """Wait for AST1060 to perform action"""
    print(f"  Waiting for AST1060 {test_name}...")
    time.sleep(timeout_ms / 1000.0)
```

**Characteristics**:
- Fixed delays (2 seconds per test)
- **No handshake** with AST1060
- Assumes AST1060 runs tests in ~same order
- Works if AST1060 starts within ~2 seconds of Python script

**Failure Modes**:
- Python starts before AST1060 boots → Tests timeout
- AST1060 runs faster than expected → Python reads stale data
- Tests run out of sync → Wrong test validates wrong operation

### AST1060 Timing Strategy

```rust
pub fn run_aardvark_tests(device: &I2cDevice) {
    // Run tests immediately at startup
    test_1_bus_scan(device);
    test_2_single_write(device);
    // ... etc, no delays between tests
}

fn main() -> ! {
    // Run once at startup
    aardvark::run_aardvark_tests(&device);
    
    loop {
        // Then loop regular tests every 5 seconds
        run_master_mode_tests(&device);
        hl::sleep_for(5000);
    }
}
```

**Characteristics**:
- Runs immediately on boot
- No coordination with Python
- Tests execute back-to-back (~100ms total)
- Repeats every 5 seconds in main loop

**Timing Diagram**:
```
Time (seconds):
0    1    2    3    4    5    6    7    8
├────┼────┼────┼────┼────┼────┼────┼────┤
│ Boot & Run Aardvark Tests   │ Main Loop Tests...
│ T1 T2 T3 T4 T5 T6 T7 T8 T9  │           │
└─────────────────────────────┴───────────┴─

Python Script:
0    1    2    3    4    5    6    7    8
├────┼────┼────┼────┼────┼────┼────┼────┤
│ Setup │ T1  │ T2  │ T3  │ T4  │ T5  │ T6
│       │<-2s→│<-2s→│<-2s→│<-2s→│<-2s→│<-2s→
```

**Misalignment Risk**: If AST1060 boots and completes all tests in first 2 seconds, Python Test 1 captures multiple tests' data.

---

## Proposed Synchronization Improvement

### Option 1: Delayed Start in AST1060

```rust
fn main() -> ! {
    uart_send(b"=== I2C Client Test Task Started ===\r\n");
    uart_send(b"Waiting 5 seconds for Python script setup...\r\n");
    hl::sleep_for(5000);  // Give Python time to configure Aardvark
    
    aardvark::run_aardvark_tests(&device);
    // ...
}
```

### Option 2: Handshake via GPIO or UART

```python
# Python waits for AST1060 ready signal
uart_send_to_ast1060("START_TEST_1\n")
wait_for_response()
```

```rust
// AST1060 waits for Python trigger
wait_for_uart_command(b"START_TEST_1");
test_1_bus_scan(device);
uart_send(b"TEST_1_COMPLETE\r\n");
```

### Option 3: Inter-Test Delays in AST1060

```rust
pub fn run_aardvark_tests(device: &I2cDevice) {
    test_1_bus_scan(device);
    hl::sleep_for(2500);  // Match Python's 2-second wait + margin
    
    test_2_single_write(device);
    hl::sleep_for(2500);
    // ... etc
}
```

**Recommended**: **Option 3** - Simplest, no protocol changes, works with independent timing.

---

## Pass/Fail Determination

### Python Script

```python
self.pass_count = 0
self.test_count = 0

def test_X(...):
    self.test_count += 1
    if check_passes:
        self.pass_count += 1
        return True
    else:
        return False

# Final summary
if self.pass_count == self.test_count:
    print("✓ ALL TESTS PASSED")
    return 0
else:
    print("✗ SOME TESTS FAILED")
    return 1
```

**Exit Code**: Returns 0 (success) or 1 (failure) for CI integration.

### AST1060 Client

```rust
// No pass/fail tracking or summary
// Each test prints ✓ PASS or ✗ FAIL individually
// Runs indefinitely in loop
```

**Missing Features**:
- No test count
- No pass/fail summary
- No halt on failure
- No exit (runs forever)

**Rationale**: Embedded system, no concept of "exit" or CI. Human reads UART console.

---

## Data Flow Summary

```
Master Mode Tests (AST1060 as Master):

  Python Script          Aardvark          AST1060 Client
  ─────────────          ────────          ──────────────
       │                    │                     │
       ├─Configure slave────┤                     │
       │    (addr 0x50)     │                     │
       │                    │                     │
       │                    │   ←─── write() ─────┤  Test 2
       │                    ├──── ACK ──────→     │
       │                    │                     │
       ├─ Read received ────┤                     │
       │   (data = 0x42)    │                     │
       ├─ Validate ✓        │                     │
       │                    │                     │
       ├─ Load response ────┤                     │
       │   (0xAB)           │                     │
       │                    │   ←─── read() ──────┤  Test 3
       │                    ├──── 0xAB ─────→     │
       │                    │                     │
       │                    │   ←─Validate 0xAB───┤


Slave Mode Tests (AST1060 as Slave):

  Python Script          Aardvark          AST1060 Client
  ─────────────          ────────          ──────────────
       X                    │                     │
   (not used)               │                     ├─ Configure self─┐
                            │                     │  as slave 0x42  │  Test 7
                            │                     ├─────────────────┘
                            │                     │
       [Manual GUI]         │   ── write(0x42) ──→│  Test 8
                            │   ←──── ACK ────────┤
                            │                     ├─ Receive msg ✓
                            │                     │
       [Manual GUI]         │   ── write(0x42) ──→│  Test 9
                            │   ←──── ACK ────────┤
                            │   ── read(0x42) ────→
                            │   ←──── Response ───┤
```

---

## Command Line Usage Comparison

### Python Script

```bash
# Run all tests
$ python3 aardvark_test.py

# Run specific test
$ python3 aardvark_test.py --test 2

# Use different Aardvark device
$ python3 aardvark_test.py --device 1

# Exit codes
$ echo $?
0  # All tests passed
1  # Some tests failed or error occurred
```

### AST1060 Client

```bash
# Build and flash
$ cargo xtask dist app/ast1060-i2c-example/app.toml
$ # Flash to target via JTAG/QEMU

# Monitor output (separate terminal)
$ screen /dev/ttyUSB0 115200

# No command-line options - behavior fixed at compile time
# Runs continuously until powered off
```

---

## Summary Table

| Feature | Python Script | AST1060 Client |
|---------|--------------|----------------|
| **Execution Model** | Single run, exit | Continuous loop |
| **Test Count** | 6 tests (master mode only) | 9 tests (6 master + 3 slave) |
| **Aardvark Role** | Slave (passive) | External slave device |
| **AST1060 Role** | External master device | Active test executor |
| **Synchronization** | Time-based delays (2s) | No delays (back-to-back) |
| **Data Validation** | Validates received bytes | Validates operation success |
| **Pass/Fail Tracking** | Yes, with summary | Per-test only |
| **Exit Code** | Returns 0 or 1 | N/A (runs forever) |
| **CI Integration** | Yes (exit code) | No (manual verification) |
| **User Interface** | Terminal output | UART console |
| **Error Handling** | Exceptions, try/except | Match Result<>, print error |
| **Configuration** | CLI arguments | Compile-time constants |
| **Hardware Setup** | Automated via API | Requires pre-wired hardware |
| **Best Use Case** | Automated CI testing | Interactive hardware bringup |

---

## Recommendations

### For Reliable Testing

1. **Start Python script first** (configures Aardvark)
2. **Wait 2 seconds**, then power on AST1060
3. **Watch both consoles** (Python terminal + UART)
4. **Verify Test 1 sync** - if Python doesn't see Test 1, restart
5. **Manual slave tests** - Use Aardvark GUI after Tests 1-6 complete

### For Automated CI

Current setup is **not suitable** for fully automated testing because:
- No handshake between Python and AST1060
- Timing-dependent (2-second windows)
- Slave tests require manual Aardvark reconfiguration
- AST1060 never exits (can't detect completion)

**To make CI-ready**:
1. Add inter-test delays in AST1060 (match Python timing)
2. Add UART handshake: AST1060 signals "TEST_N_START" / "TEST_N_DONE"
3. Python parses UART for these signals
4. Or: Use Aardvark as I2C analyzer (passive monitor)

### For Development

Current design is **excellent** for:
- Manual hardware validation
- Driver bringup and debugging
- Visual verification of I2C transactions
- Understanding protocol flow

Keep both programs because they serve **different purposes**:
- **Python** = Test orchestrator and validator
- **AST1060** = Device under test
