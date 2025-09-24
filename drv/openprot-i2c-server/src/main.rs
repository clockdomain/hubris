//! Mock I2C Server - Embedded Binary
//!
//! This is the embedded binary entry point for the mock I2C server driver.

#![no_std]
#![no_main]

use drv_i2c_api::*;
use drv_i2c_types::{Op, ResponseCode, traits::I2cHardware};

use userlib::{hl, LeaseAttributes};
use ringbuf::*;

mod openprot_adapter;

// Import appropriate hardware backend based on features
#[cfg(feature = "mock")]
use openprot_platform_mock::i2c_hardware::MockI2cHardware;
#[cfg(feature = "ast1060")]
use aspeed_ddk::i2c::hardware_instantiation::{I2cControllerWrapper, instantiate_hardware};

// Static storage for AST1060 controllers
#[cfg(feature = "ast1060")]
static mut AST1060_CONTROLLERS: Option<[I2cControllerWrapper<'static>; 13]> = None;
#[cfg(feature = "ast1060")]
static CONTROLLERS_INIT: core::sync::atomic::AtomicBool = core::sync::atomic::AtomicBool::new(false);

use openprot_adapter::OpenProtI2cAdapter;

#[cfg(feature = "mock")]
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

#[cfg(feature = "mock")]
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

#[cfg(feature = "mock")]
/// Lookup a controller by ID - works with any hardware implementing OpenProt HAL traits
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

/// Get static AST1060 controllers with proper lifetime
#[cfg(feature = "ast1060")]
fn get_ast1060_controllers() -> &'static mut [I2cControllerWrapper<'static>; 13] {
    if !CONTROLLERS_INIT.load(core::sync::atomic::Ordering::Acquire) {
        unsafe {
            AST1060_CONTROLLERS = Some(instantiate_hardware());
            CONTROLLERS_INIT.store(true, core::sync::atomic::Ordering::Release);
        }
    }

    unsafe {
        AST1060_CONTROLLERS.as_mut().unwrap()
    }
}

/// Get AST1060 controller index by ID
#[cfg(feature = "ast1060")]
fn get_ast1060_controller_index(controller: Controller) -> Result<usize, ResponseCode> {
    let index = match controller {
        Controller::I2C0 => 0,
        Controller::I2C1 => 1,
        Controller::I2C2 => 2,
        Controller::I2C3 => 3,
        Controller::I2C4 => 4,
        Controller::I2C5 => 5,
        Controller::I2C6 => 6,
        Controller::I2C7 => 7,
        Controller::I2C8 => 8,
        Controller::I2C9 => 9,
        Controller::I2C10 => 10,
        Controller::I2C11 => 11,
        Controller::I2C12 => 12,
        // AST1060 only has 13 controllers, so I2C13-I2C15 are not supported
        _ => return Err(ResponseCode::BadController),
    };

    Ok(index)
}

/// Convert embedded-hal I2C errors to Hubris ResponseCode
#[cfg(feature = "ast1060")]
fn map_i2c_error(error: aspeed_ddk::i2c::ast1060_i2c::Error) -> ResponseCode {
    match error {
        aspeed_ddk::i2c::ast1060_i2c::Error::Bus => ResponseCode::BusError,
        aspeed_ddk::i2c::ast1060_i2c::Error::Timeout => ResponseCode::BusError,
        _ => ResponseCode::BusError, // Default mapping for all other errors
    }
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
    #[cfg(feature = "mock")]
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

    // Controllers are now managed via static storage for AST1060

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

                    // Perform the I2C transaction with appropriate controller
                    let bytes_read = match (cfg!(feature = "mock"), cfg!(feature = "ast1060")) {
                        (true, false) => {
                            #[cfg(feature = "mock")]
                            {
                                let controller = lookup_controller(&mut controllers, controller_id)?;
                                if op == Op::WriteReadBlock {
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
                                }
                            }
                            #[cfg(not(feature = "mock"))]
                            unreachable!()
                        }
                        (false, true) => {
                            #[cfg(feature = "ast1060")]
                            {
                                let index = get_ast1060_controller_index(controller_id)?;
                                let controllers = get_ast1060_controllers();
                                let controller = &mut controllers[index];

                                if op == Op::WriteReadBlock {
                                    // For block operations, use write_read
                                    controller.as_i2c_mut()
                                        .write_read(addr, &write_data[..winfo.len], read_slice)
                                        .map_err(map_i2c_error)?;

                                    // For SMBus block reads, first byte contains length
                                    if !read_slice.is_empty() && winfo.len > 0 {
                                        let block_length = read_slice[0] as usize;
                                        block_length.min(read_slice.len().saturating_sub(1))
                                    } else {
                                        rinfo.len
                                    }
                                } else {
                                    // For regular write_read, embedded-hal returns () on success
                                    controller.as_i2c_mut()
                                        .write_read(addr, &write_data[..winfo.len], read_slice)
                                        .map_err(map_i2c_error)?;
                                    // Return the full buffer length since embedded-hal fills the entire buffer
                                    rinfo.len
                                }
                            }
                            #[cfg(not(feature = "ast1060"))]
                            unreachable!()
                        }
                        _ => return Err(ResponseCode::NotImplemented),
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

                // Lookup the controller using appropriate approach
                #[cfg(feature = "mock")]
                {
                    let controller = lookup_controller(&mut controllers, controller_id)?;
                    let config = SlaveConfig::new(controller_id, port, slave_address)
                        .map_err(|_| ResponseCode::BadArg)?;
                    controller.adapter.configure_slave_mode(controller_id, &config)?;
                }

                #[cfg(feature = "ast1060")]
                {
                    let index = get_ast1060_controller_index(controller_id)?;
                    let controllers = get_ast1060_controllers();
                    let controller = &mut controllers[index];

                    // Use hardware-specific slave functionality if available
                    #[cfg(feature = "i2c_target")]
                    {
                        // Use the wrapper's built-in slave configuration if available
                        // This avoids the lifetime issues in get_hardware_mut()
                        // For now, assume slave configuration succeeds
                        // TODO: Implement proper slave address configuration when aspeed-ddk is fixed
                    }

                    #[cfg(not(feature = "i2c_target"))]
                    {
                        return Err(ResponseCode::OperationNotSupported);
                    }
                }
                
                caller.reply(0usize);
                Ok(())
            }
            Op::EnableSlaveReceive => {
                // Use the same marshal format as WriteRead operations
                let (payload, caller) = msg
                    .fixed::<[u8; 4], usize>()
                    .ok_or(ResponseCode::BadArg)?;

                let (_address, controller_id, _port, _segment) = Marshal::unmarshal(payload)?;
                
                // Lookup the controller using appropriate approach based on feature
                #[cfg(feature = "mock")]
                {
                    let controller = lookup_controller(&mut controllers, controller_id)?;
                    controller.adapter.enable_slave_receive(controller_id)?;
                }

                #[cfg(feature = "ast1060")]
                {
                    let index = get_ast1060_controller_index(controller_id)?;
                    let controllers = get_ast1060_controllers();
                    let controller = &mut controllers[index];

                    #[cfg(feature = "i2c_target")]
                    {
                        // Use the wrapper's built-in slave enable if available
                        // For now, assume slave enable succeeds
                        // TODO: Implement proper slave enable when aspeed-ddk is fixed
                    }

                    #[cfg(not(feature = "i2c_target"))]
                    {
                        return Err(ResponseCode::OperationNotSupported);
                    }
                }
                caller.reply(0usize);
                Ok(())
            }
            Op::DisableSlaveReceive => {
                // Use the same marshal format as WriteRead operations
                let (payload, caller) = msg
                    .fixed::<[u8; 4], usize>()
                    .ok_or(ResponseCode::BadArg)?;

                let (_address, controller_id, _port, _segment) = Marshal::unmarshal(payload)?;
                
                // Lookup the controller using appropriate approach based on feature
                #[cfg(feature = "mock")]
                {
                    let controller = lookup_controller(&mut controllers, controller_id)?;
                    controller.adapter.disable_slave_receive(controller_id)?;
                }

                #[cfg(feature = "ast1060")]
                {
                    let index = get_ast1060_controller_index(controller_id)?;
                    let controllers = get_ast1060_controllers();
                    let controller = &mut controllers[index];

                    #[cfg(feature = "i2c_target")]
                    {
                        // Use the wrapper's built-in slave disable if available
                        // For now, assume slave disable succeeds
                        // TODO: Implement proper slave disable when aspeed-ddk is fixed
                    }

                    #[cfg(not(feature = "i2c_target"))]
                    {
                        return Err(ResponseCode::OperationNotSupported);
                    }
                }
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
