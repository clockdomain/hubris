// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Driver for the PCA9545 I2C mux - STM32 platform integration

use crate::mux_adapters::{Stm32GpioPin, Stm32I2cHardware};
use crate::*;
use drv_i2c_api::{ResponseCode, Segment};
use drv_i2c_mux_core::{pca9545::Pca9545 as GenericPca9545, I2cMuxConfig, I2cMuxDriver as GenericI2cMuxDriver};

pub struct Pca9545;

impl I2cMuxDriver for Pca9545 {
    fn configure(
        &self,
        mux: &I2cMux<'_>,
        _controller: &I2cController<'_>,
        gpio: &sys_api::Sys,
        _ctrl: &I2cControl,
    ) -> Result<(), drv_i2c_api::ResponseCode> {
        // Handle GPIO configuration using the generic driver
        let generic_driver = GenericPca9545;
        let mut config = I2cMuxConfig {
            controller: drv_i2c_api::Controller::I2C1, // TODO: map from mux.controller
            address: mux.address,
            reset_pin: mux.nreset.as_ref().map(|gpio_ref| Stm32GpioPin::from_i2c_gpio(gpio_ref, gpio)),
        };
        
        <GenericPca9545 as GenericI2cMuxDriver<Stm32I2cHardware<'_>, Stm32GpioPin<'_>>>::configure(&generic_driver, &mut config)
    }

    fn enable_segment(
        &self,
        mux: &I2cMux<'_>,
        controller: &I2cController<'_>,
        segment: Option<Segment>,
        ctrl: &I2cControl,
    ) -> Result<(), ResponseCode> {
        let generic_driver = GenericPca9545;
        let config = I2cMuxConfig {
            controller: drv_i2c_api::Controller::I2C1, // TODO: map from mux.controller
            address: mux.address,
            reset_pin: None, // Not needed for segment operations
        };
        let mut i2c_hardware = Stm32I2cHardware { controller, ctrl };

        match <GenericPca9545 as GenericI2cMuxDriver<Stm32I2cHardware<'_>, Stm32GpioPin<'_>>>::enable_segment(&generic_driver, &mut i2c_hardware, &config, segment) {
            Err(code) => Err(mux.error_code(code)),
            Ok(()) => Ok(()),
        }
    }

    fn reset(
        &self,
        mux: &I2cMux<'_>,
        gpio: &sys_api::Sys,
    ) -> Result<(), drv_i2c_api::ResponseCode> {
        let generic_driver = GenericPca9545;
        let mut config = I2cMuxConfig {
            controller: drv_i2c_api::Controller::I2C1, // TODO: map from mux.controller
            address: mux.address,
            reset_pin: mux.nreset.as_ref().map(|gpio_ref| Stm32GpioPin::from_i2c_gpio(gpio_ref, gpio)),
        };
        
        <GenericPca9545 as GenericI2cMuxDriver<Stm32I2cHardware<'_>, Stm32GpioPin<'_>>>::reset(&generic_driver, &mut config)
    }
}
