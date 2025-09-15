// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Platform adapters for generic I2C mux drivers

use crate::{I2cControl, I2cController, ReadLength};
use drv_i2c_api::ResponseCode;
use drv_i2c_mux_core::GpioPin;
use drv_i2c_api::Controller;
use drv_i2c_types::traits::{I2cHardware, I2cSpeed, SlaveStatus};
use drv_i2c_types::{SlaveConfig, SlaveMessage};
use drv_stm32xx_sys_api as sys_api;

/// STM32 I2C hardware adapter that implements the I2cHardware trait
pub struct Stm32I2cHardware<'a> {
    pub controller: &'a I2cController<'a>,
    pub ctrl: &'a I2cControl,
}

impl<'a> I2cHardware for Stm32I2cHardware<'a> {
    type Error = ResponseCode;

    fn write_read(
        &mut self,
        _controller: Controller,
        addr: u8,
        write_data: &[u8],
        read_buffer: &mut [u8],
    ) -> Result<usize, Self::Error> {
        let write_len = write_data.len();
        let read_len = read_buffer.len();

        let mut bytes_read = 0;

        self.controller.write_read(
            addr,
            write_len,
            |i| write_data.get(i).copied(),
            ReadLength::Fixed(read_len),
            |i, byte| {
                if let Some(slot) = read_buffer.get_mut(i) {
                    *slot = byte;
                    bytes_read = i + 1;
                    Some(())
                } else {
                    None
                }
            },
            self.ctrl,
        )?;

        Ok(bytes_read)
    }

    fn write_read_block(
        &mut self,
        _controller: Controller,
        _addr: u8,
        _write_data: &[u8],
        _read_buffer: &mut [u8],
    ) -> Result<usize, Self::Error> {
        Err(ResponseCode::NoDevice)
    }

    fn configure_timing(
        &mut self,
        _controller: Controller,
        _speed: I2cSpeed,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    fn reset_bus(&mut self, _controller: Controller) -> Result<(), Self::Error> {
        Ok(())
    }

    fn enable_controller(&mut self, _controller: Controller) -> Result<(), Self::Error> {
        Ok(())
    }

    fn disable_controller(&mut self, _controller: Controller) -> Result<(), Self::Error> {
        Ok(())
    }

    fn configure_slave_mode(
        &mut self,
        _controller: Controller,
        _config: &SlaveConfig,
    ) -> Result<(), Self::Error> {
        Err(ResponseCode::NoDevice)
    }

    fn enable_slave_receive(&mut self, _controller: Controller) -> Result<(), Self::Error> {
        Err(ResponseCode::NoDevice)
    }

    fn disable_slave_receive(&mut self, _controller: Controller) -> Result<(), Self::Error> {
        Err(ResponseCode::NoDevice)
    }

    fn poll_slave_messages(
        &mut self,
        _controller: Controller,
        _messages: &mut [SlaveMessage],
    ) -> Result<usize, Self::Error> {
        Ok(0)
    }

    fn get_slave_status(&self, _controller: Controller) -> Result<SlaveStatus, Self::Error> {
        Ok(SlaveStatus {
            enabled: false,
            messages_received: 0,
            messages_dropped: 0,
            address_matches: 0,
            bus_errors: 0,
            buffer_full: false,
        })
    }
}

/// STM32 GPIO pin adapter that implements the generic GpioPin trait
pub struct Stm32GpioPin<'a> {
    pub pins: sys_api::PinSet,
    pub sys: &'a sys_api::Sys,
}

impl<'a> GpioPin for Stm32GpioPin<'a> {
    fn set_high(&mut self) {
        self.sys.gpio_set(self.pins);
    }
    
    fn set_low(&mut self) {
        self.sys.gpio_reset(self.pins);
    }
    
    fn configure_as_output(&mut self) {
        self.sys.gpio_configure_output(
            self.pins,
            sys_api::OutputType::PushPull,
            sys_api::Speed::Low,
            sys_api::Pull::None,
        );
    }
}

/// Convert our I2cGpio to the generic adapter
impl<'a> Stm32GpioPin<'a> {
    pub fn from_i2c_gpio(gpio: &crate::I2cGpio, sys: &'a sys_api::Sys) -> Self {
        Self {
            pins: gpio.gpio_pins,
            sys,
        }
    }
}