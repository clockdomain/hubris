//! AST1060 (ASPEED) I2C Hardware Driver Integration
//!
//! This module provides the AST1060-specific implementation of the I2cHardware trait.
//! It serves as the integration layer between the generic openprot-i2c-server and
//! the AST1060 hardware driver.
//!
//! # Vendor Integration Pattern
//!
//! This file demonstrates the pattern that other hardware vendors should follow:
//! 1. Import the vendor-specific PAC and driver crates
//! 2. Re-export any vendor-specific types needed
//! 3. Implement `create_driver()` to instantiate the hardware driver
//!
//! The `create_driver()` function is the only required interface - everything else
//! is vendor-specific implementation details.

use drv_i2c_types::traits::I2cHardware;

// Import AST1060-specific implementation
pub use crate::hardware_driver::{I2cPeripherals, Ast1060I2cDriver};

/// Create an AST1060 I2C driver instance
/// 
/// This function initializes the AST1060 I2C hardware by:
/// 1. Taking ownership of the I2C peripheral registers
/// 2. Creating the driver instance with proper initialization
/// 
/// # Safety
/// 
/// This function uses `unsafe` internally to access hardware peripherals.
/// It must only be called once per task to ensure exclusive peripheral access.
/// In the Hubris model, each task owns its peripherals exclusively, making this safe.
/// 
/// # Returns
/// 
/// An initialized `Ast1060I2cDriver` that implements the `I2cHardware` trait.
/// 
/// # Example
/// 
/// ```rust,ignore
/// // Called once at task startup
/// let driver = ast1060::create_driver();
/// // Driver is now ready for I2C operations
/// ```
pub fn create_driver() -> impl I2cHardware {
    // Safety: In Hubris, each task has exclusive access to its peripherals
    // defined in the app.toml file. This task owns the I2C peripherals.
    let peripherals = unsafe { 
        I2cPeripherals::new() 
    };
    
    Ast1060I2cDriver::new(peripherals)
}
