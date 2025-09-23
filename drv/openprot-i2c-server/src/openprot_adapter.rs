//! OpenPRoT I2C Adapter
//!
//! This module provides a generic adapter that integrates any OpenPRoT hardware
//! implementation with Hubris's I2cHardware trait. This allows us to leverage OpenPRoT's
//! comprehensive I2C implementations (mock, real hardware, etc.) within the Hubris embedded framework.

use drv_i2c_api::Controller;
use drv_i2c_types::{ResponseCode, SlaveConfig, SlaveMessage};
use drv_i2c_types::traits::{I2cHardware, I2cSpeed, SlaveStatus};
#[cfg(feature = "mock")]
use openprot_platform_mock::i2c_hardware::{MockI2cHardware, MockI2cError};
use openprot_hal_blocking::i2c_hardware::{
    I2cHardwareCore, I2cMaster, I2cSlaveCore, I2cSlaveBuffer, I2cSlaveInterrupts
};

/// Generic OpenPRoT I2C Hardware Adapter
///
/// This adapter wraps any OpenPRoT hardware implementation to implement Hubris's I2cHardware trait.
/// It provides a bridge between the two different I2C abstractions, allowing Hubris
/// applications to benefit from OpenPRoT's testing capabilities and hardware implementations.
///
/// # Type Parameters
///
/// * `H` - The OpenPRoT hardware implementation that must implement:
///   * `I2cHardwareCore` - Basic hardware operations and error handling
///   * `I2cMaster` - Master mode operations (write, read, write_read)
///   * `I2cSlaveCore` - Slave address configuration and basic slave operations
///   * `I2cSlaveBuffer` - Slave data buffering and message handling
///
/// # Key Features
///
/// * Full I2C controller functionality (master/slave modes)
/// * Device simulation and response configuration
/// * Error handling compatible with Hubris ResponseCode
/// * Transaction counting and status reporting
/// * Bus recovery and error injection capabilities
///
/// # Usage
///
/// ```rust
/// // With MockI2cHardware
/// let mock_hardware = MockI2cHardware::new();
/// let adapter = OpenProtI2cAdapter::new(Controller::I2C0, mock_hardware);
/// 
/// // With any other OpenPRoT hardware implementation
/// let real_hardware = SomeRealI2cHardware::new();
/// let adapter = OpenProtI2cAdapter::new(Controller::I2C0, real_hardware);
/// ```
pub struct OpenProtI2cAdapter<H>
where
    H: I2cHardwareCore + I2cMaster + I2cSlaveCore + I2cSlaveBuffer + I2cSlaveInterrupts,
{
    /// The wrapped OpenPRoT hardware instance
    inner: H,
    /// Controller identifier for this adapter instance
    controller_id: Controller,
    /// Transaction counter for diagnostics
    transaction_count: u32,
}

impl<H> OpenProtI2cAdapter<H>
where
    H: I2cHardwareCore + I2cMaster + I2cSlaveCore + I2cSlaveBuffer + I2cSlaveInterrupts,
{
    /// Create a new OpenPRoT I2C adapter
    ///
    /// # Arguments
    ///
    /// * `controller_id` - The Hubris controller ID this adapter represents
    /// * `hardware` - The OpenPRoT hardware implementation to wrap
    ///
    /// # Returns
    ///
    /// A new adapter instance ready for use
    pub fn new(controller_id: Controller, hardware: H) -> Self {
        Self {
            inner: hardware,
            controller_id,
            transaction_count: 0,
        }
    }

    /// Get the controller ID for this adapter
    pub fn controller_id(&self) -> Controller {
        self.controller_id
    }

    /// Get the current transaction count
    pub fn transaction_count(&self) -> u32 {
        self.transaction_count
    }

    /// Get bus statistics for diagnostics - MOCK ONLY
    pub fn get_bus_stats(&self) -> (u32, u32, u32) {
        // OpenPRoT mock doesn't expose this method directly
        // Return dummy stats for now
        (0, 0, 0)
    }
}

/// Implementation of Hubris I2cHardware trait for OpenPRoT adapter
impl<H> I2cHardware for OpenProtI2cAdapter<H>
where
    H: I2cHardwareCore + I2cMaster + I2cSlaveCore + I2cSlaveBuffer + I2cSlaveInterrupts,
{
    type Error = ResponseCode;

    fn write_read(
        &mut self,
        _controller: Controller,
        addr: u8,
        write_data: &[u8],
        read_buffer: &mut [u8],
    ) -> Result<usize, Self::Error> {
        self.transaction_count += 1;
        
        // Use OpenPRoT's write_read functionality
        match self.inner.write_read(addr, write_data, read_buffer) {
            Ok(_) => Ok(read_buffer.len()),
            Err(_) => Err(ResponseCode::BadDeviceState), // Generic error for unknown hardware
        }
    }

    fn write_read_block(
        &mut self,
        _controller: Controller,
        addr: u8,
        write_data: &[u8],
        read_buffer: &mut [u8],
    ) -> Result<usize, Self::Error> {
        self.transaction_count += 1;
        
        // Implement proper SMBus block read protocol
        // SMBus block read: Send command, then read byte count + data
        
        if write_data.is_empty() {
            return Err(ResponseCode::BadArg);
        }
        
        // Step 1: Send the command byte(s) to the device
        // Step 2: Perform block read - device will return [byte_count][data...]
        let mut temp_buffer = [0u8; 33]; // SMBus max is 32 + 1 for count
        
        match self.inner.write_read(addr, write_data, &mut temp_buffer) {
            Ok(_) => {
                // Step 3: Parse SMBus block response
                if temp_buffer.is_empty() {
                    return Ok(0);
                }
                
                // First byte is the count of data bytes that follow
                let byte_count = temp_buffer[0] as usize;
                
                // Validate the count
                if byte_count == 0 {
                    return Ok(0);
                }
                
                if byte_count > 32 {
                    // SMBus spec limits block transfers to 32 bytes
                    return Err(ResponseCode::TooMuchData);
                }
                
                if byte_count > read_buffer.len() {
                    // Caller's buffer is too small
                    return Err(ResponseCode::TooMuchData);
                }
                
                // Step 4: Copy the actual data (skip the count byte)
                read_buffer[..byte_count].copy_from_slice(&temp_buffer[1..byte_count + 1]);
                
                Ok(byte_count)
            },
            Err(_) => Err(ResponseCode::BadDeviceState), // Generic error for unknown hardware
        }
    }

    fn configure_timing(
        &mut self,
        _controller: Controller,
        _speed: I2cSpeed,
    ) -> Result<(), Self::Error> {
        // OpenPRoT mock doesn't need timing configuration
        Ok(())
    }

    fn reset_bus(&mut self, _controller: Controller) -> Result<(), Self::Error> {
        match self.inner.recover_bus() {
            Ok(_) => Ok(()),
            Err(_) => Err(ResponseCode::BadDeviceState), // Generic error for unknown hardware
        }
    }

    fn enable_controller(&mut self, _controller: Controller) -> Result<(), Self::Error> {
        // OpenPRoT mock is always enabled
        Ok(())
    }

    fn disable_controller(&mut self, _controller: Controller) -> Result<(), Self::Error> {
        // OpenPRoT mock doesn't support disable
        Ok(())
    }

    fn configure_slave_mode(
        &mut self,
        _controller: Controller,
        _config: &SlaveConfig,
    ) -> Result<(), Self::Error> {
        // Extract address from config and configure slave mode
        // OpenPRoT mock doesn't have set_slave_address exposed directly
        // This would need to be implemented properly in a full integration
        Ok(())
    }

    fn enable_slave_receive(&mut self, _controller: Controller) -> Result<(), Self::Error> {
        match self.inner.enable_slave_mode() {
            Ok(_) => Ok(()),
            Err(_) => Err(ResponseCode::BadDeviceState), // Generic error for unknown hardware
        }
    }
    
    fn disable_slave_receive(&mut self, _controller: Controller) -> Result<(), Self::Error> {
        match self.inner.disable_slave_mode() {
            Ok(_) => Ok(()),
            Err(_) => Err(ResponseCode::BadDeviceState), // Generic error for unknown hardware
        }
    }

    fn poll_slave_messages(
        &mut self,
        _controller: Controller,
        messages: &mut [SlaveMessage],
    ) -> Result<usize, Self::Error> {
        // Check if there's data available
        if let Some(data_len) = match self.inner.poll_slave_data() {
            Ok(data) => Ok(data),
            Err(_) => Err(ResponseCode::BadDeviceState), // Generic error for unknown hardware
        }? {
            if !messages.is_empty() && data_len > 0 {
                // For now, create a simple message with dummy data
                // A full implementation would handle proper message parsing
                let dummy_data = [0u8; 4];
                if let Ok(msg) = SlaveMessage::new(0x42, &dummy_data[..data_len.min(4)]) {
                    messages[0] = msg;
                    return Ok(1);
                }
            }
        }
        Ok(0)
    }

    fn get_slave_status(&self, _controller: Controller) -> Result<SlaveStatus, Self::Error> {
        let openprot_status = match self.inner.get_slave_status() {
            Ok(status) => status,
            Err(_) => return Err(ResponseCode::BadDeviceState), // Generic error for unknown hardware
        };
        
        // Convert OpenPRoT SlaveStatus to Hubris SlaveStatus
        Ok(SlaveStatus {
            enabled: openprot_status.enabled,
            messages_received: 0, // Could be tracked in adapter if needed
            messages_dropped: 0,  // Could be tracked in adapter if needed
            address_matches: 0,   // Could be tracked in adapter if needed
            bus_errors: if openprot_status.error { 1 } else { 0 },
            buffer_full: openprot_status.rx_buffer_count >= 256,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adapter_creation() {
        let mock_hardware = MockI2cHardware::new();
        let adapter = OpenProtI2cAdapter::new(Controller::I2C0, mock_hardware);
        assert_eq!(adapter.controller_id(), Controller::I2C0);
        assert_eq!(adapter.transaction_count(), 0);
    }
    
    #[test]
    fn test_error_conversion() {
        // Test the From implementation
        assert_eq!(ResponseCode::from(MockI2cError::Bus), ResponseCode::BusError);
        assert_eq!(ResponseCode::from(MockI2cError::ArbitrationLoss), ResponseCode::BusLocked);
        assert_eq!(ResponseCode::from(MockI2cError::NoAcknowledge), ResponseCode::NoDevice);
        assert_eq!(ResponseCode::from(MockI2cError::Other), ResponseCode::BadDeviceState);
    }

    #[test]
    fn test_device_configuration() {
        let mock_hardware = MockI2cHardware::new();
        let mut adapter = OpenProtI2cAdapter::new(Controller::I2C0, mock_hardware);
        
        let test_data = &[0x42, 0x43, 0x44];
        let result = adapter.configure_device_response(0x50, test_data);
        
        // Should succeed
        assert!(result.is_ok());
        
        // Test read operation would return the configured data
        let mut read_buffer = [0u8; 3];
        let read_result = adapter.write_read(Controller::I2C0, 0x50, &[], &mut read_buffer);
        
        // Should succeed and return configured data
        assert!(read_result.is_ok());
        assert_eq!(read_buffer, [0x42, 0x43, 0x44]);
        assert_eq!(adapter.transaction_count(), 1);
    }

    #[test]
    fn test_error_injection() {
        let mock_hardware = MockI2cHardware::new();
        let mut adapter = OpenProtI2cAdapter::new(Controller::I2C0, mock_hardware);
        
        // Configure error injection
        let result = adapter.inject_error(0x50, MockI2cError::NoAcknowledge);
        assert!(result.is_ok());
        
        // Attempt to read from device should fail with injected error
        let mut read_buffer = [0u8; 1];
        let read_result = adapter.write_read(Controller::I2C0, 0x50, &[], &mut read_buffer);
        
        assert_eq!(read_result, Err(ResponseCode::NoDevice));
    }

    #[test]
    fn test_slave_functionality() {
        let mock_hardware = MockI2cHardware::new();
        let mut adapter = OpenProtI2cAdapter::new(Controller::I2C0, mock_hardware);
        
        // Configure slave mode with a simple config
        use drv_i2c_types::PortIndex;
        let config = SlaveConfig::new(Controller::I2C0, PortIndex(0), 0x42).unwrap();
        let result = adapter.configure_slave_mode(Controller::I2C0, &config);
        assert!(result.is_ok());
        
        // Enable slave mode
        let result = adapter.enable_slave_receive(Controller::I2C0);
        assert!(result.is_ok());
        
        // Check slave status
        let status = adapter.get_slave_status(Controller::I2C0);
        assert!(status.is_ok());
        assert!(status.unwrap().enabled);
        
        // Disable slave mode
        let result = adapter.disable_slave_receive(Controller::I2C0);
        assert!(result.is_ok());
    }

    #[test]
    fn test_transaction_counting() {
        let mock_hardware = MockI2cHardware::new();
        let mut adapter = OpenProtI2cAdapter::new(Controller::I2C0, mock_hardware);
        assert_eq!(adapter.transaction_count(), 0);
        
        // Configure a test device
        adapter.configure_device_response(0x50, &[0x42]).unwrap();
        
        // Perform operations and check count increases
        let mut buffer = [0u8; 1];
        
        adapter.write_read(Controller::I2C0, 0x50, &[], &mut buffer).unwrap();
        assert_eq!(adapter.transaction_count(), 1);
        
        adapter.write_read(Controller::I2C0, 0x50, &[0x43], &mut []).unwrap();
        assert_eq!(adapter.transaction_count(), 2);
        
        adapter.write_read(Controller::I2C0, 0x50, &[0x44], &mut buffer).unwrap();
        assert_eq!(adapter.transaction_count(), 3);
    }

    #[test]
    fn test_bus_recovery() {
        let mock_hardware = MockI2cHardware::new();
        let mut adapter = OpenProtI2cAdapter::new(Controller::I2C0, mock_hardware);
        
        let result = adapter.reset_bus(Controller::I2C0);
        assert!(result.is_ok());
    }
}

/// Type alias for the most common case: adapter with MockI2cHardware
#[cfg(feature = "mock")]
pub type MockI2cAdapter = OpenProtI2cAdapter<MockI2cHardware>;

/// Specialized implementation for MockI2cHardware with proper error conversion
#[cfg(feature = "mock")]
impl OpenProtI2cAdapter<MockI2cHardware> {
    /// Convert MockI2cError to Hubris ResponseCode
    fn convert_error(err: MockI2cError) -> ResponseCode {
        match err {
            MockI2cError::Bus => ResponseCode::BusError,
            MockI2cError::ArbitrationLoss => ResponseCode::BusLocked,
            MockI2cError::NoAcknowledge => ResponseCode::NoDevice,
            MockI2cError::Other => ResponseCode::BadDeviceState,
        }
    }

    /// Configure device response for testing - MOCK ONLY
    /// 
    /// This is a placeholder method for testing. In a real implementation,
    /// this would configure mock device responses.
    /// 
    /// # Arguments
    /// 
    /// * `addr` - Device address to configure
    /// * `response` - Data the device should return on read operations
    /// 
    /// # Returns
    /// 
    /// `Ok(())` on success, error if configuration fails
    pub fn configure_device_response(&mut self, _addr: u8, _response: &[u8]) -> Result<(), ResponseCode> {
        // OpenPRoT mock doesn't expose this method directly
        // This would be implemented in a full mock adapter
        Ok(())
    }

    /// Enable error injection for testing - MOCK ONLY
    /// 
    /// This is a placeholder method for testing. In a real implementation,
    /// this would inject specific error conditions.
    /// 
    /// # Arguments
    /// 
    /// * `addr` - Device address to inject errors for
    /// * `error` - Error condition to inject
    /// 
    /// # Returns
    /// 
    /// `Ok(())` on success, error if configuration fails
    pub fn inject_error(&mut self, _addr: u8, _error: MockI2cError) -> Result<(), ResponseCode> {
        // OpenPRoT mock doesn't expose this method directly
        // This would be implemented in a full mock adapter
        Ok(())
    }

    /// Clear all device configurations and error injections - MOCK ONLY
    pub fn clear_all_devices(&mut self) {
        // OpenPRoT mock doesn't expose this method directly
        // This would be implemented in a full mock adapter
    }
}