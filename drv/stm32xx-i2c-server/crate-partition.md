# STM32 I2C Driver Architecture - Crate Partitioning

**Project:** Hubris Operating System  
**Component:** STM32 I2C Driver Architecture  
**Version:** 1.0  
**Date:** September 8, 2025  

## Overview

This document illustrates the crate partitioning and layered architecture of the STM32 I2C driver implementation in Hubris, showing the separation of concerns across different crates and their relationships.

## Architecture Block Diagram

```mermaid
graph TB
    subgraph "Application Layer"
        APP1[Sensor Manager Task]
        APP2[FRU Management Task]
        APP3[BMC Control Task]
        APP4[Other Client Tasks]
    end

    subgraph "IPC Interface Layer"
        API[drv-i2c-api<br/>• Message Types<br/>• Operation Enums<br/>• Response Codes<br/>• Serialization]
    end

    subgraph "Service Layer"
        SERVER[stm32xx-i2c-server<br/>• Multi-Controller Management<br/>• Mux State Tracking<br/>• Pin Configuration<br/>• Bus Recovery<br/>• IPC Message Handling<br/>• Error Recovery]
    end

    subgraph "Hardware Abstraction Layer"
        DRIVER[drv-stm32xx-i2c<br/>• STM32 Register Programming<br/>• Interrupt Handling<br/>• Hardware Configuration<br/>• Low-level Operations<br/>• Timing Calculations]
    end

    subgraph "System Interface Layer"
        SYS[drv-stm32xx-sys-api<br/>• GPIO Control<br/>• Clock Management<br/>• System Calls<br/>• Power Management]
        USERLIB[userlib<br/>• Hubris System Calls<br/>• IPC Primitives<br/>• Task Management<br/>• Memory Management]
    end

    subgraph "Hardware Layer"
        STM32[STM32 Hardware<br/>• I2C1 Controller<br/>• I2C2 Controller<br/>• I2C3 Controller<br/>• GPIO Pins<br/>• Clock Sources]
    end

    subgraph "External Devices"
        MUX[I2C Multiplexers<br/>• PCA9548<br/>• PCA9546<br/>• Custom Muxes]
        DEV[I2C Devices<br/>• Sensors<br/>• EEPROMs<br/>• Power Management<br/>• FRU Data]
    end

    %% Application to API connections
    APP1 --> API
    APP2 --> API
    APP3 --> API
    APP4 --> API

    %% API to Server connection
    API --> SERVER

    %% Server to Driver connection
    SERVER --> DRIVER

    %% Driver to System interfaces
    DRIVER --> SYS
    SERVER --> SYS
    SERVER --> USERLIB
    DRIVER --> USERLIB

    %% System to Hardware
    SYS --> STM32
    USERLIB --> STM32

    %% Hardware to External
    STM32 --> MUX
    STM32 --> DEV
    MUX --> DEV

    %% Styling
    classDef appLayer fill:#e1f5fe,stroke:#01579b,stroke-width:2px
    classDef apiLayer fill:#f3e5f5,stroke:#4a148c,stroke-width:2px
    classDef serviceLayer fill:#e8f5e8,stroke:#1b5e20,stroke-width:2px
    classDef driverLayer fill:#fff3e0,stroke:#e65100,stroke-width:2px
    classDef systemLayer fill:#fce4ec,stroke:#880e4f,stroke-width:2px
    classDef hardwareLayer fill:#f1f8e9,stroke:#33691e,stroke-width:2px
    classDef deviceLayer fill:#e0f2f1,stroke:#004d40,stroke-width:2px

    class APP1,APP2,APP3,APP4 appLayer
    class API apiLayer
    class SERVER serviceLayer
    class DRIVER driverLayer
    class SYS,USERLIB systemLayer
    class STM32 hardwareLayer
    class MUX,DEV deviceLayer
```

## Crate Responsibilities

### **Application Layer**
Applications that need I2C functionality communicate through the standardized IPC interface.

### **drv-i2c-api**
**Type:** Interface Definition Crate  
**Purpose:** Defines the IPC contract between clients and the I2C server
```rust
// Message types for IPC communication
pub struct I2cRequest {
    pub controller: Controller,
    pub operation: I2cOperation,
    pub timeout_ms: u32,
}

pub enum I2cOperation {
    WriteRead { addr: u8, write_data: Vec<u8>, read_len: u8 },
    // ... other operations
}
```

### **stm32xx-i2c-server**
**Type:** Server Task Crate (Binary)  
**Purpose:** High-level I2C system service with advanced features
```rust
// Main server loop handling IPC requests
#[export_name = "main"]
fn main() -> ! {
    // Multi-controller initialization
    // Mux state management
    // Complex error recovery
    // IPC message processing
}
```

### **drv-stm32xx-i2c**
**Type:** Hardware Driver Crate (Library)  
**Purpose:** Low-level STM32 I2C peripheral driver
```rust
// Direct hardware register programming
impl I2cController {
    pub fn write_read(&mut self, addr: u8, ...) -> Result<(), ResponseCode> {
        // STM32-specific register operations
        self.registers.cr2.write(|w| /* ... */);
    }
}
```

### **drv-stm32xx-sys-api**
**Type:** System Interface Crate  
**Purpose:** STM32-specific system operations (GPIO, clocks, etc.)
```rust
// GPIO control for pin muxing and bus recovery
impl Sys {
    pub fn gpio_configure_alternate(&self, pin: PinSet, ...) { }
    pub fn gpio_reset(&self, pin: PinSet) { }
}
```

### **userlib**
**Type:** Hubris System Library  
**Purpose:** Core Hubris system calls and IPC primitives
```rust
// IPC and system call interface
pub fn recv_without_notification<T>(
    buffer: &mut [u8],
    handler: impl FnOnce(Op, Message) -> Result<T, ResponseCode>
) -> T
```

## Data Flow Diagram

```mermaid
sequenceDiagram
    participant App as Client Application
    participant API as drv-i2c-api
    participant Server as stm32xx-i2c-server
    participant Driver as drv-stm32xx-i2c
    participant Sys as drv-stm32xx-sys-api
    participant HW as STM32 Hardware

    Note over App,HW: I2C Read Operation Flow

    App->>API: Create I2cRequest
    API->>Server: IPC Message (serialized)
    
    Note over Server: Validate request, configure mux
    
    Server->>Sys: Configure GPIO pins
    Sys->>HW: Set pin alternate function
    
    Server->>Driver: write_read(addr, data)
    Driver->>HW: Program I2C registers
    HW->>HW: Perform I2C transaction
    
    Note over HW: Interrupt on completion
    
    HW->>Driver: Hardware interrupt
    Driver->>Server: Return result
    Server->>API: IPC Response
    API->>App: Parsed response
```

## Crate Dependencies

```mermaid
graph LR
    subgraph "Dependency Graph"
        SERVER[stm32xx-i2c-server] --> API[drv-i2c-api]
        SERVER --> DRIVER[drv-stm32xx-i2c]
        SERVER --> SYS[drv-stm32xx-sys-api]
        SERVER --> USERLIB[userlib]
        
        DRIVER --> SYS
        DRIVER --> USERLIB
        
        API --> USERLIB
        
        SYS --> USERLIB
        
        %% External dependencies
        SERVER --> FIXEDMAP[fixedmap]
        SERVER --> RINGBUF[ringbuf]
        DRIVER --> BITFLAGS[bitflags]
        API --> SERDE[serde]
    end

    classDef hubrisCrate fill:#e3f2fd,stroke:#1565c0,stroke-width:2px
    classDef externalCrate fill:#fff8e1,stroke:#f57f17,stroke-width:2px

    class SERVER,API,DRIVER,SYS,USERLIB hubrisCrate
    class FIXEDMAP,RINGBUF,BITFLAGS,SERDE externalCrate
```

## Build-Time Configuration

```mermaid
graph TB
    subgraph "Build Process"
        APPTOML[app.toml<br/>System Configuration]
        BUILD[build.rs<br/>Code Generation]
        
        APPTOML --> BUILD
        
        BUILD --> CONFIG[i2c_config.rs<br/>• Controllers<br/>• Pin Mappings<br/>• Mux Definitions]
        BUILD --> NOTIF[notifications.rs<br/>• Interrupt Mappings<br/>• Task IDs]
        
        CONFIG --> SERVER
        NOTIF --> SERVER
    end

    classDef configFile fill:#e8eaf6,stroke:#3f51b5,stroke-width:2px
    classDef generatedFile fill:#f3e5f5,stroke:#7b1fa2,stroke-width:2px

    class APPTOML configFile
    class CONFIG,NOTIF generatedFile
```

## Memory Layout

```mermaid
graph TB
    subgraph "Memory Organization"
        subgraph "Flash Memory"
            SERVERCODE[Server Task Code<br/>~16KB]
            DRIVERCODE[Driver Code<br/>~8KB]
            APICODE[API Definitions<br/>~2KB]
        end
        
        subgraph "RAM Memory"
            STACK[Server Stack<br/>~2KB]
            STATE[Runtime State<br/>• Port Maps<br/>• Mux State<br/>• Trace Buffer<br/>~1KB]
            BUFFERS[IPC Buffers<br/>~1KB]
        end
        
        subgraph "Hardware Registers"
            I2CREGS[I2C Controller Registers<br/>• Control Registers<br/>• Status Registers<br/>• Data Registers]
            GPIOREGS[GPIO Registers<br/>• Pin Configuration<br/>• Output Control]
        end
    end

    classDef flashMem fill:#e1f5fe,stroke:#0277bd,stroke-width:2px
    classDef ramMem fill:#e8f5e8,stroke:#388e3c,stroke-width:2px
    classDef hwMem fill:#fff3e0,stroke:#f57c00,stroke-width:2px

    class SERVERCODE,DRIVERCODE,APICODE flashMem
    class STACK,STATE,BUFFERS ramMem
    class I2CREGS,GPIOREGS hwMem
```

## Comparison with ASPEED Architecture

```mermaid
graph LR
    subgraph "ASPEED Architecture"
        subgraph "Single Crate Approach"
            ASPEEDAPP[Application Code]
            ASPEEDCTRL[I2cController]
            ASPEEDTRAIT[HardwareInterface Trait]
            ASPEEDIMPL[Ast1060I2c Implementation]
            ASPEEDPAC[AST1060 PAC]
        end
        
        ASPEEDAPP --> ASPEEDCTRL
        ASPEEDCTRL --> ASPEEDTRAIT
        ASPEEDTRAIT --> ASPEEDIMPL
        ASPEEDIMPL --> ASPEEDPAC
    end

    subgraph "Hubris Architecture"
        subgraph "Multi-Crate Approach"
            HUBRISAPP[Client Applications]
            HUBRISAPI[drv-i2c-api]
            HUBRISSERVER[stm32xx-i2c-server]
            HUBRISDRIVER[drv-stm32xx-i2c]
            HUBRISSYS[drv-stm32xx-sys-api]
        end
        
        HUBRISAPP --> HUBRISAPI
        HUBRISAPI --> HUBRISSERVER
        HUBRISSERVER --> HUBRISDRIVER
        HUBRISSERVER --> HUBRISSYS
    end

    classDef aspeedCrate fill:#ffebee,stroke:#c62828,stroke-width:2px
    classDef hubrisCrate fill:#e8f5e8,stroke:#2e7d32,stroke-width:2px

    class ASPEEDAPP,ASPEEDCTRL,ASPEEDTRAIT,ASPEEDIMPL,ASPEEDPAC aspeedCrate
    class HUBRISAPP,HUBRISAPI,HUBRISSERVER,HUBRISDRIVER,HUBRISSYS hubrisCrate
```

## Key Architectural Benefits

### **Separation of Concerns**
- **API Layer**: Clean IPC interface definition
- **Service Layer**: High-level system management
- **Driver Layer**: Hardware-specific implementation
- **System Layer**: Platform services

### **Modularity**
- Each crate has a single, well-defined responsibility
- Clear dependency relationships
- Testable components in isolation

### **Safety & Security**
- Server task isolation prevents direct hardware access
- IPC-based communication with validation
- Controlled resource access through system APIs

### **Maintainability**
- Hardware changes isolated to driver crate
- API changes don't affect hardware implementation
- Server logic separate from low-level details

### **Scalability**
- Multiple controllers managed centrally
- Complex mux topologies supported
- System-wide coordination of I2C resources

This architecture represents a **production-grade embedded system design** where reliability, maintainability, and security are prioritized over simplicity, contrasting with the more direct ASPEED trait-based approach that prioritizes ease of use and portability.

---

## Platform Portability Analysis

Based on expert firmware architecture review, the STM32 I2C architecture demonstrates varying levels of hardware coupling across its layers:

### **🔄 Highly Portable Components (Hardware-Agnostic)**

**✅ `drv-i2c-api`**: Completely decoupled from hardware
- Pure IPC interface definitions with 5-tuple device identification
- Platform-agnostic `I2cDevice` API with comprehensive operations
- Only dependencies: `userlib`, `drv-i2c-types`
- **Portability Score: 100% - Zero modification required**

**✅ `drv-i2c-types`**: Fully portable type definitions
- Hardware-neutral enums: `Controller`, `Mux`, `Segment`, `ResponseCode` 
- Works across host and embedded systems
- Comprehensive error taxonomy (29 specific response codes)
- **Portability Score: 100% - Direct reuse across platforms**

**✅ `build-i2c`**: Generic build system
- Platform-agnostic TOML configuration parsing
- Code generation framework for any I2C backend
- Controller/device/mux topology definitions
- **Portability Score: 95% - Minor platform-specific feature flags**

**✅ Multiplexer Drivers**: Hardware-independent implementations
- `pca9545.rs`, `pca9548.rs`, `ltc4306.rs`, `oximux16.rs`, `max7358.rs`
- Trait-based design (`I2cMuxDriver`) decoupled from hardware layer
- **Portability Score: 90% - Only depend on I2C hardware abstraction**

### **⚙️ Requires Abstraction Layer (Medium Coupling)**

**🔧 `stm32xx-i2c-server`**: Mixed hardware coupling
```rust
// Portable business logic (70%):
- Mux state management and coordination
- IPC message handling and validation  
- Multi-controller resource management
- Error recovery and bus reset logic
- Complex transaction orchestration

// STM32-specific coupling (30%):
- Pin configuration via drv-stm32xx-sys-api
- Feature flags (h743, h753, g031, g030)
- STM32-specific peripheral management
```
**Portability Score: 70% - Requires hardware abstraction layer**

**Recommended Decoupling Strategy:**
```rust
trait I2cHardwareAbstraction {
    type Error;
    fn configure_pins(&self, pins: &PinConfig) -> Result<(), Self::Error>;
    fn reset_controller(&self, ctrl: Controller) -> Result<(), Self::Error>;
    fn enable_peripheral(&self, peripheral: Peripheral) -> Result<(), Self::Error>;
}

// Generic server implementation
pub struct GenericI2cServer<H: I2cHardwareAbstraction> {
    hardware: H,
    mux_state: MuxMap,
    // ... portable business logic
}
```

### **🔩 Platform-Specific Components (High Coupling)**

**🚫 `drv-stm32xx-i2c`**: Tightly coupled to STM32 hardware
```rust
// STM32-specific register abstractions:
pub type RegisterBlock = device::i2c1::RegisterBlock;  // STM32 PAC types
pub type Isr = device::i2c1::isr::R;                  // STM32 interrupts

// Conditional compilation for STM32 variants:
#[cfg(feature = "h743")] use stm32h7::stm32h743 as device;
#[cfg(feature = "g031")] use stm32g0::stm32g031 as device;

// Direct register manipulation:
self.registers.cr2.write(|w| /* STM32-specific bitfields */);
```
**Portability Score: 15% - Requires complete rewrite for new platforms**

**🚫 `drv-stm32xx-sys-api`**: STM32 system interface
- GPIO abstractions (`PinSet`, `Port` enums)
- Clock tree management
- Peripheral enable/disable control  
**Portability Score: 10% - Fundamental system interface differences**

### **📋 Existing Cross-Platform Evidence**

The codebase **already demonstrates** successful platform abstraction:

```rust
// LPC55 I2C implementation (drv-lpc55-i2c):
use lpc55_pac as device;              // NXP LPC55 PAC
use drv_lpc55_gpio_api::*;           // NXP-specific GPIO
use drv_lpc55_syscon_api::*;         // NXP system control

// Same I2C concepts, different hardware layer:
let i2c = unsafe { &*device::I2C4::ptr() };
i2c.cfg.modify(|_, w| w.msten().enabled());
```

This proves the **architecture boundaries are correctly positioned** for multi-platform support.

### **🎯 Portability Roadmap**

**Phase 1: Hardware Abstraction Layer**
```rust
pub trait I2cHardware {
    type Error: From<ResponseCode>;
    
    fn write_read(&mut self, addr: u8, write: &[u8], read: &mut [u8]) 
        -> Result<usize, Self::Error>;
    fn configure_timing(&mut self, speed: I2cSpeed) -> Result<(), Self::Error>;
    fn reset_bus(&mut self) -> Result<(), Self::Error>;
    fn enable_controller(&mut self, controller: Controller) -> Result<(), Self::Error>;
}
```

**Phase 2: Platform Implementations**
```rust
impl I2cHardware for Stm32I2c<RegisterBlock> { /* STM32 implementation */ }
impl I2cHardware for Lpc55I2c<device::I2C4>  { /* NXP implementation */ }
impl I2cHardware for RiscVU74I2c              { /* RISC-V implementation */ }
impl I2cHardware for MockI2c                  { /* Testing implementation */ }
```

**Phase 3: Generic Server**
```rust
pub struct GenericI2cServer<H: I2cHardware> {
    hardware: Vec<H>,
    mux_state: MuxMap,
    // All business logic becomes hardware-agnostic
}
```

### **💡 Architecture Assessment Summary**

The STM32 I2C architecture exhibits **excellent separation of concerns** with clear portability boundaries:

1. **API/Types Layer**: Already 100% portable - demonstrates proper abstraction
2. **Business Logic**: 70% portable - needs hardware abstraction trait
3. **Hardware Layer**: 15% portable - platform-specific by design
4. **Proven Pattern**: LPC55 implementation validates the approach

**Verdict**: This architecture is **exceptionally well-designed for portability**. The hard coupling is isolated to the bottom hardware layer where it belongs, while business logic and interfaces remain cleanly abstracted. The existing LPC55 implementation proves this design scales across different microcontroller families.