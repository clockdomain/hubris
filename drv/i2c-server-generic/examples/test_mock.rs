// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Test example showing the generic I2C server with mock backend
//! 
//! This can be compiled and run on the host to demonstrate the hardware
//! abstraction working with the mock implementation.

use drv_i2c_types::{Controller, I2cHardware, I2cSpeed, mock::MockI2cHardware};

fn main() {
    println!("Testing Generic I2C Server with Mock Backend");
    
    // Create and configure mock hardware
    let mut hardware = MockI2cHardware::new();
    
    // Enable controllers
    hardware.enable_controller(Controller::I2C0).unwrap();
    hardware.enable_controller(Controller::I2C1).unwrap();
    
    // Configure timing
    hardware.configure_timing(Controller::I2C0, I2cSpeed::Fast).unwrap();
    
    // Add some test devices
    hardware.add_device(Controller::I2C0, 0x50);  // EEPROM-like
    hardware.add_device(Controller::I2C1, 0x48);  // Sensor-like
    
    println!("✅ Hardware initialization complete");
    
    // Test basic I2C operations
    test_basic_operations(&mut hardware);
    test_error_conditions(&mut hardware);
    
    println!("🎉 All tests passed! Generic I2C server works with mock backend.");
}

fn test_basic_operations(hardware: &mut MockI2cHardware) {
    println!("\n🔧 Testing basic I2C operations...");
    
    // Test successful read from device 0x50
    let mut read_buf = [0u8; 4];
    let bytes_read = hardware.write_read(
        Controller::I2C0,
        0x50,
        &[0x10],  // Write register address
        &mut read_buf
    ).expect("Read should succeed");
    
    println!("   Read {} bytes: {:02x?}", bytes_read, read_buf);
    assert_eq!(bytes_read, 4);
    assert_eq!(read_buf, [0x70, 0x71, 0x72, 0x73]);  // Expected pattern
    
    // Test write-only operation
    hardware.write_read(
        Controller::I2C1,
        0x48,
        &[0x01, 0xFF],  // Write data
        &mut []
    ).expect("Write should succeed");
    
    println!("   Write operation completed successfully");
}

fn test_error_conditions(hardware: &mut MockI2cHardware) {
    println!("\n⚠️  Testing error conditions...");
    
    // Test device not found
    let mut buf = [0u8; 1];
    let result = hardware.write_read(Controller::I2C0, 0x99, &[], &mut buf);
    assert!(result.is_err());
    println!("   ✅ NoDevice error handled correctly");
    
    // Test controller not enabled
    let result = hardware.write_read(Controller::I2C7, 0x50, &[], &mut buf);
    assert!(result.is_err());
    println!("   ✅ BadController error handled correctly");
    
    // Test error injection
    hardware.inject_error(drv_i2c_types::ResponseCode::BusLocked);
    let result = hardware.write_read(Controller::I2C0, 0x50, &[], &mut buf);
    assert!(result.is_err());
    println!("   ✅ Error injection working correctly");
    
    // Test bus reset clears errors
    hardware.reset_bus(Controller::I2C0).unwrap();
    let result = hardware.write_read(Controller::I2C0, 0x50, &[], &mut buf);
    assert!(result.is_ok());
    println!("   ✅ Bus reset clears error conditions");
}