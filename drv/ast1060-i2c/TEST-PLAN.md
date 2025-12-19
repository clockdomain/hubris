# AST1060 I2C Driver Test Plan

## Test Equipment

- **AST1060 Hardware**: Target device running the I2C driver
- **Aardvark I2C/SPI Host Adapter**: External I2C master/slave device for testing
- **UART Connection**: For debug output and test status

## Test Categories

### 1. Master Mode Tests (AST1060 as Master, Aardvark as Slave)

#### 1.1 Basic Write Operations
**Setup**: Configure Aardvark as I2C slave at address 0x50

| Test | Description | Expected Result |
|------|-------------|-----------------|
| Write Single Byte | Write 1 byte to slave | ACK received, Aardvark receives correct byte |
| Write Multiple Bytes | Write 2-10 bytes | All bytes received in order |
| Write 32 Bytes (Buffer Mode) | Write exactly 32 bytes | Buffer mode used, all bytes correct |
| Write 64 Bytes (Chunked) | Write 64 bytes | Auto-chunked into 2x32 byte transfers |
| Write 100 Bytes | Write 100 bytes | Chunked into 3x32 + 4 bytes, all correct |

#### 1.2 Basic Read Operations
**Setup**: Aardvark pre-loaded with known data pattern

| Test | Description | Expected Result |
|------|-------------|-----------------|
| Read Single Byte | Read 1 byte from slave | Correct byte received |
| Read Multiple Bytes | Read 2-10 bytes | Correct sequence received |
| Read 32 Bytes | Read 32 bytes | Buffer mode used, correct data |
| Read 64 Bytes | Read 64 bytes | Auto-chunked, all data correct |

#### 1.3 Write-Read Operations
**Setup**: Combined write-then-read (register access pattern)

| Test | Description | Expected Result |
|------|-------------|-----------------|
| Write 1, Read 1 | Write register address, read value | Correct value returned |
| Write 1, Read 32 | Write command, read 32-byte response | Full response correct |
| Write 2, Read 16 | Write 16-bit address, read block | Correct block returned |

#### 1.4 Error Handling

| Test | Description | Expected Result |
|------|-------------|-----------------|
| No ACK | Write to non-existent address 0x7F | Timeout/NoAcknowledge error |
| Bus Busy | Aardvark holds SDA low | Bus recovery triggered |
| Clock Stretching | Aardvark stretches clock >1ms | Transaction completes or timeout |
| Arbitration | Two masters (if possible) | Arbitration loss detected |

#### 1.5 Multiple Controllers
**Setup**: Test all 14 I2C controllers

| Test | Description | Expected Result |
|------|-------------|-----------------|
| Controller 0 | Basic read/write on I2C0 | Works |
| Controller 1-13 | Basic read/write on each | All controllers functional |
| Concurrent Access | Multiple tasks request different controllers | No conflicts |

### 2. Slave Mode Tests (AST1060 as Slave, Aardvark as Master)

#### 2.1 Basic Slave Configuration

| Test | Description | Expected Result |
|------|-------------|-----------------|
| Configure Slave Address | Set AST1060 as slave at 0x1D | Configuration succeeds |
| Enable Slave Mode | Enable slave receive | Mode activated |
| Address Match | Aardvark addresses 0x1D | AST1060 ACKs |
| Wrong Address | Aardvark addresses 0x1E | AST1060 does not ACK |

#### 2.2 Slave Receive (Master writes to us)

| Test | Description | Expected Result |
|------|-------------|-----------------|
| Receive 1 Byte | Aardvark writes 1 byte | Byte received correctly |
| Receive 10 Bytes | Aardvark writes 10 bytes | All bytes received in order |
| Receive 32 Bytes | Aardvark writes 32 bytes (buffer full) | All bytes received |
| Receive 64 Bytes | Aardvark writes 64 bytes | Data received (may need chunking) |
| Receive with STOP | Normal transaction with STOP | Stop event detected |

#### 2.3 Slave Transmit (Master reads from us)

| Test | Description | Expected Result |
|------|-------------|-----------------|
| Transmit 1 Byte | Pre-load 1 byte, Aardvark reads | Byte transmitted correctly |
| Transmit 10 Bytes | Pre-load 10 bytes, Aardvark reads | All bytes transmitted |
| Transmit 32 Bytes | Pre-load 32 bytes buffer | Full buffer transmitted |
| Read Request No Data | Aardvark reads but no data ready | NACK or 0xFF returned |

#### 2.4 Slave Interrupts & Events

| Test | Description | Expected Result |
|------|-------------|-----------------|
| Address Match Event | Monitor address match interrupt | Event fired on addressing |
| RX Done Event | Monitor receive complete | Event fired after data received |
| TX Done Event | Monitor transmit complete | Event fired after data sent |
| Stop Event | Monitor STOP condition | Event fired on STOP |
| Error Conditions | Invalid conditions | Error events detected |

#### 2.5 Slave Notification System

| Test | Description | Expected Result |
|------|-------------|-----------------|
| Enable Notification | Enable interrupt notification | Notification mask set |
| Receive Triggers Notify | Aardvark writes data | Task receives notification |
| Get Slave Message | Call get_slave_message() | Message retrieved correctly |
| Multiple Messages | Queue multiple messages | All messages retrieved in order |
| Disable Notification | Disable notifications | No further notifications sent |

### 3. I2C Multiplexer Tests

#### 3.1 PCA9545 (4-channel mux)
**Setup**: PCA9545 at 0x70, slaves on each channel

| Test | Description | Expected Result |
|------|-------------|-----------------|
| Select Channel 0 | Select and access device on ch0 | Device accessible |
| Select Channel 1-3 | Select each channel sequentially | Each channel works |
| Channel Isolation | Access device on ch0, verify ch1 isolated | Isolation confirmed |
| Auto-select | Driver auto-selects channel | Transparent to user |

#### 3.2 PCA9548 (8-channel mux)
**Setup**: PCA9548 at 0x71, slaves on channels

| Test | Description | Expected Result |
|------|-------------|-----------------|
| All 8 Channels | Test all channels 0-7 | All channels functional |
| Multiple Muxes | Two muxes on same bus | Both work independently |

### 4. Timing & Performance Tests

| Test | Description | Expected Result |
|------|-------------|-----------------|
| 100 kHz Standard Mode | Set bus to 100 kHz | Aardvark measures ~100 kHz |
| 400 kHz Fast Mode | Set bus to 400 kHz | Aardvark measures ~400 kHz |
| 1 MHz Fast Mode Plus | Set bus to 1 MHz | Aardvark measures ~1 MHz |
| Throughput | Transfer 1KB of data | Measure transfer rate |
| Latency | Measure request-to-completion time | Record latency |

### 5. Bus Recovery Tests

| Test | Description | Expected Result |
|------|-------------|-----------------|
| SDA Stuck Low | Aardvark holds SDA low | Recovery successful (9 clocks) |
| SCL Stuck Low | Simulate SCL stuck | Recovery attempted |
| Recovery Count | Multiple stuck conditions | Each recovery succeeds |

### 6. Integration Tests

#### 6.1 Real I2C Devices
Test with actual I2C devices (if available):

- **EEPROM (24C32)**: Read/write operations
- **RTC (DS3231)**: Read time, set time
- **Temp Sensor (TMP75)**: Read temperature
- **GPIO Expander (PCA9555)**: Set/read pins

#### 6.2 MCTP Protocol
**Setup**: Aardvark running MCTP master, AST1060 as MCTP slave

| Test | Description | Expected Result |
|------|-------------|-----------------|
| MCTP Message RX | Receive MCTP control message | Message parsed correctly |
| MCTP Response TX | Send MCTP response | Response transmitted |
| MCTP Multi-Packet | Receive fragmented MCTP message | Reassembly works |

### 7. Stress Tests

| Test | Description | Expected Result |
|------|-------------|-----------------|
| Continuous Transfers | 1000 consecutive transfers | No errors |
| Rapid Direction Changes | Alternate read/write rapidly | All operations succeed |
| All Controllers Active | Simultaneous traffic on all 14 buses | No interference |
| Long Running | 24-hour continuous operation | No hangs or errors |

## Test Execution Priority

### Phase 1: Smoke Tests (Quick validation)
1. Master write single byte
2. Master read single byte
3. Slave receive single byte
4. Slave transmit single byte

### Phase 2: Core Functionality
1. All master mode tests (1.1-1.3)
2. Basic slave mode tests (2.1-2.2)
3. Error handling (1.4)

### Phase 3: Advanced Features
1. Slave interrupts and notifications (2.4-2.5)
2. Multiplexer support (3.1-3.2)
3. Multiple controllers (1.5)

### Phase 4: Validation
1. Timing tests (4)
2. Bus recovery (5)
3. Integration with real devices (6)
4. Stress tests (7)

## Test Harness Setup

### Aardvark Configuration

```python
# Example Aardvark setup for slave mode testing
from aardvark_py import *

handle = aa_open(0)  # Open first Aardvark
aa_configure(handle, AA_CONFIG_SPI_I2C)
aa_i2c_pullup(handle, AA_I2C_PULLUP_BOTH)

# Configure as slave at 0x50 for master mode tests
aa_i2c_slave_enable(handle, 0x50, 256, 256)

# Pre-load data for read tests
data = [0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07]
aa_i2c_slave_set_response(handle, data)
```

### AST1060 Test Application

Extend `task/i2c-client/src/main.rs` with:
- Test orchestration logic
- UART output for results
- Test result reporting
- Automated test sequencing

### Physical Connections

```
AST1060 I2C0    ←→    Aardvark
  SDA           ←→    SDA (pin 5)
  SCL           ←→    SCL (pin 7)
  GND           ←→    GND (pin 3,4)
  
Pull-ups: 2.2kΩ to 3.3V (on Aardvark or AST1060 board)
```

## Success Criteria

- ✅ All Phase 1 & 2 tests pass
- ✅ At least 90% of Phase 3 tests pass
- ✅ No bus hangs or deadlocks
- ✅ Error conditions handled gracefully
- ✅ Timing measurements within ±10% of target
- ✅ Slave mode receives and transmits correctly
- ✅ All 14 controllers functional

## Known Limitations

1. **No DMA Support**: All transfers use PIO (programmed I/O)
2. **32-Byte Buffer Limit**: Larger transfers auto-chunk
3. **Single Slave Address**: Only one address per controller in slave mode
4. **No Multi-Master Arbitration**: Limited multi-master testing
5. **No SMBus Packet Error Checking**: PEC not implemented

## Future Test Additions

- Power management (sleep/wake)
- Clock gating effects
- Temperature variation testing
- EMI/noise immunity
- Hot-plug handling
