//! Mock I2C Server - Embedded Binary
//!
//! This is the embedded binary entry point for the mock I2C server driver.

#![no_std]
#![no_main]

use drv_i2c_api::*;
use drv_i2c_types::{traits::I2cHardware, Op, ResponseCode, SlaveConfig};

use userlib::{hl, LeaseAttributes};
use ringbuf::*;

mod openprot_adapter;
use openprot_platform_mock::i2c_hardware::MockI2cHardware;
use openprot_adapter::OpenProtI2cAdapter;

/// I2C Controller wrapper for slice-based management
/// 
/// This follows the same pattern as STM32xx I2C server for consistency
/// and memory efficiency, while maintaining generic hardware support.
struct I2cController<H>
where
    H: openprot_hal_blocking::i2c_hardware::I2cHardwareCore 
        + openprot_hal_blocking::i2c_hardware::I2cMaster 
        + openprot_hal_blocking::i2c_hardware::I2cSlaveCore 
        + openprot_hal_blocking::i2c_hardware::I2cSlaveBuffer 
        + openprot_hal_blocking::i2c_hardware::I2cSlaveInterrupts,
{
    controller: Controller,
    adapter: OpenProtI2cAdapter<H>,
}

impl<H> I2cController<H>
where
    H: openprot_hal_blocking::i2c_hardware::I2cHardwareCore 
        + openprot_hal_blocking::i2c_hardware::I2cMaster 
        + openprot_hal_blocking::i2c_hardware::I2cSlaveCore 
        + openprot_hal_blocking::i2c_hardware::I2cSlaveBuffer 
        + openprot_hal_blocking::i2c_hardware::I2cSlaveInterrupts,
{
    fn new(controller: Controller, hardware: H) -> Self {
        let adapter = OpenProtI2cAdapter::new(controller, hardware);
        
        Self {
            controller,
            adapter,
        }
    }
}

/// Lookup a controller by ID, similar to STM32xx pattern
fn lookup_controller<H>(
    controllers: &mut [I2cController<H>],
    controller: Controller,
) -> Result<&mut I2cController<H>, ResponseCode>
where
    H: openprot_hal_blocking::i2c_hardware::I2cHardwareCore 
        + openprot_hal_blocking::i2c_hardware::I2cMaster 
        + openprot_hal_blocking::i2c_hardware::I2cSlaveCore 
        + openprot_hal_blocking::i2c_hardware::I2cSlaveBuffer 
        + openprot_hal_blocking::i2c_hardware::I2cSlaveInterrupts,
{
    controllers
        .iter_mut()
        .find(|c| c.controller == controller)
        .ok_or(ResponseCode::BadController)
}

#[derive(Copy, Clone, PartialEq, Count)]
enum Trace {
    None,
    Transaction { controller: u8, addr: u8, len: usize },
    SlaveConfigured { controller: u8, addr: u8 },
    SlaveMessage { controller: u8, addr: u8, len: usize },
    #[count(skip)]
    Panic { controller: u8, status: u32 },
}

counted_ringbuf!(Trace, 64, Trace::None);

#[export_name = "main"]
fn main() -> ! {
    // Create controllers array - using slice approach like STM32xx
    // but with generic hardware support and maximum controller instances
    let mut controllers = [
        I2cController::new(Controller::I2C0, MockI2cHardware::new()),
        I2cController::new(Controller::I2C1, MockI2cHardware::new()),
        I2cController::new(Controller::I2C2, MockI2cHardware::new()),
        I2cController::new(Controller::I2C3, MockI2cHardware::new()),
        I2cController::new(Controller::I2C4, MockI2cHardware::new()),
        I2cController::new(Controller::I2C5, MockI2cHardware::new()),
        I2cController::new(Controller::I2C6, MockI2cHardware::new()),
        I2cController::new(Controller::I2C7, MockI2cHardware::new()),
    ];

    // Field messages
    let mut buffer = [0; 4];

    loop {
        hl::recv_without_notification(&mut buffer, |op, msg| match op {
            Op::WriteRead | Op::WriteReadBlock => {
                let lease_count = msg.lease_count();

                let (payload, caller) = msg
                    .fixed::<[u8; 4], usize>()
                    .ok_or(ResponseCode::BadArg)?;

                if lease_count < 2 || lease_count % 2 != 0 {
                    return Err(ResponseCode::BadArg);
                }

                // For mock mode, we use the standard marshal format but ignore complex topology
                let (addr, controller_id, _port, _mux) = Marshal::unmarshal(payload)?;

                // Lookup the controller using slice approach
                let controller = lookup_controller(&mut controllers, controller_id)?;

                let mut total = 0;

                // Iterate over write/read pairs
                for i in (0..lease_count).step_by(2) {
                    let wbuf = caller.borrow(i);
                    let winfo = wbuf.info().ok_or(ResponseCode::BadArg)?;

                    if !winfo.attributes.contains(LeaseAttributes::READ) {
                        return Err(ResponseCode::BadArg);
                    }

                    let rbuf = caller.borrow(i + 1);
                    let rinfo = rbuf.info().ok_or(ResponseCode::BadArg)?;

                    if winfo.len == 0 && rinfo.len == 0 {
                        return Err(ResponseCode::BadArg);
                    }

                    if winfo.len > 255 || rinfo.len > 255 {
                        // Keep the 255 limit as per IPC protocol
                        return Err(ResponseCode::BadArg);
                    }

                    // Read write data from lease
                    let mut write_data = [0u8; 255];
                    for pos in 0..winfo.len {
                        write_data[pos] = wbuf.read_at(pos).ok_or(ResponseCode::BadArg)?;
                    }

                    // Prepare read buffer
                    let mut read_buffer = [0u8; 255];
                    let read_slice = &mut read_buffer[..rinfo.len];

                    // Perform the I2C transaction
                    let bytes_read = if op == Op::WriteReadBlock {
                        controller.adapter.write_read_block(
                            controller_id,
                            addr,
                            &write_data[..winfo.len],
                            read_slice,
                        )?
                    } else {
                        controller.adapter.write_read(
                            controller_id,
                            addr,
                            &write_data[..winfo.len],
                            read_slice,
                        )?
                    };

                    // Write read data back to lease
                    for pos in 0..bytes_read.min(rinfo.len) {
                        rbuf.write_at(pos, read_slice[pos]).ok_or(ResponseCode::BadArg)?;
                    }

                    total += bytes_read;
                }

                caller.reply(total);
                Ok(())
            }
            Op::ConfigureSlaveAddress => {
                // Use the same marshal format as WriteRead operations
                let (payload, caller) = msg
                    .fixed::<[u8; 4], usize>()
                    .ok_or(ResponseCode::BadArg)?;

                let (slave_address, controller_id, port, _segment) = Marshal::unmarshal(payload)?;
                
                // Lookup the controller
                let controller = lookup_controller(&mut controllers, controller_id)?;
                
                // Create slave configuration  
                let config = SlaveConfig::new(controller_id, port, slave_address)
                    .map_err(|_| ResponseCode::BadArg)?;
                
                // Configure slave mode on hardware
                controller.adapter.configure_slave_mode(controller_id, &config)?;
                
                caller.reply(0usize);
                Ok(())
            }
            Op::EnableSlaveReceive => {
                // Use the same marshal format as WriteRead operations
                let (payload, caller) = msg
                    .fixed::<[u8; 4], usize>()
                    .ok_or(ResponseCode::BadArg)?;

                let (_address, controller_id, _port, _segment) = Marshal::unmarshal(payload)?;
                
                // Lookup the controller
                let controller = lookup_controller(&mut controllers, controller_id)?;
                
                controller.adapter.enable_slave_receive(controller_id)?;
                caller.reply(0usize);
                Ok(())
            }
            Op::DisableSlaveReceive => {
                // Use the same marshal format as WriteRead operations
                let (payload, caller) = msg
                    .fixed::<[u8; 4], usize>()
                    .ok_or(ResponseCode::BadArg)?;

                let (_address, controller_id, _port, _segment) = Marshal::unmarshal(payload)?;
                
                // Lookup the controller
                let controller = lookup_controller(&mut controllers, controller_id)?;
                
                controller.adapter.disable_slave_receive(controller_id)?;
                caller.reply(0usize);
                Ok(())
            }
            Op::CheckSlaveBuffer => {
                // Use the same marshal format as WriteRead operations
                let (payload, caller) = msg
                    .fixed::<[u8; 4], usize>()
                    .ok_or(ResponseCode::BadArg)?;

                let (_address, _controller, _port, _segment) = Marshal::unmarshal(payload)?;
                
                // Check for slave messages - for now just return count
                // A full implementation would need to handle message data formatting
                let temp_messages: [u8; 0] = []; // Empty for mock
                let count = temp_messages.len();
                
                caller.reply(count);
                Ok(())
            }
        });
    }
}
