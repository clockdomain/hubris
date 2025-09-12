// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Generic I2C Server
//! 
//! This crate provides a hardware-agnostic I2C server implementation that can
//! work with different hardware backends through conditional compilation:
//! 
//! - **Mock backend**: For testing and simulation
//! - **STM32 backend**: For STM32 microcontrollers (H7, G0 families)
//! 
//! The same business logic handles multiplexer management, device coordination,
//! and IPC message processing regardless of the underlying hardware.

#![no_std]

use drv_i2c_api::ResponseCode;
use drv_i2c_types::{Controller, I2cHardware, I2cSpeed};
use userlib::*;

// Backend-specific hardware implementations
#[cfg(i2c_backend = "mock")]
mod hardware {
    pub use drv_i2c_types::mock::MockI2cHardware as Hardware;
    pub type Config = ();
    
    pub fn create_hardware(_config: &Config) -> Hardware {
        Hardware::new()
    }
}

#[cfg(i2c_backend = "stm32")]
mod hardware {
    pub use drv_stm32xx_i2c::Stm32I2cHardware as Hardware;
    
    // STM32-specific configuration will come from build-generated code
    pub use crate::i2c_config::{controllers, pins};
    use userlib::sys;
    
    pub fn create_hardware() -> Hardware<'static> {
        let controllers = controllers();
        let pins = pins();
        let sys_task = drv_stm32xx_sys_api::Sys::from(userlib::SYSTEM.get_task_id());
        let ctrl = &drv_stm32xx_i2c::I2cControl {
            enable: sys::enable_irq,
            wfi: sys::kipc_wait_for_interrupt,
        };
        
        Hardware::new(controllers, pins, &sys_task, ctrl)
    }
}

// Re-export hardware type for use by main
pub use hardware::*;

/// Generic I2C server state
pub struct GenericI2cServer<H: I2cHardware> {
    /// Hardware abstraction layer
    hardware: H,
    /// Mux state tracking (using same pattern as STM32 server)
    mux_state: MuxMap,
}

// Type alias for mux state tracking (matches STM32 server pattern)
type MuxMap = fixedmap::FixedMap<
    (Controller, drv_i2c_api::PortIndex),
    MuxState,
    { MAX_MUXED_BUSES }
>;

#[derive(Copy, Clone, Debug)]
struct MuxState {
    current_segment: Option<drv_i2c_api::Segment>,
}

// Configuration constants
const MAX_MUXED_BUSES: usize = 32;  // Same as STM32 server

impl<H: I2cHardware> GenericI2cServer<H> {
    pub fn new(mut hardware: H) -> Self {
        // Initialize all controllers
        for controller in [
            Controller::I2C0, Controller::I2C1, Controller::I2C2, Controller::I2C3,
            Controller::I2C4, Controller::I2C5, Controller::I2C6, Controller::I2C7,
        ] {
            let _ = hardware.enable_controller(controller);
            let _ = hardware.configure_timing(controller, I2cSpeed::Fast);
        }

        Self {
            hardware,
            mux_state: MuxMap::default(),
        }
    }

    /// Handle I2C write-read operation
    pub fn write_read(
        &mut self,
        device: drv_i2c_api::I2cDevice,
        write_data: &[u8],
        read_buffer: &mut [u8],
    ) -> Result<usize, ResponseCode> {
        // Validate device configuration
        self.validate_device(&device)?;

        // Handle multiplexer if needed
        if let Some((mux_id, segment)) = device.segment {
            self.configure_mux(device.controller, device.port, mux_id, segment)?;
        }

        // Perform the actual I2C operation
        self.hardware.write_read(
            device.controller,
            device.address,
            write_data,
            read_buffer,
        ).map_err(|e| e.into())
    }

    /// Handle I2C write-read-block operation (SMBus)
    pub fn write_read_block(
        &mut self,
        device: drv_i2c_api::I2cDevice,
        write_data: &[u8],
        read_buffer: &mut [u8],
    ) -> Result<usize, ResponseCode> {
        // Validate device configuration
        self.validate_device(&device)?;

        // Handle multiplexer if needed
        if let Some((mux_id, segment)) = device.segment {
            self.configure_mux(device.controller, device.port, mux_id, segment)?;
        }

        // Perform the actual I2C block operation
        self.hardware.write_read_block(
            device.controller,
            device.address,
            write_data,
            read_buffer,
        ).map_err(|e| e.into())
    }

    fn validate_device(&self, device: &drv_i2c_api::I2cDevice) -> Result<(), ResponseCode> {
        // Basic validation - in a real implementation, this would check
        // against build-generated device configurations
        if device.address >= 0x80 {
            return Err(ResponseCode::ReservedAddress);
        }
        Ok(())
    }

    fn configure_mux(
        &mut self,
        controller: Controller,
        port: drv_i2c_api::PortIndex,
        _mux_id: drv_i2c_api::Mux,
        segment: drv_i2c_api::Segment,
    ) -> Result<(), ResponseCode> {
        let key = (controller, port);
        
        // Get current mux state
        let current_state = self.mux_state.get(key)
            .unwrap_or(MuxState { current_segment: None });

        // If already on correct segment, nothing to do
        if current_state.current_segment == Some(segment) {
            return Ok(());
        }

        // For mock backend, we just track state without real mux operations
        // For STM32 backend, this would call into the actual mux driver
        let new_state = MuxState {
            current_segment: Some(segment),
        };
        
        self.mux_state.insert(key, new_state).map_err(|_| ResponseCode::BadArg)?;
        
        Ok(())
    }
}

// Mock-specific initialization
#[cfg(i2c_backend = "mock")]
pub fn initialize_server() -> GenericI2cServer<hardware::Hardware> {
    let mut hardware = hardware::create_hardware(&());
    
    // Add some test devices for demo purposes
    hardware.add_device(Controller::I2C0, 0x50);  // EEPROM-like device
    hardware.add_device(Controller::I2C1, 0x48);  // Sensor-like device
    
    GenericI2cServer::new(hardware)
}

// STM32-specific initialization  
#[cfg(i2c_backend = "stm32")]
pub fn initialize_server() -> GenericI2cServer<hardware::Hardware<'static>> {
    let hardware = hardware::create_hardware();
    GenericI2cServer::new(hardware)
}