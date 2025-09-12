//! Minimal mock I2C hardware for IPC testing

use crate::{Controller, ResponseCode, ReservedAddress, I2cSpeed, I2cHardware};
use num_traits::FromPrimitive;

/// Minimal mock I2C hardware for IPC testing
/// 
/// This provides just enough I2C simulation to test IPC message handling
/// without real hardware. Focus is on protocol testing, not device simulation.
#[derive(Debug)]
pub struct MockI2cHardware {
    /// Simple device list - just addresses that respond
    responding_devices: [(Controller, u8); 8],
    /// Number of devices that respond
    device_count: u8,
    /// Controller enable state (bitfield: bit N = controller N enabled)
    enabled_controllers: u8,
    /// Force next operation to fail with this error
    force_error: Option<ResponseCode>,
}

impl Default for MockI2cHardware {
    fn default() -> Self {
        Self::new()
    }
}

impl MockI2cHardware {
    pub fn new() -> Self {
        Self {
            responding_devices: [(Controller::I2C0, 0); 8],  // Initialize with dummy values
            device_count: 0,
            enabled_controllers: 0,
            force_error: None,
        }
    }

    /// Add a device that will respond to I2C operations
    pub fn add_device(&mut self, controller: Controller, addr: u8) {
        if (self.device_count as usize) < self.responding_devices.len() {
            self.responding_devices[self.device_count as usize] = (controller, addr);
            self.device_count += 1;
        }
    }

    /// Make next operation fail with specified error
    pub fn inject_error(&mut self, error: ResponseCode) {
        self.force_error = Some(error);
    }

    /// Check if device exists and responds
    fn device_responds(&self, controller: Controller, addr: u8) -> bool {
        for &(dev_controller, dev_addr) in &self.responding_devices[..self.device_count as usize] {
            if dev_controller == controller && dev_addr == addr {
                return true;
            }
        }
        false
    }

    /// Check if controller is enabled
    fn controller_enabled(&self, controller: Controller) -> bool {
        let bit = controller as u8;
        if bit < 8 {
            (self.enabled_controllers & (1 << bit)) != 0
        } else {
            false
        }
    }
}

impl I2cHardware for MockI2cHardware {
    type Error = ResponseCode;

    fn write_read(
        &mut self,
        controller: Controller,
        addr: u8,
        _write_data: &[u8],
        read_buffer: &mut [u8],
    ) -> Result<usize, Self::Error> {
        // Check for injected errors first
        if let Some(error) = self.force_error.take() {
            return Err(error);
        }

        // Check controller is enabled
        if !self.controller_enabled(controller) {
            return Err(ResponseCode::BadController);
        }

        // Check for reserved addresses
        if ReservedAddress::from_u8(addr).is_some() {
            return Err(ResponseCode::ReservedAddress);
        }

        // Check if device responds
        if !self.device_responds(controller, addr) {
            return Err(ResponseCode::NoDevice);
        }

        // Simple success: fill read buffer with predictable pattern
        let bytes_read = read_buffer.len();
        for (i, slot) in read_buffer.iter_mut().enumerate() {
            *slot = (addr.wrapping_add(i as u8)).wrapping_add(0x10);
        }

        Ok(bytes_read)
    }

    fn write_read_block(
        &mut self,
        controller: Controller,
        addr: u8,
        write_data: &[u8],
        read_buffer: &mut [u8],
    ) -> Result<usize, Self::Error> {
        // For mock, block read is same as regular read
        self.write_read(controller, addr, write_data, read_buffer)
    }

    fn configure_timing(&mut self, _controller: Controller, _speed: I2cSpeed) -> Result<(), Self::Error> {
        // Mock always succeeds
        Ok(())
    }

    fn reset_bus(&mut self, _controller: Controller) -> Result<(), Self::Error> {
        // Clear any pending errors
        self.force_error = None;
        Ok(())
    }

    fn enable_controller(&mut self, controller: Controller) -> Result<(), Self::Error> {
        let bit = controller as u8;
        if bit < 8 {
            self.enabled_controllers |= 1 << bit;
            Ok(())
        } else {
            Err(ResponseCode::BadController)
        }
    }

    fn disable_controller(&mut self, controller: Controller) -> Result<(), Self::Error> {
        let bit = controller as u8;
        if bit < 8 {
            self.enabled_controllers &= !(1 << bit);
            Ok(())
        } else {
            Err(ResponseCode::BadController)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_mock_functionality() {
        let mut mock = MockI2cHardware::new();
        
        // Enable controller and add device
        mock.enable_controller(Controller::I2C0).unwrap();
        mock.add_device(Controller::I2C0, 0x50);

        // Test successful read
        let mut buf = [0u8; 4];
        let bytes_read = mock.write_read(Controller::I2C0, 0x50, &[], &mut buf).unwrap();
        
        assert_eq!(bytes_read, 4);
        assert_eq!(buf, [0x60, 0x61, 0x62, 0x63]); // 0x50 + 0x10 + index
    }

    #[test]
    fn test_error_injection() {
        let mut mock = MockI2cHardware::new();
        mock.enable_controller(Controller::I2C0).unwrap();
        mock.add_device(Controller::I2C0, 0x50);
        
        // Inject error
        mock.inject_error(ResponseCode::BusLocked);
        
        let mut buf = [0u8; 1];
        let result = mock.write_read(Controller::I2C0, 0x50, &[], &mut buf);
        
        assert_eq!(result, Err(ResponseCode::BusLocked));
    }

    #[test]
    fn test_no_device_error() {
        let mut mock = MockI2cHardware::new();
        mock.enable_controller(Controller::I2C0).unwrap();
        // Don't add any devices
        
        let mut buf = [0u8; 1];
        let result = mock.write_read(Controller::I2C0, 0x50, &[], &mut buf);
        
        assert_eq!(result, Err(ResponseCode::NoDevice));
    }
}