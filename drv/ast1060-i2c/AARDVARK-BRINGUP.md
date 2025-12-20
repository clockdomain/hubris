# AST1060 I2C Driver Bringup with Aardvark

Quick test plan for bringing up the AST1060 I2C driver using an Aardvark I2C/SPI adapter.

## Equipment Required

- **AST1060 development board** with I2C pins accessible
- **Total Phase Aardvark I2C/SPI Host Adapter**
- **USB connection** for Aardvark to PC
- **UART connection** for AST1060 debug console
- **Jumper wires** for I2C connection

## Hardware Setup

### Pin Connections

Connect Aardvark to AST1060 I2C0 bus:

```
Aardvark Pin    →    AST1060 Pin       Signal
─────────────────────────────────────────────
SCL (Pin 1)     →    I2C0_SCL         Clock
SDA (Pin 3)     →    I2C0_SDA         Data
GND (Pin 8)     →    GND              Ground
VCC (Pin 4)     →    3.3V (optional)  Power
```

**Pull-up Resistors**: 
- If Aardvark pull-ups disabled: Use external 2.2kΩ resistors to 3.3V on SCL/SDA
- If Aardvark pull-ups enabled: Enable in Aardvark software (2.2kΩ internal)

### Build and Flash Firmware

```bash
cd /path/to/hubris
cargo xtask dist app/ast1060-i2c-example/app.toml
# Flash via JTAG/SWD or QEMU
```

## Test Sequence

### Test 1: Bus Scan (Verify Connectivity)

**Goal**: Confirm AST1060 can detect Aardvark on I2C bus

**Aardvark Setup**:
1. Launch Aardvark Control Center
2. Configure as I2C Slave
3. Set slave address: `0x50`
4. Enable I2C slave mode

**AST1060 Action**:
Monitor debug console output from i2c_client task:
```
I2C bus scan: Looking for device at 0x50...
✓ Device found at 0x50 (ACK received)
```

**Success Criteria**: Device detected, no timeout errors

---

### Test 2: Single Byte Write

**Goal**: Verify AST1060 can write data to Aardvark

**Aardvark Setup**:
1. Slave mode at 0x50
2. Monitor incoming I2C transactions
3. Clear receive buffer

**Expected Transaction**:
```
START → 0xA0 (0x50 << 1 | W) → 0x42 → STOP
```

**AST1060 Console Output**:
```
Master Write Test: addr=0x50, data=[0x42]
✓ Write successful
```

**Aardvark Display**:
```
I2C Slave Received: [0x42]
```

**Success Criteria**: Aardvark receives 0x42

---

### Test 3: Single Byte Read

**Goal**: Verify AST1060 can read data from Aardvark

**Aardvark Setup**:
1. Slave mode at 0x50
2. Load slave response buffer: `[0xAB]`

**Expected Transaction**:
```
START → 0xA1 (0x50 << 1 | R) → [0xAB] ← NACK → STOP
```

**AST1060 Console Output**:
```
Master Read Test: addr=0x50, len=1
✓ Read successful: [0xAB]
```

**Success Criteria**: AST1060 reads 0xAB correctly

---

### Test 4: Write-Read (Register Access Pattern)

**Goal**: Test combined write-then-read transaction

**Aardvark Setup**:
1. Slave mode at 0x50
2. Program response for register 0x10: `[0xDE, 0xAD, 0xBE, 0EF]`

**Expected Transaction**:
```
START → 0xA0 → 0x10 → RESTART → 0xA1 → [0xDE, 0xAD, 0xBE, 0xEF] ← NACK → STOP
```

**AST1060 Console Output**:
```
Write-Read Test: addr=0x50, reg=0x10, read_len=4
✓ Write-Read successful: [DE, AD, BE, EF]
```

**Success Criteria**: Correct 4-byte sequence received

---

### Test 5: Buffer Mode (32 Bytes)

**Goal**: Verify buffer mode for larger transfers

**Aardvark Setup**:
1. Slave mode at 0x50
2. Clear receive buffer
3. Prepare to receive 32 bytes

**AST1060 Action**:
Write 32-byte pattern (0x00 to 0x1F)

**Expected Transaction**:
```
START → 0xA0 → [0x00, 0x01, ..., 0x1F] → STOP
(Single transaction, all 32 bytes)
```

**AST1060 Console Output**:
```
Buffer Mode Write: addr=0x50, len=32
✓ Using buffer mode (single transaction)
✓ Write successful
```

**Aardvark Display**:
```
I2C Slave Received (32 bytes): [00 01 02 ... 1E 1F]
```

**Success Criteria**: All 32 bytes received in one transaction

---

### Test 6: Bus Error Recovery

**Goal**: Test timeout and bus recovery mechanism

**Aardvark Setup**:
1. **Simulate bus hang**: Hold SDA low manually or via Aardvark GPIO
2. Or disable Aardvark slave to cause NACK

**Expected Behavior**:
```
Write attempt to 0x7F (non-existent address)
✗ Error: NoAcknowledge or Timeout
✓ Bus recovery initiated
✓ Bus functional after recovery
```

**AST1060 Console Output**:
```
Error Test: Writing to invalid address 0x7F
✗ Error detected (expected): NoAcknowledge
✓ Bus recovery successful
✓ Next transaction successful
```

**Success Criteria**: Error detected, recovery works, next transaction succeeds

---

### Test 7: Speed Verification

**Goal**: Confirm 400 kHz Fast mode operation

**Aardvark Setup**:
1. Connect Aardvark as analyzer mode
2. Capture I2C traffic on logic analyzer

**Measurement**:
Use Aardvark Data Center or oscilloscope to measure SCL frequency

**Expected**:
- SCL frequency: ~400 kHz (±10%)
- Clock duty cycle: ~50%
- Valid I2C timing (setup/hold times met)

**Success Criteria**: Measured frequency is 360-440 kHz

---

## Automated Test Script

For automated testing with Aardvark Python API:

```python
#!/usr/bin/env python3
"""
Automated AST1060 I2C Bringup Test
Requires: aardvark_py library
"""

from aardvark_py import *
import time

# Open Aardvark device
handle = aa_open(0)  # First Aardvark device
if handle <= 0:
    print("Error: Cannot open Aardvark")
    exit(1)

# Configure as I2C slave at 0x50
aa_i2c_slave_enable(handle, 0x50, 1024, 1024)
aa_target_power(handle, AA_TARGET_POWER_BOTH)  # Enable pull-ups

print("✓ Aardvark configured as I2C slave at 0x50")
print("  Waiting for AST1060 transactions...")

# Test 1: Receive single byte write
print("\nTest 1: Single Byte Write")
time.sleep(2)
(count, addr, data) = aa_i2c_slave_read(handle, 100)
if count == 1 and data[0] == 0x42:
    print("  ✓ PASS: Received 0x42")
else:
    print(f"  ✗ FAIL: Expected [0x42], got {data[:count]}")

# Test 2: Send single byte for read
print("\nTest 2: Single Byte Read")
aa_i2c_slave_set_response(handle, [0xAB])
time.sleep(2)
print("  ✓ Loaded response: 0xAB")

# Test 3: Write-Read transaction
print("\nTest 3: Write-Read")
aa_i2c_slave_set_response(handle, [0xDE, 0xAD, 0xBE, 0xEF])
time.sleep(2)
print("  ✓ Loaded 4-byte response")

# Cleanup
aa_close(handle)
print("\n✓ All tests complete")
```

**Run**: `python3 aardvark_test.py` while AST1060 is running

---

## Expected Results Summary

| Test | Pass Criteria | Typical Time |
|------|--------------|--------------|
| Bus Scan | Device 0x50 detected | < 1s |
| Single Write | Aardvark receives 0x42 | < 1s |
| Single Read | AST1060 reads 0xAB | < 1s |
| Write-Read | 4-byte sequence correct | < 1s |
| Buffer Mode | 32 bytes received | < 1s |
| Error Recovery | Bus recovered after error | 2-3s |
| Speed Check | 400 kHz ±10% | 5s |

**Total bringup time**: ~10-15 minutes

---

## Troubleshooting

### No Device Detected

**Symptom**: AST1060 reports "No ACK from device"

**Check**:
1. Verify wiring: SCL → SCL, SDA → SDA
2. Check GND connection
3. Verify Aardvark slave mode enabled (not master)
4. Check pull-up resistors (measure 3.3V on idle SCL/SDA)
5. Verify AST1060 firmware is running (check UART output)

### Wrong Data Received

**Symptom**: Data mismatch between sent and received

**Check**:
1. Verify Aardvark slave address matches (0x50 = 0xA0/0xA1 with R/W bit)
2. Check for noise on I2C lines (add stronger pull-ups)
3. Reduce I2C speed if errors persist
4. Check for proper ground reference

### Bus Hangs / Timeouts

**Symptom**: Transactions timeout or hang

**Check**:
1. Enable SMBus timeout in I2C config (should be default)
2. Verify Aardvark is powered and responding
3. Check for stuck SDA/SCL (both should be high when idle)
4. Test bus recovery manually via console command

### Incorrect Speed

**Symptom**: Measured speed not 400 kHz

**Check**:
1. Verify I2C clock configuration in AST1060 chip.toml
2. Check if Aardvark pull-ups are too weak (causing RC delay)
3. Verify AST1060 system clock (HPLL) is configured correctly

---

## Next Steps After Bringup

Once basic tests pass:

1. **Test all 14 controllers** (I2C0-I2C13) - repeat tests on each
2. **Slave mode testing** - Configure AST1060 as slave, Aardvark as master
3. **MCTP testing** - Run MCTP echo app with bidirectional communication
4. **Stress testing** - Continuous transfers, error injection, timing tests
5. **Real device integration** - Connect actual I2C sensors/peripherals

---

## Reference Documents

- [AST1060 I2C Driver README](drv/ast1060-i2c/README.md)
- [Detailed Test Plan](drv/ast1060-i2c/TEST-PLAN.md)
- [Hardware Setup Guide](drv/ast1060-i2c/HARDWARE-SETUP.md)
- [Aardvark User Manual](https://www.totalphase.com/support/articles/200349176/)
