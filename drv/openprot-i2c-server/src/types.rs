use drv_i2c_api;
use drv_i2c_types;

/// Generic trait for hardware abstraction
pub trait I2cControllerHardware {
    type Peripheral;    // Platform-specific (sys_api::Peripheral for STM32, u8 for AST1060)
    type Registers;     // Platform-specific (RegisterBlock for STM32, AST1060RegisterBlock for AST1060)
    
    fn peripheral(&self) -> &Self::Peripheral;
    fn notification(&self) -> u32;
}

/// Generic controller that works with any hardware
pub struct I2cController<'a, H: I2cControllerHardware> {
    pub controller: drv_i2c_api::Controller,
    pub hardware: &'a H,
}

// ===== CONTROLLER MANAGER OPERATION TRAITS =====
// These traits define the operations that the Controller Manager routes to controller instances

/// Operations that each I2C controller instance must support
pub trait I2cControllerOperations {
    type Error: Into<drv_i2c_types::ResponseCode>;
    
    /// Perform write followed by read operation
    fn write_read(
        &mut self,
        addr: u8,
        write_data: &[u8],
        read_buffer: &mut [u8],
    ) -> Result<usize, Self::Error>;
    
    /// Master write-read block operation (Op::WriteReadBlock)  
    fn write_read_block(
        &mut self,
        addr: u8,
        write_data: &[u8],
        read_buffer: &mut [u8],
    ) -> Result<usize, Self::Error>;
    
    /// Configure slave mode (Op::ConfigureSlaveAddress)
    fn configure_slave_mode(
        &mut self,
        config: &drv_i2c_types::SlaveConfig,
    ) -> Result<(), Self::Error>;
    
    /// Enable slave receive mode (Op::EnableSlaveReceive)
    fn enable_slave_receive(&mut self) -> Result<(), Self::Error>;
    
    /// Disable slave receive mode (Op::DisableSlaveReceive)
    fn disable_slave_receive(&mut self) -> Result<(), Self::Error>;
}

/// Controller Manager - Routes operations to controller instances
pub struct I2cControllerManager<C: I2cControllerOperations> {
    /// Array of controller instances indexed by controller ID
    controllers: [Option<C>; 8], // Support up to 8 I2C controllers (I2C0-I2C7)
}

impl<C: I2cControllerOperations> I2cControllerManager<C> {
    pub fn new() -> Self {
        Self {
            controllers: [None, None, None, None, None, None, None, None],
        }
    }
    
    /// Convert Controller enum to array index
    fn controller_to_index(controller: drv_i2c_api::Controller) -> usize {
        match controller {
            drv_i2c_api::Controller::I2C0 => 0,
            drv_i2c_api::Controller::I2C1 => 1,
            drv_i2c_api::Controller::I2C2 => 2,
            drv_i2c_api::Controller::I2C3 => 3,
            drv_i2c_api::Controller::I2C4 => 4,
            drv_i2c_api::Controller::I2C5 => 5,
            drv_i2c_api::Controller::I2C6 => 6,
            drv_i2c_api::Controller::I2C7 => 7,
        }
    }
    
    /// Add a controller instance to the manager
    pub fn add_controller(
        &mut self, 
        controller_id: drv_i2c_api::Controller, 
        controller: C
    ) -> Result<(), ()> {
        let index = Self::controller_to_index(controller_id);
        if self.controllers[index].is_some() {
            return Err(()); // Controller already exists
        }
        self.controllers[index] = Some(controller);
        Ok(())
    }
    
    /// Route Op::WriteRead to the appropriate controller
    pub fn handle_write_read(
        &mut self,
        controller_id: drv_i2c_api::Controller,
        addr: u8,
        write_data: &[u8],
        read_buffer: &mut [u8],
    ) -> Result<usize, drv_i2c_types::ResponseCode> {
        let index = Self::controller_to_index(controller_id);
        let controller = self.controllers[index].as_mut()
            .ok_or(drv_i2c_types::ResponseCode::BadController)?;
            
        controller.write_read(addr, write_data, read_buffer)
            .map_err(|e| e.into())
    }
    
    /// Route Op::ConfigureSlaveAddress to the appropriate controller
    pub fn handle_configure_slave_mode(
        &mut self,
        controller_id: drv_i2c_api::Controller,
        config: &drv_i2c_types::SlaveConfig,
    ) -> Result<(), drv_i2c_types::ResponseCode> {
        let index = Self::controller_to_index(controller_id);
        let controller = self.controllers[index].as_mut()
            .ok_or(drv_i2c_types::ResponseCode::BadController)?;
            
        controller.configure_slave_mode(config)
            .map_err(|e| e.into())
    }
    
    /// Route Op::EnableSlaveReceive to the appropriate controller
    pub fn handle_enable_slave_receive(
        &mut self,
        controller_id: drv_i2c_api::Controller,
    ) -> Result<(), drv_i2c_types::ResponseCode> {
        let index = Self::controller_to_index(controller_id);
        let controller = self.controllers[index].as_mut()
            .ok_or(drv_i2c_types::ResponseCode::BadController)?;
            
        controller.enable_slave_receive()
            .map_err(|e| e.into())
    }
}

// Implement I2cHardware trait for I2cControllerManager so it can be used directly
impl<C: I2cControllerOperations> drv_i2c_types::traits::I2cHardware for I2cControllerManager<C> {
    type Error = drv_i2c_types::ResponseCode;
    
    fn write_read(&mut self, controller: drv_i2c_api::Controller, addr: u8, write_data: &[u8], read_buffer: &mut [u8]) -> Result<usize, Self::Error> {
        self.handle_write_read(controller, addr, write_data, read_buffer)
    }
    
    fn write_read_block(&mut self, controller: drv_i2c_api::Controller, addr: u8, write_data: &[u8], read_buffer: &mut [u8]) -> Result<usize, Self::Error> {
        // For now, delegate to regular write_read - can be specialized later
        self.handle_write_read(controller, addr, write_data, read_buffer)
    }
    
    fn configure_timing(&mut self, _controller: drv_i2c_api::Controller, _speed: drv_i2c_types::traits::I2cSpeed) -> Result<(), Self::Error> {
        // Not implemented for mock - would need to be added to I2cControllerOperations trait
        Ok(())
    }
    
    fn reset_bus(&mut self, _controller: drv_i2c_api::Controller) -> Result<(), Self::Error> {
        // Not implemented for mock - would need to be added to I2cControllerOperations trait
        Ok(())
    }
    
    fn enable_controller(&mut self, _controller: drv_i2c_api::Controller) -> Result<(), Self::Error> {
        // Not implemented for mock - would need to be added to I2cControllerOperations trait
        Ok(())
    }
    
    fn disable_controller(&mut self, _controller: drv_i2c_api::Controller) -> Result<(), Self::Error> {
        // Not implemented for mock - would need to be added to I2cControllerOperations trait
        Ok(())
    }
    
    fn configure_slave_mode(&mut self, controller: drv_i2c_api::Controller, config: &drv_i2c_types::SlaveConfig) -> Result<(), Self::Error> {
        self.handle_configure_slave_mode(controller, config)
    }
    
    fn enable_slave_receive(&mut self, controller: drv_i2c_api::Controller) -> Result<(), Self::Error> {
        self.handle_enable_slave_receive(controller)
    }
    
    fn disable_slave_receive(&mut self, _controller: drv_i2c_api::Controller) -> Result<(), Self::Error> {
        // Not implemented for mock - would need to be added to I2cControllerOperations trait
        Ok(())
    }
    
    fn poll_slave_messages(&mut self, _controller: drv_i2c_api::Controller, _messages: &mut [drv_i2c_types::SlaveMessage]) -> Result<usize, Self::Error> {
        // Not implemented for mock - would need to be added to I2cControllerOperations trait
        Ok(0)
    }
    
    fn get_slave_status(&self, _controller: drv_i2c_api::Controller) -> Result<drv_i2c_types::traits::SlaveStatus, Self::Error> {
        // Not implemented for mock - return default status
        Ok(drv_i2c_types::traits::SlaveStatus {
            enabled: false,
            messages_received: 0,
            messages_dropped: 0,
            address_matches: 0,
            bus_errors: 0,
            buffer_full: false,
        })
    }
}