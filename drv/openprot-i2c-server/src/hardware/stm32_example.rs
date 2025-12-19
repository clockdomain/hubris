//! Example: STM32 I2C Hardware Driver Integration
//!
//! This is a template showing how a vendor would integrate STM32 support.
//! To actually enable STM32:
//! 1. Uncomment the stm32 feature in Cargo.toml
//! 2. Add stm32 module declaration in hardware.rs
//! 3. Implement the actual driver (this is just a skeleton)

#![allow(dead_code)]  // Template code, not yet implemented

use drv_i2c_types::traits::I2cHardware;

// These would be your actual driver types
pub struct Stm32Peripherals {
    // STM32 I2C peripheral instances
}

pub struct Stm32I2cDriver {
    // STM32-specific driver state
}

impl Stm32Peripherals {
    /// # Safety
    /// Must only be called once to ensure exclusive peripheral access
    pub unsafe fn new() -> Self {
        // Take ownership of STM32 I2C peripherals
        Self {
            // Initialize from stm32-pac
        }
    }
}

impl Stm32I2cDriver {
    pub fn new(_peripherals: Stm32Peripherals) -> Self {
        Self {
            // Initialize driver
        }
    }
}

// This is the only required public interface
/// Create an STM32 I2C driver instance
pub fn create_driver() -> impl I2cHardware {
    let peripherals = unsafe { 
        Stm32Peripherals::new() 
    };
    Stm32I2cDriver::new(peripherals)
}

// The vendor would implement I2cHardware trait here
// impl I2cHardware for Stm32I2cDriver { ... }
