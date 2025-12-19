# OpenProt I2C Server - Hardware Vendor Integration Guide

This I2C server provides a vendor-agnostic implementation that works with any hardware implementing the `I2cHardware` trait. Hardware vendors can integrate their platform **without modifying the main server code**.

## Architecture

```
drv-openprot-i2c-server/
├── src/
│   ├── main.rs              ← Generic server (DO NOT MODIFY)
│   ├── hardware.rs          ← Hardware dispatcher (add your feature here)
│   ├── hardware/
│   │   ├── ast1060.rs       ← AST1060 integration (reference implementation)
│   │   └── <your-chip>.rs   ← Your hardware integration (add here)
│   └── hardware_driver.rs   ← AST1060 driver (rename/duplicate for your chip)
└── Cargo.toml               ← Add your dependencies and feature flag
```

## Integration Steps for New Hardware

### 1. Create Your Hardware Driver Module

Create `src/hardware/<your-chip>.rs`:

```rust
//! <YourChip> I2C Hardware Driver Integration

use drv_i2c_types::traits::I2cHardware;

// Import your hardware-specific types
pub use crate::<your_chip>_driver::{YourPeripherals, YourI2cDriver};

/// Create your chip's I2C driver instance
pub fn create_driver() -> impl I2cHardware {
    let peripherals = unsafe { 
        YourPeripherals::new() 
    };
    YourI2cDriver::new(peripherals)
}
```

### 2. Implement Your Hardware Driver

Create `src/<your_chip>_driver.rs` that implements the `I2cHardware` trait from `drv-i2c-types`:

```rust
use drv_i2c_types::traits::I2cHardware;

pub struct YourI2cDriver {
    // Your hardware-specific state
}

impl I2cHardware for YourI2cDriver {
    type Error = ResponseCode;
    
    fn write_read(&mut self, controller: Controller, addr: u8, 
                  write_data: &[u8], read_buffer: &mut [u8]) 
        -> Result<usize, Self::Error> {
        // Your implementation
    }
    
    // Implement other required methods...
}
```

See `drv/i2c-types/src/traits.rs` for the complete trait definition.

### 3. Add Feature Flag to hardware.rs

Edit `src/hardware.rs` to add your chip:

```rust
// Add at the top with other vendors
#[cfg(feature = "your-chip")]
pub mod your_chip;

// Add to create_driver() function
#[cfg(feature = "your-chip")]
pub fn create_driver() -> impl I2cHardware {
    your_chip::create_driver()
}
```

### 4. Update Cargo.toml

Add your dependencies and feature:

```toml
[dependencies]
# Add your hardware dependencies (optional = true)
drv-your-chip-i2c = { path = "../your-chip-i2c", optional = true }
your-chip-pac = { ... , optional = true }

[features]
# Add your hardware vendor feature
your-chip = ["drv-your-chip-i2c", "your-chip-pac"]
```

### 5. Create Application Configuration

Create `app/your-app/app.toml`:

```toml
[tasks.i2c]
name = "drv-openprot-i2c-server"
priority = 3
features = ["your-chip", "slave"]  # Enable your hardware
uses = ["i2c0", ...]  # Your peripheral definitions
```

## Reference Implementation

The **AST1060** integration serves as a complete reference:
- `src/hardware/ast1060.rs` - Integration module
- `src/hardware_driver.rs` - Full driver implementation
- `app/ast1060-i2c-example/app.toml` - Application config

## Required Trait Methods

Your driver must implement:

### Master Mode
- `write_read()` - Combined write then read transaction
- `write_read_block()` - Block mode transactions

### Slave Mode (Optional)
- `configure_slave_mode()` - Set slave address
- `enable_slave_receive()` - Start receiving as slave
- `disable_slave_receive()` - Stop slave mode

### Management
- `configure_timing()` - Set I2C speed/timing
- `reset_bus()` - Recover from bus errors
- `enable_controller()` - Power on controller
- `disable_controller()` - Power off controller

## Building

```bash
# Build with your hardware
cargo xtask dist app/your-app/app.toml

# Or specify feature directly
cargo build --features your-chip,slave
```

## Benefits of This Architecture

✅ **No main.rs changes** - Server code stays clean and vendor-agnostic  
✅ **Compile-time dispatch** - Zero runtime overhead  
✅ **Type safety** - Trait ensures all vendors provide required functionality  
✅ **Modularity** - Each vendor's code is isolated  
✅ **Feature flags** - Easy to select target hardware at build time  

## Support

For questions about integration, refer to:
- `drv/i2c-types/src/traits.rs` - Complete trait documentation
- `src/hardware/ast1060.rs` - Reference implementation
- AST1060 driver in `drv/ast1060-i2c/` - Full example

## Current Supported Hardware

- **AST1060** (ASPEED) - Reference implementation ✓

*Add your chip here!*
