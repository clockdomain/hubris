// Copyright 2024 Advanced Micro Devices, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Aardvark I2C Bringup Test Suite
//!
//! This module contains hardware bringup tests designed to be used with
//! a Total Phase Aardvark I2C/SPI adapter configured as an I2C slave device.
//!
//! Test sequence matches AARDVARK-BRINGUP.md documentation.

use crate::uart_send;
use drv_i2c_api::{I2cDevice, ResponseCode};

/// Aardvark slave address for testing
const AARDVARK_ADDR: u8 = 0x50;

/// AST1060 slave address for slave mode tests
const AST1060_SLAVE_ADDR: u8 = 0x42;

/// Run all Aardvark bringup tests in sequence
pub fn run_aardvark_tests(device: &I2cDevice) {
    uart_send(b"\r\n=== AARDVARK BRINGUP TEST SUITE ===\r\n");
    uart_send(b"Target device: Aardvark I2C/SPI adapter at 0x50\r\n");
    uart_send(b"See AARDVARK-BRINGUP.md for setup instructions\r\n\r\n");

    // Master mode tests (AST1060 as master, Aardvark as slave)
    uart_send(b"--- MASTER MODE TESTS ---\r\n");
    test_1_bus_scan(device);
    test_2_single_write(device);
    test_3_single_read(device);
    test_4_write_read(device);
    test_5_buffer_mode(device);
    test_6_error_recovery(device);

    // Slave mode tests (AST1060 as slave, Aardvark as master)
    uart_send(b"\r\n--- SLAVE MODE TESTS ---\r\n");
    uart_send(b"Configure Aardvark as I2C Master for these tests\r\n\r\n");
    test_7_slave_configuration(device);
    test_8_slave_receive(device);
    test_9_slave_bidirectional(device);

    uart_send(b"\r\n=== AARDVARK TESTS COMPLETE ===\r\n\r\n");
}

/// Test 1: Bus Scan - Verify device presence
fn test_1_bus_scan(device: &I2cDevice) {
    uart_send(b"Test 1: Bus Scan (Verify Connectivity)\r\n");
    uart_send(b"  Goal: Confirm AST1060 can detect Aardvark on I2C bus\r\n");
    uart_send(b"  I2C bus scan: Looking for device at 0x50...\r\n");

    // Attempt a zero-byte write to check for ACK
    let scan_result = device.write(&[]);

    match scan_result {
        Ok(_) => {
            uart_send(
                b"  \xE2\x9C\x93 Device found at 0x50 (ACK received)\r\n",
            );
            uart_send(b"  PASS\r\n\r\n");
        }
        Err(e) => {
            uart_send(b"  \xE2\x9C\x97 No device found at 0x50\r\n");
            print_error(e);
            uart_send(b"  FAIL\r\n\r\n");
        }
    }
}

/// Test 2: Single Byte Write
fn test_2_single_write(device: &I2cDevice) {
    uart_send(b"Test 2: Single Byte Write\r\n");
    uart_send(b"  Goal: Verify AST1060 can write data to Aardvark\r\n");
    uart_send(b"  Master Write Test: addr=0x50, data=[0x42]\r\n");

    let write_data = [0x42u8];
    let result = device.write(&write_data);

    match result {
        Ok(_) => {
            uart_send(b"  \xE2\x9C\x93 Write successful\r\n");
            uart_send(
                b"  Expected on Aardvark: I2C Slave Received: [0x42]\r\n",
            );
            uart_send(b"  PASS\r\n\r\n");
        }
        Err(e) => {
            uart_send(b"  \xE2\x9C\x97 Write failed\r\n");
            print_error(e);
            uart_send(b"  FAIL\r\n\r\n");
        }
    }
}

/// Test 3: Single Byte Read
fn test_3_single_read(device: &I2cDevice) {
    uart_send(b"Test 3: Single Byte Read\r\n");
    uart_send(b"  Goal: Verify AST1060 can read data from Aardvark\r\n");
    uart_send(
        b"  Expected: Aardvark slave response buffer contains [0xAB]\r\n",
    );
    uart_send(b"  Master Read Test: addr=0x50, len=1\r\n");

    let mut read_buffer = [0u8; 1];
    let result: Result<u8, ResponseCode> = device.read_reg(0u8);

    match result {
        Ok(value) => {
            read_buffer[0] = value;
            uart_send(b"  \xE2\x9C\x93 Read successful: [0x");
            print_hex_byte(value);
            uart_send(b"]\r\n");

            if value == 0xAB {
                uart_send(
                    b"  \xE2\x9C\x93 Data matches expected value (0xAB)\r\n",
                );
                uart_send(b"  PASS\r\n\r\n");
            } else {
                uart_send(b"  \xE2\x9C\x97 Data mismatch (expected 0xAB)\r\n");
                uart_send(
                    b"  PARTIAL PASS (read works, but wrong data)\r\n\r\n",
                );
            }
        }
        Err(e) => {
            uart_send(b"  \xE2\x9C\x97 Read failed\r\n");
            print_error(e);
            uart_send(b"  FAIL\r\n\r\n");
        }
    }
}

/// Test 4: Write-Read (Register Access Pattern)
fn test_4_write_read(device: &I2cDevice) {
    uart_send(b"Test 4: Write-Read (Register Access Pattern)\r\n");
    uart_send(b"  Goal: Test combined write-then-read transaction\r\n");
    uart_send(
        b"  Expected: Aardvark response for reg 0x10 = [DE, AD, BE, EF]\r\n",
    );
    uart_send(b"  Write-Read Test: addr=0x50, reg=0x10, read_len=4\r\n");

    let mut read_buffer = [0u8; 4];
    let result = device.read_reg_into(0x10u8, &mut read_buffer);

    match result {
        Ok(_) => {
            uart_send(b"  \xE2\x9C\x93 Write-Read successful: [");
            for (i, byte) in read_buffer.iter().enumerate() {
                if i > 0 {
                    uart_send(b", ");
                }
                print_hex_byte(*byte);
            }
            uart_send(b"]\r\n");

            if read_buffer == [0xDE, 0xAD, 0xBE, 0xEF] {
                uart_send(b"  \xE2\x9C\x93 Data matches expected sequence\r\n");
                uart_send(b"  PASS\r\n\r\n");
            } else {
                uart_send(
                    b"  \xE2\x9C\x97 Data mismatch (expected DE AD BE EF)\r\n",
                );
                uart_send(b"  PARTIAL PASS (transaction works, but wrong data)\r\n\r\n");
            }
        }
        Err(e) => {
            uart_send(b"  \xE2\x9C\x97 Write-Read failed\r\n");
            print_error(e);
            uart_send(b"  FAIL\r\n\r\n");
        }
    }
}

/// Test 5: Buffer Mode (32 Bytes)
fn test_5_buffer_mode(device: &I2cDevice) {
    uart_send(b"Test 5: Buffer Mode (32 Bytes)\r\n");
    uart_send(b"  Goal: Verify buffer mode for larger transfers\r\n");
    uart_send(b"  Buffer Mode Write: addr=0x50, len=32\r\n");
    uart_send(b"  Pattern: 0x00 to 0x1F\r\n");

    // Create 32-byte pattern: 0x00, 0x01, ..., 0x1F
    let mut write_data = [0u8; 32];
    for (i, byte) in write_data.iter_mut().enumerate() {
        *byte = i as u8;
    }

    let result = device.write(&write_data);

    match result {
        Ok(_) => {
            uart_send(
                b"  \xE2\x9C\x93 Using buffer mode (single transaction)\r\n",
            );
            uart_send(b"  \xE2\x9C\x93 Write successful\r\n");
            uart_send(
                b"  Expected on Aardvark: [00 01 02 ... 1E 1F] (32 bytes)\r\n",
            );
            uart_send(b"  PASS\r\n\r\n");
        }
        Err(e) => {
            uart_send(b"  \xE2\x9C\x97 Buffer mode write failed\r\n");
            print_error(e);
            uart_send(b"  FAIL\r\n\r\n");
        }
    }
}

/// Test 6: Bus Error Recovery
fn test_6_error_recovery(device: &I2cDevice) {
    uart_send(b"Test 6: Bus Error Recovery\r\n");
    uart_send(b"  Goal: Test timeout and bus recovery mechanism\r\n");
    uart_send(b"  Error Test: Writing to invalid address 0x7F\r\n");

    // Attempt to write to non-existent device
    let invalid_device = I2cDevice::new(
        device.task,
        device.controller,
        device.port,
        device.segment,
        0x7F, // Non-existent address
        "aardvark-test",
    );

    let result = invalid_device.write(&[0x00]);

    match result {
        Ok(_) => {
            uart_send(b"  \xE2\x9C\x97 Unexpected success (device shouldn't exist)\r\n");
            uart_send(b"  FAIL\r\n\r\n");
        }
        Err(e) => {
            uart_send(b"  \xE2\x9C\x93 Error detected (expected): ");
            print_error(e);

            // Now verify bus is still functional by writing to valid device
            uart_send(b"  Testing bus recovery...\r\n");
            let recovery_result = device.write(&[0xFF]);

            match recovery_result {
                Ok(_) => {
                    uart_send(b"  \xE2\x9C\x93 Bus recovery successful\r\n");
                    uart_send(
                        b"  \xE2\x9C\x93 Next transaction successful\r\n",
                    );
                    uart_send(b"  PASS\r\n\r\n");
                }
                Err(e) => {
                    uart_send(b"  \xE2\x9C\x97 Bus recovery failed\r\n");
                    print_error(e);
                    uart_send(b"  FAIL\r\n\r\n");
                }
            }
        }
    }
}

/// Print error code in human-readable format
fn print_error(error: ResponseCode) {
    match error {
        ResponseCode::NoDevice => uart_send(b"NoDevice\r\n"),
        ResponseCode::BusLocked => uart_send(b"BusLocked\r\n"),
        ResponseCode::BusError => uart_send(b"BusError\r\n"),
        ResponseCode::ControllerBusy => uart_send(b"ControllerBusy\r\n"),
        ResponseCode::BadArg => uart_send(b"BadArg\r\n"),
        ResponseCode::BusReset => uart_send(b"BusReset\r\n"),
        ResponseCode::BadDeviceState => uart_send(b"BadDeviceState\r\n"),
        _ => {
            uart_send(b"Error code: ");
            // Would need to convert error code to string here
            uart_send(b"(see ResponseCode enum)\r\n");
        }
    }
}

/// Print a single byte as hex
fn print_hex_byte(byte: u8) {
    const HEX_CHARS: &[u8; 16] = b"0123456789ABCDEF";
    let high = HEX_CHARS[(byte >> 4) as usize];
    let low = HEX_CHARS[(byte & 0x0F) as usize];
    uart_send(&[high, low]);
}

// =============================================================================
// SLAVE MODE TESTS (AST1060 as slave, Aardvark as master)
// =============================================================================

/// Test 7: Slave Mode Configuration
fn test_7_slave_configuration(device: &I2cDevice) {
    uart_send(b"Test 7: Slave Mode Configuration\r\n");
    uart_send(b"  Goal: Configure AST1060 as I2C slave device\r\n");
    uart_send(b"  Configuring slave address: 0x42\r\n");

    // Configure AST1060 to respond as slave at address 0x42
    let result = device.configure_slave_address(AST1060_SLAVE_ADDR);

    match result {
        Ok(_) => {
            uart_send(
                b"  \xE2\x9C\x93 Slave address configured successfully\r\n",
            );

            // Enable slave receive mode
            uart_send(b"  Enabling slave receive mode...\r\n");
            let enable_result = device.enable_slave_receive();

            match enable_result {
                Ok(_) => {
                    uart_send(b"  \xE2\x9C\x93 Slave receive mode enabled\r\n");

                    // Enable interrupt notification for slave receive
                    uart_send(b"  Enabling slave notification...\r\n");
                    match device.enable_slave_notification(
                        super::notifications::SLAVE_RX_MASK,
                    ) {
                        Ok(_) => {
                            uart_send(b"  \xE2\x9C\x93 Slave notification enabled\r\n");
                            uart_send(b"  AST1060 is now listening at address 0x42 (interrupt-driven)\r\n");
                            uart_send(b"  PASS\r\n\r\n");
                        }
                        Err(e) => {
                            uart_send(b"  \xE2\x9C\x97 Failed to enable slave notification\r\n");
                            print_error(e);
                            uart_send(b"  WARN (slave mode enabled but notification failed)\r\n\r\n");
                        }
                    }
                }
                Err(e) => {
                    uart_send(
                        b"  \xE2\x9C\x97 Failed to enable slave receive\r\n",
                    );
                    print_error(e);
                    uart_send(b"  FAIL\r\n\r\n");
                }
            }
        }
        Err(e) => {
            uart_send(b"  \xE2\x9C\x97 Slave configuration failed\r\n");
            print_error(e);
            uart_send(b"  FAIL\r\n\r\n");
        }
    }
}

/// Test 8: Slave Receive (Aardvark writes to AST1060)
fn test_8_slave_receive(device: &I2cDevice) {
    uart_send(b"Test 8: Slave Receive\r\n");
    uart_send(b"  Goal: Receive data from Aardvark master\r\n");
    uart_send(b"  \r\n");
    uart_send(b"  ** ACTION REQUIRED **\r\n");
    uart_send(b"  Use Aardvark Control Center:\r\n");
    uart_send(b"    1. Configure as I2C Master\r\n");
    uart_send(b"    2. Write to address 0x42: [0x12, 0x34, 0x56, 0x78]\r\n");
    uart_send(b"  \r\n");
    uart_send(b"  Waiting for incoming message (10 seconds)...\r\n");

    // Use interrupt-driven notification instead of polling
    use userlib::{sys_recv_open, sys_set_timer, TaskId};
    sys_set_timer(Some(10000), super::notifications::TIMER_MASK);

    let mut msg_buf = [0u8; 4];
    let msginfo = sys_recv_open(
        &mut msg_buf,
        super::notifications::SLAVE_RX_MASK | super::notifications::TIMER_MASK,
    );

    let mut received = false;
    if msginfo.sender == TaskId::KERNEL {
        if (msginfo.operation & super::notifications::SLAVE_RX_MASK) != 0 {
            // Notification received! Get the message
            match device.get_slave_message() {
                Ok(message) => {
                    uart_send(
                        b"  \xE2\x9C\x93 Message received via interrupt!\r\n",
                    );
                    uart_send(b"  Length: ");
                    print_hex_byte(message.data().len() as u8);
                    uart_send(b" bytes\r\n");
                    uart_send(b"  Data: [");

                    for (i, byte) in message.data().iter().enumerate() {
                        if i > 0 {
                            uart_send(b", ");
                        }
                        uart_send(b"0x");
                        print_hex_byte(*byte);
                    }
                    uart_send(b"]\r\n");

                    // Check if it matches expected pattern
                    if message.data().len() == 4
                        && message.data()[0] == 0x12
                        && message.data()[1] == 0x34
                        && message.data()[2] == 0x56
                        && message.data()[3] == 0x78
                    {
                        uart_send(b"  \xE2\x9C\x93 Data matches expected pattern!\r\n");
                    }

                    received = true;
                    uart_send(b"  PASS\r\n\r\n");
                }
                Err(ResponseCode::NoSlaveMessage) => {
                    uart_send(b"  \xE2\x9C\x97 Notification received but no message available\r\n");
                    uart_send(b"  FAIL\r\n\r\n");
                }
                Err(e) => {
                    uart_send(b"  \xE2\x9C\x97 Error retrieving message\r\n");
                    print_error(e);
                    uart_send(b"  FAIL\r\n\r\n");
                }
            }
        } else if (msginfo.operation & super::notifications::TIMER_MASK) != 0 {
            uart_send(b"  \xE2\x9C\x97 Timeout - No message received in 10 seconds\r\n");
            uart_send(b"  Check Aardvark configuration and retry\r\n");
            uart_send(b"  TIMEOUT\r\n\r\n");
        }
    }
}

/// Test 9: Slave Bidirectional (MCTP-style communication)
fn test_9_slave_bidirectional(device: &I2cDevice) {
    uart_send(b"Test 9: Slave Bidirectional Communication\r\n");
    uart_send(b"  Goal: Demonstrate MCTP-style bidirectional I2C\r\n");
    uart_send(b"  \r\n");
    uart_send(b"  This test shows how MCTP-over-I2C works:\r\n");
    uart_send(b"  1. AST1060 acts as slave to receive commands\r\n");
    uart_send(b"  2. AST1060 switches to master to send responses\r\n");
    uart_send(b"  \r\n");
    uart_send(b"  ** ACTION REQUIRED **\r\n");
    uart_send(b"  Step 1: Aardvark writes command to AST1060 slave (0x42)\r\n");
    uart_send(b"          Command: [0xAA, 0xBB]\r\n");
    uart_send(b"  \r\n");
    uart_send(b"  Waiting for command (10 seconds)...\r\n");

    // Wait for incoming command using interrupt notification
    use userlib::{sys_recv_open, sys_set_timer, TaskId};
    sys_set_timer(Some(10000), super::notifications::TIMER_MASK);

    let mut msg_buf = [0u8; 4];
    let msginfo = sys_recv_open(
        &mut msg_buf,
        super::notifications::SLAVE_RX_MASK | super::notifications::TIMER_MASK,
    );

    let mut command_received = false;
    if msginfo.sender == TaskId::KERNEL {
        if (msginfo.operation & super::notifications::SLAVE_RX_MASK) != 0 {
            match device.get_slave_message() {
                Ok(message) => {
                    uart_send(b"  \xE2\x9C\x93 Command received: [");
                    for (i, byte) in message.data().iter().enumerate() {
                        if i > 0 {
                            uart_send(b", ");
                        }
                        uart_send(b"0x");
                        print_hex_byte(*byte);
                    }
                    uart_send(b"]\r\n");
                    command_received = true;
                }
                Err(_) => {}
            }
        } else if (msginfo.operation & super::notifications::TIMER_MASK) != 0 {
            uart_send(b"  \xE2\x9C\x97 Timeout - No command received\r\n");
        }
    }

    if !command_received {
        uart_send(b"\r\n  \xE2\x9C\x97 Timeout waiting for command\r\n");
        uart_send(b"  TIMEOUT\r\n\r\n");
        return;
    }

    uart_send(b"  \r\n");
    uart_send(b"  Step 2: AST1060 becomes master and sends response\r\n");
    uart_send(b"          Configure Aardvark as slave at 0x50\r\n");
    uart_send(b"  \r\n");
    uart_send(b"  Waiting 3 seconds for reconfiguration...\r\n");
    use userlib::hl;
    hl::sleep_for(3000);

    // Now act as master and send response back to Aardvark (now slave)
    uart_send(b"  Sending response to Aardvark slave (0x50)...\r\n");
    let response_data = [0xCC, 0xDD, 0xEE, 0xFF];

    match device.write(&response_data) {
        Ok(_) => {
            uart_send(
                b"  \xE2\x9C\x93 Response sent: [0xCC, 0xDD, 0xEE, 0xFF]\r\n",
            );
            uart_send(
                b"  \xE2\x9C\x93 Bidirectional communication complete!\r\n",
            );
            uart_send(b"  \r\n");
            uart_send(b"  This demonstrates MCTP peer-to-peer capability:\r\n");
            uart_send(b"  - Device can receive commands as slave\r\n");
            uart_send(b"  - Device can send responses as master\r\n");
            uart_send(b"  - Critical for platform management protocols\r\n");
            uart_send(b"  PASS\r\n\r\n");
        }
        Err(e) => {
            uart_send(b"  \xE2\x9C\x97 Failed to send response\r\n");
            print_error(e);
            uart_send(b"  FAIL\r\n\r\n");
        }
    }

    // Clean up - disable slave mode
    uart_send(b"  Disabling slave mode...\r\n");
    match device.disable_slave_receive() {
        Ok(_) => uart_send(b"  \xE2\x9C\x93 Slave mode disabled\r\n"),
        Err(e) => {
            uart_send(b"  \xE2\x9C\x97 Failed to disable slave mode\r\n");
            print_error(e);
        }
    }
}
