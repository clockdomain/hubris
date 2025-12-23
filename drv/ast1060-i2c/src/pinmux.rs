// Copyright 2024 Advanced Micro Devices, Inc.
// SPDX-License-Identifier: Apache-2.0

//! AST1060 I2C Pin Multiplexing Configuration
//!
//! The AST1060 I2C pins must be configured via SCU (System Control Unit) registers
//! before the I2C controllers can communicate with external devices. Each I2C bus
//! requires 2 pins (SCL and SDA) to be configured.
//!
//! Pin mapping:
//! - I2C0-1: SCU414 bits 28-31
//! - I2C2-7: SCU418 bits 0-11

use crate::error::I2cError;
use drv_i2c_api::Controller;

/// Configure I2C pins for a specific controller
///
/// This must be called before `init_hardware()` to enable the physical
/// I2C pins on the chip. Without this, the I2C signals won't reach the
/// external pins and communication will fail.
///
/// Currently only supports I2C0-7.
///
/// # Arguments
/// * `controller` - Which I2C controller to configure pins for
///
/// # Safety
/// This function directly accesses SCU registers. It should only be called
/// once per controller during initialization.
pub unsafe fn configure_i2c_pins(controller: Controller) -> Result<(), I2cError> {
    let scu = &*ast1060_pac::Scu::ptr();
    
    match controller {
        Controller::I2C0 => {
            // SCU414[28:29] = I2C0 SCL/SDA
            scu.scu414().modify(|r, w| {
                w.bits(r.bits() | (0b11 << 28))
            });
        }
        Controller::I2C1 => {
            // SCU414[30:31] = I2C1 SCL/SDA
            scu.scu414().modify(|r, w| {
                w.bits(r.bits() | (0b11 << 30))
            });
        }
        Controller::I2C2 => {
            // SCU418[0:1] = I2C2 SCL/SDA
            scu.scu418().modify(|r, w| {
                w.bits(r.bits() | (0b11 << 0))
            });
        }
        Controller::I2C3 => {
            // SCU418[2:3] = I2C3 SCL/SDA
            scu.scu418().modify(|r, w| {
                w.bits(r.bits() | (0b11 << 2))
            });
        }
        Controller::I2C4 => {
            // SCU418[4:5] = I2C4 SCL/SDA
            scu.scu418().modify(|r, w| {
                w.bits(r.bits() | (0b11 << 4))
            });
        }
        Controller::I2C5 => {
            // SCU418[6:7] = I2C5 SCL/SDA
            scu.scu418().modify(|r, w| {
                w.bits(r.bits() | (0b11 << 6))
            });
        }
        Controller::I2C6 => {
            // SCU418[8:9] = I2C6 SCL/SDA
            scu.scu418().modify(|r, w| {
                w.bits(r.bits() | (0b11 << 8))
            });
        }
        Controller::I2C7 => {
            // SCU418[10:11] = I2C7 SCL/SDA
            scu.scu418().modify(|r, w| {
                w.bits(r.bits() | (0b11 << 10))
            });
        }
    }
    
    Ok(())
}
