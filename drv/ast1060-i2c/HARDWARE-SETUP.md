# AST1060 I2C Test Setup - Aardvark Connections

## Hardware Wiring Diagram

```
┌─────────────────────────────────────────────────────────────────────────┐
│                         AST1060 Development Board                        │
│                                                                           │
│  ┌──────────────────────────────────────────────────────────────────┐   │
│  │                                                                  │   │
│  │  I2C Controller 0 (I2C0)                                         │   │
│  │                                                                  │   │
│  │    Pin 1: SDA0 ────────────────────┐                            │   │
│  │    Pin 2: SCL0 ──────────────────┐ │                            │   │
│  │    Pin 3: GND ─────────────────┐ │ │                            │   │
│  │                                │ │ │                            │   │
│  └────────────────────────────────┼─┼─┼────────────────────────────┘   │
│                                   │ │ │                                 │
│  ┌────────────────────────────────┼─┼─┼────────────────────────────┐   │
│  │  Optional Pull-up Resistors    │ │ │                            │   │
│  │                                │ │ │                            │   │
│  │    2.2kΩ to 3.3V ──────┬───────┘ │                             │   │
│  │                        │         │                              │   │
│  │    2.2kΩ to 3.3V ──────┼─────────┘                              │   │
│  │                        │                                         │   │
│  └────────────────────────┼─────────────────────────────────────────   │
│                           │                                             │
└───────────────────────────┼─────────────────────────────────────────────┘
                            │
                            │  (Use pull-ups on EITHER AST1060 OR Aardvark,
                            │   not both - Aardvark has built-in pull-ups)
                            │
                ┌───────────┼───────────────────────────────────────┐
                │           │                                       │
                │           │     Aardvark I2C/SPI Host Adapter     │
                │           │                                       │
                │           │  ┌────────────────────────────┐       │
                │           │  │                            │       │
                │           │  │   Pin 1: SCL (Red)   ◄─────┼───────┘
                │           │  │   Pin 2: GND (Blk)   ◄─────┼───────┐
                │           │  │   Pin 3: SDA (Grn)   ◄─────┘       │
                │           │  │   Pin 4: +3.3V               │
                │           │  │                              │
                │           │  │   Connector: 10-pin          │
                │           │  └──────────────────────────────┘
                │           │                                       │
                │           │  USB Cable to Host PC                 │
                │           └───────────────────────────────────────┤
                │                       │                           │
                └───────────────────────┼───────────────────────────┘
                                        │
                                        ▼
                            ┌───────────────────────┐
                            │     Host PC           │
                            │                       │
                            │  - Aardvark Software  │
                            │  - Python Scripts     │
                            │  - Control Center GUI │
                            └───────────────────────┘


Alternative: UART Debug Connection (Separate from I2C)
──────────────────────────────────────────────────────

AST1060 UART0                         USB-to-Serial Adapter
┌─────────────────┐                   ┌──────────────────┐
│  TX  ────────────┼───────────────────┤ RX               │
│  RX  ────────────┼───────────────────┤ TX               │
│  GND ────────────┼───────────────────┤ GND              │
└─────────────────┘                   └──────────────────┘
                                              │
                                              │ USB
                                              ▼
                                      ┌───────────────┐
                                      │   Host PC     │
                                      │   Terminal    │
                                      │   (115200 8N1)│
                                      └───────────────┘
```

## Detailed Pin Connections

### Aardvark to AST1060 I2C

| Aardvark Pin | Signal | Color | AST1060 Pin | Notes |
|--------------|--------|-------|-------------|-------|
| 1            | SCL    | Red   | I2C0_SCL    | Clock line |
| 2            | GND    | Black | GND         | Common ground |
| 3            | SDA    | Green | I2C0_SDA    | Data line |
| 4            | +3.3V  | Orange| *Optional*  | Only if powering devices |
| 5-10         | NC     | -     | -           | Not connected |

### Pull-up Resistor Configuration

**Option 1: Use Aardvark Built-in Pull-ups (Recommended)**
```
No external resistors needed.
Enable in software:
  aa_i2c_pullup(handle, AA_I2C_PULLUP_BOTH)
```

**Option 2: Use AST1060 Board Pull-ups**
```
2.2kΩ resistors from SDA/SCL to 3.3V on AST1060 board.
Disable Aardvark pull-ups:
  aa_i2c_pullup(handle, AA_I2C_PULLUP_NONE)
```

**Option 3: External Pull-ups**
```
Connect external 2.2kΩ resistors:
  SDA ──[2.2kΩ]── +3.3V
  SCL ──[2.2kΩ]── +3.3V
```

## Test Configuration Examples

### Configuration 1: Master Mode Testing
**AST1060 = Master, Aardvark = Slave**

```
Physical:
  AST1060 I2C0 ←→ Aardvark

Aardvark Setup:
  aa_i2c_slave_enable(handle, 0x50, 256, 256)
  # Aardvark responds as slave at address 0x50

AST1060 Setup:
  // Write to Aardvark
  i2c.write(0x50, &data)?;
  // Read from Aardvark
  i2c.read(0x50, &mut buffer)?;
```

### Configuration 2: Slave Mode Testing
**AST1060 = Slave, Aardvark = Master**

```
Physical:
  AST1060 I2C0 ←→ Aardvark

AST1060 Setup:
  device.configure_slave_address(0x1D)?;
  device.enable_slave_receive()?;
  // AST1060 listens as slave at address 0x1D

Aardvark Setup:
  aa_i2c_write(handle, 0x1D, data, len)
  # Aardvark sends data to AST1060 slave
```

### Configuration 3: Multi-Controller Testing
**Test multiple I2C controllers simultaneously**

```
Physical Connections:
  AST1060 I2C0  ←→ Aardvark 1 (or same Aardvark, test sequentially)
  AST1060 I2C1  ←→ Aardvark 2 (if available)
  AST1060 I2C2  ←→ Aardvark 3 (if available)
  ...or test each controller one at a time

Note: You can test all 14 controllers with one Aardvark by
      reconnecting the wires for each test.
```

### Configuration 4: I2C Multiplexer Testing
**AST1060 + PCA9545 + Multiple Slaves**

```
                 AST1060 I2C0
                      │
                      │ SDA/SCL
                      │
                 ┌────▼────┐
                 │ PCA9545 │  (I2C Mux at 0x70)
                 │  4-Ch   │
                 └─┬─┬─┬─┬─┘
                   │ │ │ │
       ┌───────────┘ │ │ └─────────┐
       │             │ │           │
     CH0           CH1 CH2        CH3
       │             │ │           │
   Aardvark      Device Device  Device
   (0x50)        (0x51) (0x52)  (0x53)

Test: Verify channel switching and device isolation
```

## Logic Analyzer Connections (Optional)

For debugging, add a logic analyzer:

```
AST1060 I2C0          Logic Analyzer (8 channels)
┌──────────┐          ┌─────────────────┐
│ SDA ─────┼──────────┤ CH0 (SDA)       │
│ SCL ─────┼──────────┤ CH1 (SCL)       │
│ INT ─────┼──────────┤ CH2 (Interrupt) │  (if testing interrupts)
│ GPIO ────┼──────────┤ CH3 (Debug)     │  (if using debug pins)
│ GND ─────┼──────────┤ GND             │
└──────────┘          └─────────────────┘

Recommended: Saleae Logic 8, 24 MHz sampling
```

## Power Considerations

### AST1060 Power
- Powered via development board (USB or external supply)
- Typical: 3.3V @ 200mA

### Aardvark Power
- USB-powered from Host PC
- Can supply +3.3V on pin 4 (up to 50mA)
- **Do NOT power AST1060 from Aardvark**

### I2C Bus Voltage
- AST1060: 3.3V I/O
- Aardvark: 1.8V - 5.5V compatible
- **Use 3.3V for both devices**

## Safety Checklist

- [ ] Ground connections established first
- [ ] Pull-ups enabled (Aardvark OR board, not both)
- [ ] Voltage levels matched (3.3V)
- [ ] No short circuits on SDA/SCL
- [ ] USB connections secure
- [ ] Firmware loaded on AST1060
- [ ] Aardvark drivers installed on PC

## Common Issues & Troubleshooting

### Issue: No ACK from device
```
Check:
  ✓ Pull-up resistors enabled
  ✓ Correct device address
  ✓ Ground connection secure
  ✓ Wires connected to correct pins
```

### Issue: Bus appears stuck
```
Check:
  ✓ No other devices holding lines low
  ✓ Pull-up resistors not too weak (< 10kΩ)
  ✓ Cable length < 30cm for testing
  ✓ Try bus recovery function
```

### Issue: Intermittent communication
```
Check:
  ✓ Loose connections
  ✓ Pull-up resistor value (try 2.2kΩ)
  ✓ Cable quality
  ✓ Nearby noise sources
```

## Software Setup Summary

```python
# Aardvark Python setup
from aardvark_py import *

# Open device
handle = aa_open(0)
if handle <= 0:
    print("Failed to open Aardvark")
    exit(1)

# Configure I2C mode
aa_configure(handle, AA_CONFIG_SPI_I2C)

# Enable pull-ups
aa_i2c_pullup(handle, AA_I2C_PULLUP_BOTH)

# Set bitrate (100 kHz, 400 kHz, or 1000 kHz)
aa_i2c_bitrate(handle, 400)  # 400 kHz

# Configure as slave for master mode testing
aa_i2c_slave_enable(handle, 0x50, 256, 256)

print("Aardvark ready!")
```

## Next Steps

1. **Connect hardware** following the diagram above
2. **Verify connections** with a multimeter (continuity test)
3. **Run Aardvark Control Center** to verify device is detected
4. **Load test firmware** on AST1060
5. **Run Phase 1 smoke tests** from TEST-PLAN.md
