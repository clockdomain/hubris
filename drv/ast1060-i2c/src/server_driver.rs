// Copyright 2024 Advanced Micro Devices, Inc.
// SPDX-License-Identifier: Apache-2.0

//! AST1060 Hardware I2C Driver Integration
//!
//! This module implements the I2cHardware trait for the AST1060, bridging
//! the low-level ast1060-i2c driver with the generic openprot-i2c-server.

use crate::*;
use ast1060_pac;
use drv_i2c_api::*;
use drv_i2c_types::{traits::I2cHardware, ResponseCode, SlaveConfig};

/// Safe wrapper for AST1060 I2C peripherals
///
/// This type owns the peripherals and provides safe access to all 14 I2C controllers.
/// By keeping the peripherals alive, we ensure all borrowed references remain valid.
pub struct I2cPeripherals {
    peripherals: ast1060_pac::Peripherals,
}

impl I2cPeripherals {
    /// Create a new I2C peripherals wrapper
    ///
    /// # Safety
    /// Must only be called once. Caller must ensure no other code accesses I2C peripherals.
    pub unsafe fn new() -> Self {
        Self {
            peripherals: ast1060_pac::Peripherals::steal(),
        }
    }

    /// Get references to a specific I2C controller's registers and buffer
    ///
    /// Returns None if the controller ID is out of range (>13).
    pub fn controller_and_buffer(
        &self,
        id: u8,
    ) -> Option<(
        &ast1060_pac::i2c::RegisterBlock,
        &ast1060_pac::i2cbuff::RegisterBlock,
    )> {
        Some(match id {
            0 => (&self.peripherals.i2c, &self.peripherals.i2cbuff),
            1 => (&self.peripherals.i2c1, &self.peripherals.i2cbuff1),
            2 => (&self.peripherals.i2c2, &self.peripherals.i2cbuff2),
            3 => (&self.peripherals.i2c3, &self.peripherals.i2cbuff3),
            4 => (&self.peripherals.i2c4, &self.peripherals.i2cbuff4),
            5 => (&self.peripherals.i2c5, &self.peripherals.i2cbuff5),
            6 => (&self.peripherals.i2c6, &self.peripherals.i2cbuff6),
            7 => (&self.peripherals.i2c7, &self.peripherals.i2cbuff7),
            8 => (&self.peripherals.i2c8, &self.peripherals.i2cbuff8),
            9 => (&self.peripherals.i2c9, &self.peripherals.i2cbuff9),
            10 => (&self.peripherals.i2c10, &self.peripherals.i2cbuff10),
            11 => (&self.peripherals.i2c11, &self.peripherals.i2cbuff11),
            12 => (&self.peripherals.i2c12, &self.peripherals.i2cbuff12),
            13 => (&self.peripherals.i2c13, &self.peripherals.i2cbuff13),
            _ => return None,
        })
    }
}

/// Hardware driver for AST1060 I2C controllers
///
/// This driver owns the I2C peripherals for the lifetime of the task.
/// This is the standard Hubris pattern - each task owns its resources.
pub struct Ast1060I2cDriver {
    peripherals: I2cPeripherals,
}

impl Ast1060I2cDriver {
    /// Create a new hardware driver instance
    ///
    /// Takes ownership of the I2C peripherals. This is safe because in Hubris,
    /// each task exclusively owns its peripherals.
    pub fn new(peripherals: I2cPeripherals) -> Self {
        Self { peripherals }
    }

    /// Get references to a specific controller's registers
    fn get_controller(
        &self,
        controller: Controller,
    ) -> Result<
        (
            &ast1060_pac::i2c::RegisterBlock,
            &ast1060_pac::i2cbuff::RegisterBlock,
        ),
        ResponseCode,
    > {
        self.peripherals
            .controller_and_buffer(controller as u8)
            .ok_or(ResponseCode::BadArg)
    }
}

impl I2cHardware for Ast1060I2cDriver {
    type Error = ResponseCode;

    fn write_read(
        &mut self,
        controller: Controller,
        addr: u8,
        write_data: &[u8],
        read_buffer: &mut [u8],
    ) -> Result<usize, ResponseCode> {
        let (regs, buffs) = self.get_controller(controller)?;

        // Create I2C controller reference
        let ctrl = I2cController {
            controller,
            registers: regs,
            buff_registers: buffs,
            notification: None,
        };

        // Create I2C configuration
        let config = I2cConfig::default();

        // Create I2C instance
        let mut i2c = Ast1060I2c::new(&ctrl, config)
            .map_err(|e| ResponseCode::from(e))?;

        // Perform write-read operation
        if !write_data.is_empty() && !read_buffer.is_empty() {
            i2c.write_read(addr, write_data, read_buffer)
                .map_err(|e| ResponseCode::from(e))?;
            Ok(read_buffer.len())
        } else if !write_data.is_empty() {
            i2c.write(addr, write_data)
                .map_err(|e| ResponseCode::from(e))?;
            Ok(0)
        } else if !read_buffer.is_empty() {
            i2c.read(addr, read_buffer)
                .map_err(|e| ResponseCode::from(e))?;
            Ok(read_buffer.len())
        } else {
            Ok(0)
        }
    }

    fn write_read_block(
        &mut self,
        controller: Controller,
        addr: u8,
        write_data: &[u8],
        read_buffer: &mut [u8],
    ) -> Result<usize, ResponseCode> {
        // For AST1060, block mode is the same as regular write_read
        // The driver automatically chunks large transfers
        self.write_read(controller, addr, write_data, read_buffer)
    }

    fn configure_slave_mode(
        &mut self,
        controller: Controller,
        config: &SlaveConfig,
    ) -> Result<(), ResponseCode> {
        let (regs, buffs) = self.get_controller(controller)?;

        let ctrl = I2cController {
            controller,
            registers: regs,
            buff_registers: buffs,
            notification: None,
        };

        let i2c_config = I2cConfig::default();
        let mut i2c = Ast1060I2c::new(&ctrl, i2c_config)
            .map_err(|e| ResponseCode::from(e))?;

        // Convert SlaveConfig to I2C slave config
        let slave_cfg = crate::SlaveConfig::new(config.address)
            .map_err(|_| ResponseCode::BadArg)?;

        i2c.configure_slave(&slave_cfg)
            .map_err(|e| ResponseCode::from(e))?;

        Ok(())
    }

    fn enable_slave_receive(
        &mut self,
        controller: Controller,
    ) -> Result<(), ResponseCode> {
        let (regs, buffs) = self.get_controller(controller)?;

        let ctrl = I2cController {
            controller,
            registers: regs,
            buff_registers: buffs,
            notification: None,
        };

        let i2c_config = I2cConfig::default();
        let _i2c = Ast1060I2c::new(&ctrl, i2c_config)
            .map_err(|e| ResponseCode::from(e))?;

        // Slave mode already configured, just ensure it's enabled
        // The slave interrupts are enabled during configure_slave
        Ok(())
    }

    fn disable_slave_receive(
        &mut self,
        controller: Controller,
    ) -> Result<(), ResponseCode> {
        let (regs, buffs) = self.get_controller(controller)?;

        let ctrl = I2cController {
            controller,
            registers: regs,
            buff_registers: buffs,
            notification: None,
        };

        let i2c_config = I2cConfig::default();
        let mut i2c = Ast1060I2c::new(&ctrl, i2c_config)
            .map_err(|e| ResponseCode::from(e))?;

        i2c.disable_slave();
        Ok(())
    }

    fn configure_timing(
        &mut self,
        _controller: Controller,
        _speed: drv_i2c_types::traits::I2cSpeed,
    ) -> Result<(), Self::Error> {
        // AST1060 timing is configured in I2cConfig during new()
        // The driver automatically sets appropriate timing based on hardware defaults
        Ok(())
    }

    fn reset_bus(&mut self, controller: Controller) -> Result<(), Self::Error> {
        let (regs, buffs) = self.get_controller(controller)?;

        let ctrl = I2cController {
            controller,
            registers: regs,
            buff_registers: buffs,
            notification: None,
        };

        let i2c_config = I2cConfig::default();
        let mut i2c = Ast1060I2c::new(&ctrl, i2c_config)
            .map_err(|e| ResponseCode::from(e))?;

        i2c.recover_bus().map_err(|e| ResponseCode::from(e))
    }

    fn enable_controller(
        &mut self,
        _controller: Controller,
    ) -> Result<(), Self::Error> {
        // AST1060 controllers are enabled by default when peripherals are accessed
        // No explicit enable required
        Ok(())
    }

    fn disable_controller(
        &mut self,
        _controller: Controller,
    ) -> Result<(), Self::Error> {
        // For safety/simplicity, we don't actually disable controllers
        // They can be left enabled without issues
        Ok(())
    }

    /// Check if a specific controller has slave data available
    ///
    /// This is used by the interrupt handler to determine which controller
    /// triggered the interrupt and has data ready.
    fn check_slave_data(&self, controller: Controller) -> bool {
        if let Ok((regs, buffs)) = self.get_controller(controller) {
            let ctrl = I2cController {
                controller,
                registers: regs,
                buff_registers: buffs,
                notification: None,
            };

            let i2c_config = I2cConfig::default();
            if let Ok(i2c) = Ast1060I2c::new(&ctrl, i2c_config) {
                return i2c.slave_has_data();
            }
        }
        false
    }

    /// Clear slave interrupts for a specific controller
    ///
    /// Called after processing slave data to acknowledge the interrupt
    fn clear_slave_interrupts(&self, controller: Controller) {
        if let Ok((regs, _buffs)) = self.get_controller(controller) {
            // Clear all slave interrupt status bits
            unsafe {
                regs.i2cs24().write(|w| w.bits(0xFFFF_FFFF));
            }
        }
    }

    /// Read slave data from hardware
    ///
    /// Returns the source address and number of bytes read into the buffer
    fn read_slave_data(
        &mut self,
        controller: Controller,
        buffer: &mut [u8],
    ) -> Result<(u8, usize), ResponseCode> {
        let (regs, buffs) = self.get_controller(controller)?;

        let ctrl = I2cController {
            controller,
            registers: regs,
            buff_registers: buffs,
            notification: None,
        };

        let i2c_config = I2cConfig::default();
        let mut i2c = Ast1060I2c::new(&ctrl, i2c_config)
            .map_err(|e| ResponseCode::from(e))?;

        // Read slave data
        let data_len =
            i2c.slave_read(buffer).map_err(|e| ResponseCode::from(e))?;

        // For now, we don't have the master's address from hardware
        // This would need to be extracted from the slave address match register
        // For MCTP, the addressing is typically in the data payload anyway
        let source_addr = 0x00; // TODO: Extract from hardware if available

        Ok((source_addr, data_len))
    }
}
