use crate::{Controller, ResponseCode, SlaveConfig};
use serde::{Deserialize, Serialize};

/// I2C bus speed configurations
#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum I2cSpeed {
    /// Standard mode: 100 kHz
    Standard,
    /// Fast mode: 400 kHz  
    Fast,
    /// Fast mode plus: 1 MHz
    FastPlus,
    /// High speed mode: 3.4 MHz
    HighSpeed,
}

/// Hardware abstraction trait for I2C controllers
///
/// This trait provides a platform-agnostic interface for I2C hardware operations,
/// enabling the I2C server to work across different microcontroller families
/// while maintaining the same high-level business logic.
///
/// # Design Principles
///
/// - **Hardware Agnostic**: Works across STM32, LPC55, RISC-V, and other platforms
/// - **Error Transparent**: Uses existing ResponseCode taxonomy for consistency  
/// - **Operation Atomic**: Each method represents a complete I2C transaction
/// - **Resource Safe**: Handles controller enable/disable and bus recovery
///
/// # Implementation Notes
///
/// Platform-specific implementations should handle:
/// - Register programming for the target microcontroller
/// - Interrupt management and timing
/// - GPIO pin configuration and alternate function setup
/// - Clock tree configuration for the I2C peripheral
/// - Bus recovery procedures (SCL/SDA manipulation)
pub trait I2cHardware {
    /// Hardware-specific error type that can be converted to ResponseCode
    type Error: Into<ResponseCode>;

    /// Perform a write followed by read operation on the I2C bus
    ///
    /// This is the fundamental I2C operation supporting both simple reads/writes
    /// and complex register-based device interactions.
    ///
    /// # Arguments
    ///
    /// * `controller` - Which I2C controller to use (I2C0, I2C1, etc.)
    /// * `addr` - 7-bit I2C device address
    /// * `write_data` - Data to write to the device (empty slice for read-only)
    /// * `read_buffer` - Buffer to fill with read data (empty slice for write-only)
    ///
    /// # Returns
    ///
    /// Number of bytes successfully read, or hardware-specific error
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// // Read register 0x42 from device at address 0x50
    /// let mut value = [0u8; 2];
    /// let count = hw.write_read(Controller::I2C0, 0x50, &[0x42], &mut value)?;
    ///
    /// // Write-only operation
    /// hw.write_read(Controller::I2C0, 0x50, &[0x10, 0xFF], &mut [])?;
    ///
    /// // Read-only operation  
    /// let mut data = [0u8; 4];
    /// hw.write_read(Controller::I2C0, 0x50, &[], &mut data)?;
    /// ```
    fn write_read(
        &mut self,
        controller: Controller,
        addr: u8,
        write_data: &[u8],
        read_buffer: &mut [u8],
    ) -> Result<usize, Self::Error>;

    /// Perform an SMBus block read operation
    ///
    /// In SMBus block read, the device returns a byte count followed by that
    /// many data bytes. This is commonly used for reading variable-length
    /// data from smart devices.
    ///
    /// # Arguments
    ///
    /// * `controller` - Which I2C controller to use  
    /// * `addr` - 7-bit I2C device address
    /// * `write_data` - Command/register to write before reading
    /// * `read_buffer` - Buffer to fill with block data (without length byte)
    ///
    /// # Returns
    ///
    /// Number of actual data bytes read (excluding the length byte)
    fn write_read_block(
        &mut self,
        controller: Controller,
        addr: u8,
        write_data: &[u8],
        read_buffer: &mut [u8],
    ) -> Result<usize, Self::Error>;

    /// Configure I2C bus timing for the specified speed
    ///
    /// This configures the I2C controller's clock dividers and timing parameters
    /// to achieve the target bus frequency.
    ///
    /// # Arguments
    ///
    /// * `controller` - Which I2C controller to configure
    /// * `speed` - Target bus speed (Standard, Fast, FastPlus, HighSpeed)
    fn configure_timing(
        &mut self,
        controller: Controller,
        speed: I2cSpeed,
    ) -> Result<(), Self::Error>;

    /// Reset and recover a locked I2C bus
    ///
    /// When I2C transactions fail or devices misbehave, the bus can become
    /// locked with SDA held low. This method attempts recovery by:
    /// - Switching pins to GPIO mode
    /// - Generating clock pulses to complete any partial transactions
    /// - Sending a STOP condition
    /// - Restoring I2C alternate function mode
    ///
    /// # Arguments
    ///
    /// * `controller` - Which I2C controller/bus to reset
    fn reset_bus(&mut self, controller: Controller) -> Result<(), Self::Error>;

    /// Enable I2C controller hardware and configure pins
    ///
    /// This method handles platform-specific initialization:
    /// - Enable peripheral clocks  
    /// - Configure GPIO pins for I2C alternate function
    /// - Initialize I2C controller registers
    /// - Enable interrupts if using interrupt-driven mode
    ///
    /// # Arguments
    ///
    /// * `controller` - Which I2C controller to enable
    fn enable_controller(
        &mut self,
        controller: Controller,
    ) -> Result<(), Self::Error>;

    /// Disable I2C controller and return pins to GPIO mode
    ///
    /// This provides clean shutdown and power savings:
    /// - Disable I2C controller
    /// - Return GPIO pins to input/floating state  
    /// - Disable peripheral clocks
    /// - Clear any pending interrupts
    ///
    /// # Arguments
    ///
    /// * `controller` - Which I2C controller to disable
    fn disable_controller(
        &mut self,
        controller: Controller,
    ) -> Result<(), Self::Error>;

    // =========================================================================
    // Slave Mode Operations for MCTP and Peer-to-Peer Protocols
    // =========================================================================

    /// Configure I2C controller to operate as a slave device
    ///
    /// This enables the controller to respond to incoming I2C transactions from
    /// other masters on the bus. Essential for MCTP-over-I2C implementations
    /// where devices need bidirectional communication.
    ///
    /// # Hardware Implementation Requirements
    ///
    /// - Program slave address registers (primary and optionally secondary)
    /// - Enable slave mode interrupts or polling mechanism
    /// - Configure receive buffers for incoming data
    /// - Set up address matching (7-bit, general call, etc.)
    /// - Enable slave acknowledge generation
    ///
    /// # Arguments
    ///
    /// * `controller` - Which I2C controller to configure
    /// * `config` - Slave configuration (address, port, etc.)
    ///
    /// # MCTP Considerations
    ///
    /// For MCTP-over-I2C, the slave address typically represents the device's
    /// Endpoint ID (EID) in the MCTP network topology.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// // Configure as MCTP endpoint with EID 0x1D
    /// let config = SlaveConfig::new(Controller::I2C0, PortIndex::new(0), 0x1D)?;
    /// hw.configure_slave_mode(Controller::I2C0, &config)?;
    /// ```
    fn configure_slave_mode(
        &mut self,
        controller: Controller,
        config: &SlaveConfig,
    ) -> Result<(), Self::Error>;

    /// Enable slave receive mode to start listening for incoming messages
    ///
    /// After configuring the slave address, this method activates the hardware
    /// to begin receiving and buffering incoming I2C transactions addressed to
    /// this device.
    ///
    /// # Hardware Implementation Requirements
    ///
    /// - Enable slave mode operation in controller registers
    /// - Start address matching logic
    /// - Enable interrupts for slave events (address match, data receive, stop)
    /// - Initialize receive buffers and state machines
    /// - Configure clock stretching if supported
    ///
    /// # Arguments
    ///
    /// * `controller` - Which I2C controller to enable
    fn enable_slave_receive(
        &mut self,
        controller: Controller,
    ) -> Result<(), Self::Error>;

    /// Disable slave receive mode
    ///
    /// Stops the controller from responding to incoming slave transactions.
    /// This is useful for power management or when switching communication modes.
    ///
    /// # Hardware Implementation Requirements
    ///
    /// - Disable slave mode operation
    /// - Stop address matching
    /// - Disable slave interrupts
    /// - Clear any pending slave state
    /// - Optionally flush receive buffers
    ///
    /// # Arguments
    ///
    /// * `controller` - Which I2C controller to disable
    fn disable_slave_receive(
        &mut self,
        controller: Controller,
    ) -> Result<(), Self::Error>;

    /// Check if a specific controller has slave data available
    ///
    /// This is used by interrupt handlers to determine which controller
    /// triggered an interrupt and has data ready for processing.
    ///
    /// # Arguments
    ///
    /// * `controller` - Which I2C controller to check
    ///
    /// # Returns
    ///
    /// `true` if the controller has received slave data, `false` otherwise
    ///
    /// # Default Implementation
    ///
    /// Hardware without slave interrupt support should return `false`.
    fn check_slave_data(&self, _controller: Controller) -> bool {
        false
    }

    /// Clear slave interrupts for a specific controller
    ///
    /// Called after processing slave data to acknowledge the interrupt
    /// and allow new interrupts to be generated.
    ///
    /// # Arguments
    ///
    /// * `controller` - Which I2C controller to clear interrupts for
    ///
    /// # Default Implementation
    ///
    /// Hardware without slave interrupt support can use the default no-op.
    fn clear_slave_interrupts(&self, _controller: Controller) {}

    /// Read slave data from hardware
    ///
    /// Called when an interrupt indicates slave data is available.
    /// Returns the source address and number of bytes read.
    ///
    /// # Arguments
    ///
    /// * `controller` - Which I2C controller to read from
    /// * `buffer` - Buffer to store received data (up to 255 bytes)
    ///
    /// # Returns
    ///
    /// `Ok((source_address, data_length))` with the master's address and bytes read,
    /// or an error if the read fails.
    fn read_slave_data(
        &mut self,
        controller: Controller,
        buffer: &mut [u8],
    ) -> Result<(u8, usize), Self::Error>;
}
