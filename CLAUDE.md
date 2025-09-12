# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Hubris is a microcontroller operating environment designed for deeply-embedded systems with reliability requirements. This is the monorepo containing both the Hubris kernel and Oxide Computer's production firmware applications that run on it.

## Build Commands

**Primary build tool:** Use `cargo xtask` instead of `cargo build` directly.

### Common Commands

- `cargo xtask dist <app.toml>` - Build a complete distribution image
- `cargo xtask build <app.toml> <task-name>` - Build individual tasks for iteration
- `cargo xtask clippy <app.toml> [task-names...]` - Run clippy on specific tasks
- `cargo xtask flash <app.toml>` - Build and flash to connected hardware
- `cargo xtask test <app.toml>` - Build, flash, and run tests (test images only)
- `cargo xtask humility <app.toml> -- <humility-args>` - Run Humility debugger
- `cargo xtask gdb <app.toml> -- <gdb-args>` - Attach GDB debugger
- `cargo xtask graph -o <output.dot> <app.toml>` - Generate task relationship graph

### Example Build Commands

- `cargo xtask dist app/demo-stm32f4-discovery/app.toml`
- `cargo xtask build app/gimletlet/app.toml ping`
- `cargo xtask test test/tests-stm32fx/app.toml`
- `cargo xtask flash app/lpc55xpresso/app.toml`

## Toolchain Requirements

- **Rust version:** Specified in `rust-toolchain.toml` (nightly toolchain required)
- **Targets:** thumbv6m-none-eabi, thumbv7em-none-eabihf, thumbv8m.main-none-eabihf
- **Required tools:**
  - [Humility](https://github.com/oxidecomputer/humility) debugger
  - libusb, libftdi1, arm-none-eabi-gdb
  - OpenOCD or appropriate debugger for target hardware

## Architecture Overview

### Directory Structure

- `app/` - Top-level application images for specific hardware platforms
- `sys/` - Kernel (`kern`), ABI definitions (`abi`), and user library (`userlib`)
- `drv/` - Hardware drivers (both library crates and server tasks)
- `task/` - Reusable system tasks (not drivers)
- `lib/` - Utility libraries and shared code
- `idl/` - Interface definitions using Idol IDL
- `chips/` - Microcontroller peripheral definitions and debugging support
- `boards/` - Board-specific configuration files (*.toml)
- `test/` - Test framework and test images
- `build/` - Build system and supporting tools

### Application Configuration

Applications are defined by `app.toml` files with hierarchical configuration:
- Base configurations define common task sets
- Hardware variants inherit and override base settings
- Memory allocation, task priorities, and IPC permissions explicitly configured
- I2C/SPI device trees with complex multiplexer hierarchies

### Task System Architecture

**Core System Pattern:**
- `jefe` (priority 0) - Supervisor task managing system lifecycle and faults
- `sys` (priority 1) - System driver for RCC, GPIO, interrupts
- `idle` (lowest priority) - Runs when no other tasks are active

**Key Architectural Principles:**
- **Capability-based IPC:** Tasks specify `task-slots` defining communication permissions
- **Resource ownership:** Tasks use `uses` field for exclusive peripheral access
- **Memory isolation:** Each task has separate memory regions with explicit limits
- **Priority-based preemptive scheduling:** Lower numbers = higher priority
- **Fault isolation:** Task failures converted to supervisor notifications

### Hardware Abstraction

**Device Categories:**
- **Demo boards:** STM32F4 Discovery, STM32H7 Nucleo, LPC55Xpresso
- **Production hardware:** Gimlet (compute sled), Sidecar (network switch), Cosmo (compute platform)
- **Security modules:** Root of Trust (RoT) carriers

**Configuration Patterns:**
- Chip-specific features at kernel and task levels
- Board-specific pin configurations and memory layouts
- Complex I2C device trees with mux hierarchies (up to 32 QSFP transceivers on Sidecar)
- SPI device configurations with chip-select management

## Testing

**Test Images:** Located in `test/` directory with their own app.toml files
- `test/tests-stm32fx/app.toml` - STM32F4 Discovery tests
- `test/tests-stm32h7/app-h743.toml` - STM32H7 Nucleo tests
- `test/tests-gimletlet/app.toml` - Gimletlet-specific tests

**Test Execution:**
- Tests run on actual hardware, not simulators
- Results transmitted via ITM and captured by Humility
- Use `cargo xtask test <test-app.toml>` for automated testing

## Development Workflow

1. **Iteration:** Use `cargo xtask build` for individual tasks during development
2. **Testing:** Run specific test suites with `cargo xtask test`
3. **Debugging:** Use `cargo xtask humility` for runtime inspection and `cargo xtask gdb` for step-through debugging
4. **Hardware:** Flash images with `cargo xtask flash` (builds and flashes in one command)

## Important Notes

- **No `async/await`:** Currently not supported; uses explicit state machines
- **Memory safety:** Relies on Rust's ownership system and hardware memory protection units
- **Real-time guarantees:** Predictable preemption with priority-based scheduling
- **Fault tolerance:** Task crashes don't affect other tasks or kernel
- **No dynamic allocation:** All memory statically allocated at build time

## LSP Integration

For rust-analyzer support, use `cargo xtask lsp <rust-file>` to generate configuration for specific tasks within the Hubris build context.