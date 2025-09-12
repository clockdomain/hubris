// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Common types for the I2C server client API
//!
//! This crate works on both the host and embedded system, so it can be used in
//! host-side tests.

#![no_std]

// Mock implementation will use the same FixedMap pattern as STM32 server

use hubpack::SerializedSize;
use num_derive::FromPrimitive;
use num_traits::FromPrimitive as _;
use serde::{Deserialize, Serialize};

use derive_idol_err::IdolError;
use enum_kinds::EnumKind;

#[derive(FromPrimitive, Eq, PartialEq)]
pub enum Op {
    WriteRead = 1,

    /// In a `WriteReadBlock` operation, only the **final read** is an SMBus
    /// block operation.
    ///
    /// All writes and all other read operations are normal (non-block)
    /// operations.
    ///
    /// We don't need a special way to perform block writes, because they can be
    /// constructed by the caller without cooperation from the driver.
    /// Specifically, the caller can construct the array `[reg, size, data[0],
    /// data[1], ...]` and pass it to a normal `WriteRead` operation.
    ///
    /// If we encounter a device which requires multiple block reads in a row
    /// without interruption, this logic would not work, but that would be a
    /// very strange device indeed.
    WriteReadBlock = 2,
}

/// The response code returned from the I2C server.  These response codes pretty
/// specific, not because the caller is expected to necessarily handle them
/// differently, but to give upstack software some modicum of context
/// surrounding the error.
#[derive(
    Copy,
    Clone,
    Debug,
    EnumKind,
    FromPrimitive,
    Eq,
    PartialEq,
    IdolError,
    Serialize,
    Deserialize,
    SerializedSize,
    counters::Count,
)]
#[enum_kind(ResponseCodeU8, derive(counters::Count))]
#[repr(u32)]
pub enum ResponseCode {
    /// Bad response from server
    BadResponse = 1,
    /// Bad argument sent to server
    BadArg,
    /// Indicated I2C device is invalid
    NoDevice,
    /// Indicated I2C controller is invalid
    BadController,
    /// Device address is reserved
    ReservedAddress,
    /// Indicated port is invalid
    BadPort,
    /// Device does not have indicated register
    NoRegister,
    /// Indicated mux is an invalid mux identifier
    BadMux,
    /// Indicated segment is an invalid segment identifier
    BadSegment,
    /// Indicated mux does not exist on this controller
    MuxNotFound,
    /// Indicated segment does not exist on this controller
    SegmentNotFound,
    /// Segment disconnected during operation
    SegmentDisconnected,
    /// Mux disconnected during operation
    MuxDisconnected,
    /// No device at address used for mux in-band management
    MuxMissing,
    /// Register used for mux in-band management is invalid
    BadMuxRegister,
    /// I2C bus was spontaneously reset during operation
    BusReset,
    /// I2C bus was reset during a mux in-band management operation
    BusResetMux,
    /// I2C bus locked up and was reset
    BusLocked,
    /// I2C bus locked up during in-band management operation and was reset
    BusLockedMux,
    /// I2C controller appeared to be busy and was reset
    ControllerBusy,
    /// I2C bus error
    BusError,
    /// Bad device state of unknown origin
    BadDeviceState,
    /// Requested operation is not supported
    OperationNotSupported,
    /// Illegal number of leases
    IllegalLeaseCount,
    /// Too much data -- or not enough buffer
    TooMuchData,
}

///
/// The controller for a given I2C device. The numbering here should be
/// assumed to follow the numbering for the peripheral as described by the
/// microcontroller.
///
#[derive(
    Copy,
    Clone,
    Debug,
    FromPrimitive,
    Eq,
    PartialEq,
    SerializedSize,
    Serialize,
    Deserialize,
)]
#[repr(u8)]
pub enum Controller {
    I2C0 = 0,
    I2C1 = 1,
    I2C2 = 2,
    I2C3 = 3,
    I2C4 = 4,
    I2C5 = 5,
    I2C6 = 6,
    I2C7 = 7,
    Mock = 0xff,
}

#[derive(Copy, Clone, Debug, FromPrimitive, Eq, PartialEq)]
#[allow(clippy::unusual_byte_groupings)]
pub enum ReservedAddress {
    GeneralCall = 0b0000_000,
    CBUSAddress = 0b0000_001,
    FutureBus = 0b0000_010,
    FuturePurposes = 0b0000_011,
    HighSpeedReserved00 = 0b0000_100,
    HighSpeedReserved01 = 0b0000_101,
    HighSpeedReserved10 = 0b0000_110,
    HighSpeedReserved11 = 0b0000_111,
    TenBit00 = 0b1111_100,
    TenBit01 = 0b1111_101,
    TenBit10 = 0b1111_110,
    TenBit11 = 0b1111_111,
}

///
/// The port index for a given I2C device.  Some controllers can have multiple
/// ports (which themselves are connected to different I2C buses), but only
/// one port can be active at a time.  For these controllers, a port index
/// must be specified.  The mapping between these indices and values that make
/// sense in terms of the I2C controller (e.g., the lettered port) is
/// specified in the application configuration; to minimize confusion, the
/// letter should generally match the GPIO port of the I2C bus (assuming that
/// GPIO ports are lettered), but these values are in fact strings and can
/// take any value.  Note that if a given I2C controller straddles two ports,
/// the port of SDA should generally be used when naming the port; if a GPIO
/// port contains multiple SDAs on it from the same controller, the
/// letter/number convention should be used (e.g., "B1") -- but this is purely
/// convention.
///
#[derive(Copy, Clone, Debug, FromPrimitive, Eq, PartialEq)]
pub struct PortIndex(pub u8);

///
/// A multiplexer identifier for a given I2C bus.  Multiplexer identifiers
/// need not start at 0.
///
#[derive(
    Copy,
    Clone,
    Debug,
    FromPrimitive,
    Eq,
    PartialEq,
    SerializedSize,
    Serialize,
    Deserialize,
)]
#[repr(u8)]
pub enum Mux {
    M1 = 1,
    M2 = 2,
    M3 = 3,
    M4 = 4,
    M5 = 5,
}

///
/// A segment identifier on a given multiplexer.  Segment identifiers
/// need not start at 0.
///
#[derive(
    Copy,
    Clone,
    Debug,
    FromPrimitive,
    Eq,
    PartialEq,
    SerializedSize,
    Serialize,
    Deserialize,
)]
#[repr(u8)]
pub enum Segment {
    S1 = 1,
    S2 = 2,
    S3 = 3,
    S4 = 4,
    S5 = 5,
    S6 = 6,
    S7 = 7,
    S8 = 8,
    S9 = 9,
    S10 = 10,
    S11 = 11,
    S12 = 12,
    S13 = 13,
    S14 = 14,
    S15 = 15,
    S16 = 16,
}

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
    fn enable_controller(&mut self, controller: Controller) -> Result<(), Self::Error>;

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
    fn disable_controller(&mut self, controller: Controller) -> Result<(), Self::Error>;
}

/// Mock I2C hardware implementation for testing
pub mod mock;
