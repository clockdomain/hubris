//! AST1060 I2C Driver for Hubris
//!
//! This driver provides I2C controller support for the ASPEED AST1060 SoC.
//! It supports:
//! - Byte mode (1 byte transfers)
//! - Buffer mode (up to 32 byte transfers)
//! - Master mode operations (read, write, write-read)
//! - Optional slave/target mode (with `slave` feature)
//! - 14 I2C controllers (I2C0-I2C13)

#![no_std]

mod constants;
mod controller;
mod error;
mod master;
mod mux;
mod recovery;
mod timing;
mod transfer;

mod server_driver;
mod slave;

pub use constants::*;
pub use controller::*;
pub use error::*;
pub use mux::*;

pub use server_driver::*;
pub use slave::*;

use drv_i2c_api::Controller;

/// I2C controller configuration
pub struct I2cController<'a> {
    pub controller: Controller,
    pub registers: &'a ast1060_pac::i2c::RegisterBlock,
    pub buff_registers: &'a ast1060_pac::i2cbuff::RegisterBlock,
    pub notification: Option<u32>,
}

/// I2C configuration
///
/// Default configuration is optimized for MCTP-over-I2C:
/// - Fast mode (400 kHz) - standard MCTP speed
/// - Buffer mode - efficient for MCTP packet transfers
/// - SMBus timeout enabled - required for robust MCTP operation
#[derive(Debug, Clone, Copy)]
pub struct I2cConfig {
    /// Transfer mode (byte-by-byte or buffer)
    pub xfer_mode: I2cXferMode,
    /// Bus speed (Standard/Fast/FastPlus)
    /// MCTP typically uses Fast mode (400 kHz)
    pub speed: I2cSpeed,
    /// Enable multi-master support
    pub multi_master: bool,
    /// Enable SMBus timeout detection (25-35ms per SMBus spec)
    /// Required for MCTP reliability in multi-master environments
    pub smbus_timeout: bool,
    /// Enable SMBus alert interrupt
    pub smbus_alert: bool,
}

impl Default for I2cConfig {
    fn default() -> Self {
        Self {
            xfer_mode: I2cXferMode::BufferMode,
            speed: I2cSpeed::Fast, // 400 kHz - MCTP standard
            multi_master: false,
            smbus_timeout: true, // Required for MCTP reliability
            smbus_alert: false,
        }
    }
}

/// Transfer mode selection
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum I2cXferMode {
    /// Byte-by-byte mode (1 byte at a time)
    ByteMode,
    /// Buffer mode (up to 32 bytes via hardware buffer)
    BufferMode,
    // Note: DMA mode intentionally excluded for initial version
}

/// I2C bus speed
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum I2cSpeed {
    /// Standard mode: 100 kHz
    Standard,
    /// Fast mode: 400 kHz
    Fast,
    /// Fast-plus mode: 1 MHz
    FastPlus,
}
