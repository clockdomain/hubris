//! Hardware-specific driver selection
//!
//! The server is generic over I2cHardware trait. Vendor-specific drivers
//! live in their own crates (drv-ast1060-i2c, drv-stm32-i2c, etc.) and
//! just need to implement I2cHardware.

use drv_i2c_types::traits::I2cHardware;

/// Create the hardware-specific I2C driver instance
///
/// This function imports and constructs the appropriate driver based on
/// enabled feature flags. Vendor drivers must:
/// - Implement the I2cHardware trait
/// - Provide a constructor (typically Driver::new() or from PAC)
///
/// # Returns
///
/// An opaque type implementing I2cHardware
#[cfg(feature = "ast1060")]
pub fn create_driver() -> impl I2cHardware {
    use drv_ast1060_i2c::{Ast1060I2cDriver, I2cPeripherals};

    // Safety: Task owns I2C peripherals exclusively per app.toml
    let peripherals = unsafe { I2cPeripherals::new() };
    Ast1060I2cDriver::new(peripherals)
}

// Compile-time check: exactly one hardware vendor must be selected
#[cfg(not(any(feature = "ast1060")))]
compile_error!("No hardware vendor feature enabled. Enable one of: ast1060");
