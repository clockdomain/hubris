//! Hardware-specific driver selection
//! 
//! This module provides a vendor-agnostic interface for creating I2C drivers.
//! Hardware vendors integrate their implementation by:
//! 1. Creating a module under hardware/<vendor>/
//! 2. Implementing create_driver() that returns their driver type
//! 3. Adding a feature flag and conditional compilation
//!
//! The main server code remains unchanged - vendors only add their module.

use drv_i2c_types::traits::I2cHardware;

// AST1060 (ASPEED) hardware implementation
#[cfg(feature = "ast1060")]
pub mod ast1060;

/// Create the hardware-specific I2C driver instance
/// 
/// This function dispatches to the appropriate vendor implementation based
/// on the enabled feature flag. Exactly one hardware vendor feature must be enabled.
/// 
/// # Returns
/// 
/// An instance of the hardware-specific driver that implements `I2cHardware` trait.
/// 
/// # Safety
/// 
/// Must only be called once per task. The driver takes ownership of hardware
/// peripherals for its lifetime.
#[cfg(feature = "ast1060")]
pub fn create_driver() -> impl I2cHardware {
    ast1060::create_driver()
}

// Compile-time check: exactly one hardware vendor must be selected
#[cfg(not(any(feature = "ast1060")))]
compile_error!("No hardware vendor feature enabled. Enable one of: ast1060");

// Future vendor integrations would add:
// #[cfg(feature = "stm32")]
// pub mod stm32;
//
// #[cfg(feature = "lpc55")]
// pub mod lpc55;
//
// And update create_driver() with additional #[cfg] branches
