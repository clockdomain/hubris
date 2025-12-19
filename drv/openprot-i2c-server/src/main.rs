//! I2C Server - Embedded Binary
//!
//! This is a vendor-agnostic I2C server that works with any hardware
//! implementing the I2cHardware trait. Hardware vendors integrate by adding
//! their driver module under hardware/<vendor>/ without modifying this file.
//!
//! The server provides a standard IPC interface for I2C operations including:
//! - Master mode read/write/write-read transactions
//! - Slave mode configuration and operation
//! - Bus error recovery
//! - Notification-based async operation

#![no_std]
#![no_main]

use drv_i2c_api::*;
use drv_i2c_types::{traits::I2cHardware, Op, ResponseCode, SlaveMessage};

use userlib::{LeaseAttributes, sys_recv_open, sys_reply, sys_borrow_info, 
              sys_borrow_read, sys_borrow_write, sys_post, 
              TaskId, RecvMessage, FromPrimitive};
use ringbuf::*;

// Hardware abstraction - vendors add their implementation here
mod hardware;

// Hardware-specific driver implementations (conditionally compiled)
#[cfg(feature = "ast1060")]
mod hardware_driver;

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
    // Create hardware-specific I2C driver
    // The hardware module dispatches to the correct vendor implementation
    // based on enabled feature flags (ast1060, stm32, lpc55, etc.)
    let mut driver = hardware::create_driver();
    
    // State for notification-driven slave message delivery
    let mut notification_client: Option<(TaskId, u32)> = None;

    // Message buffer for IPC
    let mut buffer = [0u8; 4];

    loop {
        let msginfo = sys_recv_open(&mut buffer, 0);
        
        // Decode operation
        let op = match Op::from_u32(msginfo.operation) {
            Some(op) => op,
            None => {
                sys_reply(msginfo.sender, 1, &[]); // Bad operation
                continue;
            }
        };
        
        // Handle IPC operation
        handle_operation(op, &msginfo, &mut buffer, &mut driver, &mut notification_client);
    }
}

fn handle_operation<D: I2cHardware>(
    op: Op,
    msginfo: &RecvMessage,
    buffer: &mut [u8],
    driver: &mut D,
    notification_client: &mut Option<(TaskId, u32)>,
) {
    match op {
        Op::WriteRead | Op::WriteReadBlock => {
            handle_write_read(op, msginfo, buffer, driver);
        }
        Op::ConfigureSlaveAddress => {
            handle_configure_slave_address(msginfo, buffer, driver);
        }
        Op::EnableSlaveReceive => {
            handle_enable_slave_receive(msginfo, buffer, driver);
        }
        Op::DisableSlaveReceive => {
            handle_disable_slave_receive(msginfo, buffer, driver);
        }
        Op::EnableSlaveNotification => {
            handle_enable_slave_notification(msginfo, buffer, notification_client);
        }
        Op::DisableSlaveNotification => {
            handle_disable_slave_notification(msginfo, notification_client);
        }
        Op::GetSlaveMessage => {
            // Slave message polling not supported - use interrupt-driven notification instead
            sys_reply(msginfo.sender, ResponseCode::BadArg as u32, &[]);
        }
    }
}

fn handle_write_read<D: I2cHardware>(
    op: Op,
    msginfo: &RecvMessage,
    buffer: &[u8],
    driver: &mut D,
) {
    let lease_count = msginfo.lease_count;
    
    if lease_count < 2 || lease_count % 2 != 0 {
        sys_reply(msginfo.sender, ResponseCode::BadArg as u32, &[]);
        return;
    }
    
    // Extract marshal payload from buffer
    if msginfo.message_len < 4 {
        sys_reply(msginfo.sender, ResponseCode::BadArg as u32, &[]);
        return;
    }
    
    let payload: [u8; 4] = [buffer[0], buffer[1], buffer[2], buffer[3]];
    let (addr, controller, _port, _mux) = match Marshal::unmarshal(&payload) {
        Ok(vals) => vals,
        Err(e) => {
            sys_reply(msginfo.sender, e as u32, &[]);
            return;
        }
    };
    
    let mut total = 0;
    
    // Iterate over write/read pairs
    for i in (0..lease_count).step_by(2) {
        // Get write buffer info
        let winfo = match sys_borrow_info(msginfo.sender, i) {
            Some(info) => info,
            None => {
                sys_reply(msginfo.sender, ResponseCode::BadArg as u32, &[]);
                return;
            }
        };
        
        if !winfo.attributes.contains(LeaseAttributes::READ) {
            sys_reply(msginfo.sender, ResponseCode::BadArg as u32, &[]);
            return;
        }
        
        // Get read buffer info
        let rinfo = match sys_borrow_info(msginfo.sender, i + 1) {
            Some(info) => info,
            None => {
                sys_reply(msginfo.sender, ResponseCode::BadArg as u32, &[]);
                return;
            }
        };
        
        if winfo.len == 0 && rinfo.len == 0 {
            sys_reply(msginfo.sender, ResponseCode::BadArg as u32, &[]);
            return;
        }
        
        if winfo.len > 255 || rinfo.len > 255 {
            sys_reply(msginfo.sender, ResponseCode::BadArg as u32, &[]);
            return;
        }
        
        // Read write data from lease
        let mut write_data = [0u8; 255];
        for pos in 0..winfo.len {
            let (rc, _) = sys_borrow_read(msginfo.sender, i, pos, &mut write_data[pos..pos+1]);
            if rc != 0 {
                sys_reply(msginfo.sender, rc, &[]);
                return;
            }
        }
        
        // Prepare read buffer
        let mut read_buffer = [0u8; 255];
        let read_slice = &mut read_buffer[..rinfo.len];
        
        // Perform the I2C transaction
        let bytes_read = if op == Op::WriteReadBlock {
            match driver.write_read_block(controller, addr, &write_data[..winfo.len], read_slice) {
                Ok(n) => n,
                Err(e) => {
                    let rc: ResponseCode = e.into();
                    sys_reply(msginfo.sender, rc as u32, &[]);
                    return;
                }
            }
        } else {
            match driver.write_read(controller, addr, &write_data[..winfo.len], read_slice) {
                Ok(n) => n,
                Err(e) => {
                    let rc: ResponseCode = e.into();
                    sys_reply(msginfo.sender, rc as u32, &[]);
                    return;
                }
            }
        };
        
        // Write read data back to lease
        for pos in 0..bytes_read.min(rinfo.len) {
            let (rc, _) = sys_borrow_write(msginfo.sender, i + 1, pos, &read_slice[pos..pos+1]);
            if rc != 0 {
                sys_reply(msginfo.sender, rc, &[]);
                return;
            }
        }
        
        total += bytes_read;
    }
    
    sys_reply(msginfo.sender, 0, &total.to_le_bytes());
}

fn handle_configure_slave_address<D: I2cHardware>(
    msginfo: &RecvMessage,
    buffer: &[u8],
    driver: &mut D,
) {
    if msginfo.message_len < 4 {
        sys_reply(msginfo.sender, ResponseCode::BadArg as u32, &[]);
        return;
    }
    
    let payload: [u8; 4] = [buffer[0], buffer[1], buffer[2], buffer[3]];
    let (slave_address, controller, port, _segment) = match Marshal::unmarshal(&payload) {
        Ok(vals) => vals,
        Err(e) => {
            sys_reply(msginfo.sender, e as u32, &[]);
            return;
        }
    };
    
    use drv_i2c_types::SlaveConfig;
    let config = match SlaveConfig::new(controller, port, slave_address) {
        Ok(c) => c,
        Err(_) => {
            sys_reply(msginfo.sender, ResponseCode::BadArg as u32, &[]);
            return;
        }
    };
    
    match driver.configure_slave_mode(controller, &config) {
        Ok(()) => sys_reply(msginfo.sender, 0, &[]),
        Err(e) => {
            let rc: ResponseCode = e.into();
            sys_reply(msginfo.sender, rc as u32, &[]);
        }
    }
}

fn handle_enable_slave_receive<D: I2cHardware>(
    msginfo: &RecvMessage,
    buffer: &[u8],
    driver: &mut D,
) {
    if msginfo.message_len < 4 {
        sys_reply(msginfo.sender, ResponseCode::BadArg as u32, &[]);
        return;
    }
    
    let payload: [u8; 4] = [buffer[0], buffer[1], buffer[2], buffer[3]];
    let (_address, controller, _port, _segment) = match Marshal::unmarshal(&payload) {
        Ok(vals) => vals,
        Err(e) => {
            sys_reply(msginfo.sender, e as u32, &[]);
            return;
        }
    };
    
    match driver.enable_slave_receive(controller) {
        Ok(()) => sys_reply(msginfo.sender, 0, &[]),
        Err(e) => {
            let rc: ResponseCode = e.into();
            sys_reply(msginfo.sender, rc as u32, &[]);
        }
    }
}

fn handle_disable_slave_receive<D: I2cHardware>(
    msginfo: &RecvMessage,
    buffer: &[u8],
    driver: &mut D,
) {
    if msginfo.message_len < 4 {
        sys_reply(msginfo.sender, ResponseCode::BadArg as u32, &[]);
        return;
    }
    
    let payload: [u8; 4] = [buffer[0], buffer[1], buffer[2], buffer[3]];
    let (_address, controller, _port, _segment) = match Marshal::unmarshal(&payload) {
        Ok(vals) => vals,
        Err(e) => {
            sys_reply(msginfo.sender, e as u32, &[]);
            return;
        }
    };
    
    match driver.disable_slave_receive(controller) {
        Ok(()) => sys_reply(msginfo.sender, 0, &[]),
        Err(e) => {
            let rc: ResponseCode = e.into();
            sys_reply(msginfo.sender, rc as u32, &[]);
        }
    }
}

fn handle_enable_slave_notification(
    msginfo: &RecvMessage,
    buffer: &[u8],
    notification_client: &mut Option<(TaskId, u32)>,
) {
    if msginfo.message_len < 4 {
        sys_reply(msginfo.sender, ResponseCode::BadArg as u32, &[]);
        return;
    }
    
    // Check for lease with notification mask
    if msginfo.lease_count < 1 {
        sys_reply(msginfo.sender, ResponseCode::BadArg as u32, &[]);
        return;
    }
    
    let lease_info = match sys_borrow_info(msginfo.sender, 0) {
        Some(info) => info,
        None => {
            sys_reply(msginfo.sender, ResponseCode::BadArg as u32, &[]);
            return;
        }
    };
    
    if lease_info.len < 4 {
        sys_reply(msginfo.sender, ResponseCode::BadArg as u32, &[]);
        return;
    }
    
    // Read notification mask from lease (u32, 4 bytes)
    let mut mask_bytes = [0u8; 4];
    for i in 0..4 {
        let (rc, _) = sys_borrow_read(msginfo.sender, 0, i, &mut mask_bytes[i..i+1]);
        if rc != 0 {
            sys_reply(msginfo.sender, rc, &[]);
            return;
        }
    }
    let notif_mask = u32::from_le_bytes(mask_bytes);
    
    // Store client info for notification delivery
    *notification_client = Some((msginfo.sender, notif_mask));
    
    sys_reply(msginfo.sender, 0, &[]);
}

fn handle_disable_slave_notification(
    msginfo: &RecvMessage,
    notification_client: &mut Option<(TaskId, u32)>,
) {
    // Clear notification state
    *notification_client = None;
    
    sys_reply(msginfo.sender, 0, &[]);
}

include!(concat!(env!("OUT_DIR"), "/notifications.rs"));
