// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Generic I2C Server Main Entry Point
//! 
//! This server can operate with different hardware backends based on
//! compile-time feature selection.

#![no_std]
#![no_main]

use drv_i2c_api::{I2cDevice, I2cError};
use drv_i2c_server_generic::{initialize_server, GenericI2cServer};
use idol_runtime::{NotificationHandler, RequestError};
use userlib::*;

// Include generated build configuration for STM32 backend
#[cfg(i2c_backend = "stm32")]
include!(concat!(env!("OUT_DIR"), "/i2c_config.rs"));

struct ServerImpl<H: drv_i2c_types::I2cHardware> {
    server: GenericI2cServer<H>,
}

impl<H: drv_i2c_types::I2cHardware> ServerImpl<H> {
    fn new(server: GenericI2cServer<H>) -> Self {
        Self { server }
    }
}

impl<H: drv_i2c_types::I2cHardware> drv_i2c_api::InOrderI2cImpl for ServerImpl<H> 
where
    H::Error: Into<drv_i2c_api::ResponseCode>,
{
    fn write_read(
        &mut self,
        _msg: &RecvMessage,
        device: I2cDevice,
        write_len: u8,
        read_len: u8,
    ) -> Result<u8, RequestError<I2cError>> {
        // Validate lengths
        if write_len > 255 || read_len > 255 {
            return Err(I2cError::ResponseCode(
                drv_i2c_api::ResponseCode::TooMuchData
            ).into());
        }

        // For this demo, we'll use dummy write data and read into a small buffer
        let write_data = [0u8; 16];  // Dummy write data
        let write_slice = &write_data[..write_len.min(16) as usize];
        
        let mut read_buffer = [0u8; 16];  // Small read buffer for demo
        let read_slice = &mut read_buffer[..read_len.min(16) as usize];

        match self.server.write_read(device, write_slice, read_slice) {
            Ok(bytes_read) => Ok(bytes_read.min(255) as u8),
            Err(code) => Err(I2cError::ResponseCode(code).into()),
        }
    }

    fn write_read_block(
        &mut self,
        _msg: &RecvMessage,
        device: I2cDevice,
        write_len: u8,
        read_len: u8,
    ) -> Result<u8, RequestError<I2cError>> {
        // Validate lengths
        if write_len > 255 || read_len > 255 {
            return Err(I2cError::ResponseCode(
                drv_i2c_api::ResponseCode::TooMuchData
            ).into());
        }

        // For this demo, use dummy data (real implementation would use IPC buffers)
        let write_data = [0u8; 16];  
        let write_slice = &write_data[..write_len.min(16) as usize];
        
        let mut read_buffer = [0u8; 16];
        let read_slice = &mut read_buffer[..read_len.min(16) as usize];

        match self.server.write_read_block(device, write_slice, read_slice) {
            Ok(bytes_read) => Ok(bytes_read.min(255) as u8),
            Err(code) => Err(I2cError::ResponseCode(code).into()),
        }
    }
}

impl<H: drv_i2c_types::I2cHardware> NotificationHandler for ServerImpl<H> 
where
    H::Error: Into<drv_i2c_api::ResponseCode>,
{
    fn current_notification_mask(&self) -> u32 {
        // Return appropriate notification mask based on backend
        #[cfg(i2c_backend = "mock")]
        {
            0 // Mock doesn't use interrupts
        }
        
        #[cfg(i2c_backend = "stm32")]
        {
            notifications::I2C1_IRQ_MASK 
                | notifications::I2C2_IRQ_MASK
                | notifications::I2C3_IRQ_MASK 
                | notifications::I2C4_IRQ_MASK
        }
    }

    fn handle(&mut self, _bits: u32) {
        // Handle hardware interrupts (STM32 backend would process I2C interrupts here)
        // Mock backend doesn't need interrupt handling
    }
}

#[export_name = "main"]
fn main() -> ! {
    let server = initialize_server();
    let mut server_impl = ServerImpl::new(server);

    let mut incoming = [0u8; idol_runtime::DEFAULT_SERVER_RECV_BUFFER_SIZE];
    
    loop {
        idol_runtime::dispatch_n(&mut incoming, &mut server_impl);
    }
}