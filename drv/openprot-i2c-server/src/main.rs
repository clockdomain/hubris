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
use drv_i2c_types::{traits::I2cHardware, Op, ResponseCode};

use ringbuf::*;
use userlib::{
    sys_borrow_info, sys_borrow_read, sys_borrow_write, sys_irq_control,
    sys_post, sys_recv_open, sys_reply, FromPrimitive, LeaseAttributes,
    RecvMessage, TaskId,
};

// Hardware abstraction - vendors add their implementation here
mod hardware;

#[derive(Copy, Clone, PartialEq, Count)]
enum Trace {
    None,
    Transaction {
        controller: u8,
        addr: u8,
        len: usize,
    },
    SlaveConfigured {
        controller: u8,
        addr: u8,
    },
    SlaveMessage {
        controller: u8,
        addr: u8,
        len: usize,
    },
    #[count(skip)]
    Panic {
        controller: u8,
        status: u32,
    },
}

counted_ringbuf!(Trace, 64, Trace::None);

#[export_name = "main"]
fn main() -> ! {
    let mut driver = hardware::create_driver();

    // Slave message: just track if we have one pending and which controller
    // We only buffer one message at a time to minimize stack usage
    let mut pending_slave_msg: Option<(u8, SlaveMessage)> = None; // (controller_id, message)

    // State for notification-driven slave message delivery
    let mut notification_client: Option<(TaskId, u32)> = None;

    // Enable I2C interrupts
    sys_irq_control(notifications::I2C_IRQ_MASK, true);

    // Message buffer for IPC
    let mut buffer = [0u8; 4];

    loop {
        // Wait for IPC messages OR interrupt notifications
        let msginfo = sys_recv_open(&mut buffer, notifications::I2C_IRQ_MASK);

        // Check if this is an interrupt notification from the kernel
        if msginfo.sender == TaskId::KERNEL {
            if msginfo.operation & notifications::I2C_IRQ_MASK != 0 {
                // Handle I2C slave interrupt
                handle_slave_interrupt(
                    &mut driver,
                    &mut pending_slave_msg,
                    &notification_client,
                );

                // Re-enable interrupt for next event
                sys_irq_control(notifications::I2C_IRQ_MASK, true);
            }
            continue;
        }

        // Decode IPC operation
        let op = match Op::from_u32(msginfo.operation) {
            Some(op) => op,
            None => {
                sys_reply(msginfo.sender, 1, &[]); // Bad operation
                continue;
            }
        };

        // Handle IPC operation
        handle_operation(
            op,
            &msginfo,
            &mut buffer,
            &mut driver,
            &mut notification_client,
            &mut pending_slave_msg,
        );
    }
}

fn handle_operation<D: I2cHardware>(
    op: Op,
    msginfo: &RecvMessage,
    buffer: &mut [u8],
    driver: &mut D,
    notification_client: &mut Option<(TaskId, u32)>,
    pending_slave_msg: &mut Option<(u8, SlaveMessage)>,
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
            handle_enable_slave_notification(
                msginfo,
                buffer,
                notification_client,
            );
        }
        Op::DisableSlaveNotification => {
            handle_disable_slave_notification(msginfo, notification_client);
        }
        Op::GetSlaveMessage => {
            handle_get_slave_message(msginfo, buffer, pending_slave_msg);
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
            let (rc, _) = sys_borrow_read(
                msginfo.sender,
                i,
                pos,
                &mut write_data[pos..pos + 1],
            );
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
            match driver.write_read_block(
                controller,
                addr,
                &write_data[..winfo.len],
                read_slice,
            ) {
                Ok(n) => n,
                Err(e) => {
                    let rc: ResponseCode = e.into();
                    sys_reply(msginfo.sender, rc as u32, &[]);
                    return;
                }
            }
        } else {
            match driver.write_read(
                controller,
                addr,
                &write_data[..winfo.len],
                read_slice,
            ) {
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
            let (rc, _) = sys_borrow_write(
                msginfo.sender,
                i + 1,
                pos,
                &read_slice[pos..pos + 1],
            );
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
    let (slave_address, controller, port, _segment) =
        match Marshal::unmarshal(&payload) {
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
    let (_address, controller, _port, _segment) =
        match Marshal::unmarshal(&payload) {
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
    let (_address, controller, _port, _segment) =
        match Marshal::unmarshal(&payload) {
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
    _buffer: &[u8],
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
        let (rc, _) =
            sys_borrow_read(msginfo.sender, 0, i, &mut mask_bytes[i..i + 1]);
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

/// Handle I2C slave interrupt
///
/// This is called when the hardware interrupt fires. It reads slave data
/// from hardware, buffers it, and notifies any registered client.
fn handle_slave_interrupt<D: I2cHardware>(
    driver: &mut D,
    pending_slave_msg: &mut Option<(u8, SlaveMessage)>,
    notification_client: &Option<(TaskId, u32)>,
) {
    // Check all controllers to see which one(s) have data
    // Note: Controller enum currently only defines I2C0-I2C7
    for controller_id in 0..8 {
        let controller = match controller_id {
            0 => Controller::I2C0,
            1 => Controller::I2C1,
            2 => Controller::I2C2,
            3 => Controller::I2C3,
            4 => Controller::I2C4,
            5 => Controller::I2C5,
            6 => Controller::I2C6,
            7 => Controller::I2C7,
            _ => continue,
        };

        if driver.check_slave_data(controller) {
            // Read the slave data from hardware
            let mut data_buffer = [0u8; 255];
            match driver.read_slave_data(controller, &mut data_buffer) {
                Ok((source_addr, data_len)) => {
                    // Create and store the slave message (overwrites previous if not yet retrieved)
                    if let Ok(msg) =
                        SlaveMessage::new(source_addr, &data_buffer[..data_len])
                    {
                        *pending_slave_msg = Some((controller_id, msg));

                        ringbuf_entry!(Trace::SlaveMessage {
                            controller: controller_id as u8,
                            addr: source_addr,
                            len: data_len
                        });

                        // Notify the client if registered
                        if let Some((client_task, notification_mask)) =
                            notification_client
                        {
                            sys_post(*client_task, *notification_mask);
                        }
                    }
                }
                Err(_) => {
                    // Error reading slave data, just clear the interrupt
                }
            }

            // Clear the interrupt for this controller
            driver.clear_slave_interrupts(controller);
        }
    }
}

/// Handle GetSlaveMessage operation
///
/// Returns buffered slave message if available, or NoSlaveMessage error
fn handle_get_slave_message(
    msginfo: &RecvMessage,
    buffer: &[u8],
    pending_slave_msg: &mut Option<(u8, SlaveMessage)>,
) {
    if msginfo.message_len < 4 {
        sys_reply(msginfo.sender, ResponseCode::BadArg as u32, &[]);
        return;
    }

    let payload: [u8; 4] = [buffer[0], buffer[1], buffer[2], buffer[3]];
    let (_address, controller, _port, _segment) =
        match Marshal::unmarshal(&payload) {
            Ok(vals) => vals,
            Err(e) => {
                sys_reply(msginfo.sender, e as u32, &[]);
                return;
            }
        };

    let controller_id = controller as u8;
    if controller_id >= 8 {
        sys_reply(msginfo.sender, ResponseCode::BadArg as u32, &[]);
        return;
    }

    // Check if we have a buffered message for this controller
    if let Some((msg_controller_id, msg)) = pending_slave_msg.take() {
        if msg_controller_id == controller_id {
            // Send message: 2 bytes header (source_addr, data_length) + data
            let data_len = msg.data_length as usize;
            let total_len = 2 + data_len;
            let mut response_buf = [0u8; 257]; // 2 header + 255 data max

            response_buf[0] = msg.source_address;
            response_buf[1] = msg.data_length;
            response_buf[2..2 + data_len]
                .copy_from_slice(&msg.data[..data_len]);

            sys_reply(msginfo.sender, 0, &response_buf[..total_len]);
        } else {
            // Put it back, wrong controller
            *pending_slave_msg = Some((msg_controller_id, msg));
            sys_reply(msginfo.sender, ResponseCode::NoSlaveMessage as u32, &[]);
        }
    } else {
        // No message available
        sys_reply(msginfo.sender, ResponseCode::NoSlaveMessage as u32, &[]);
    }
}

include!(concat!(env!("OUT_DIR"), "/notifications.rs"));
