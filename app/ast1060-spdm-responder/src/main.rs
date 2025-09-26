// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

#![no_std]
#![no_main]

// We have to do this if we don't otherwise use it to ensure its vector table
// gets linked in.

use cortex_m_rt::entry;
use ast1060_pac::Peripherals;
use drv_spdm_responder_api::*;

#[cfg(feature = "jtag-halt")]
use core::ptr::{self, addr_of};

#[entry]
fn main() -> ! {

    // This code just forces the ast1060 pac to be linked in.
    let peripherals = unsafe {
        Peripherals::steal()
    };
    peripherals.scu.scu000().modify(|_, w| {
        w
    });
    peripherals.scu.scu41c().modify(|_, w| {
        // Set the JTAG pinmux to 0x1f << 25
        w.enbl_armtmsfn_pin().bit(true)
            .enbl_armtckfn_pin().bit(true)
            .enbl_armtrstfn_pin().bit(true)
            .enbl_armtdifn_pin().bit(true)
            .enbl_armtdofn_pin().bit(true)
    });

    #[cfg(feature = "jtag-halt")]
    jtag_halt();

    // Test SPDM API compilation
    let _ = SpdmError::InvalidRequest;
    let _ = SpdmVersion::V1_0;
    let _ = SpdmRequestCode::GetVersion;

    // Default boot speed, until we bother raising it:
    const CYCLES_PER_MS: u32 = 200_000;

    unsafe { kern::startup::start_kernel(CYCLES_PER_MS) }
}

#[cfg(feature = "jtag-halt")]
fn jtag_halt() {
    static mut HALT : u32 = 1;

    // This is a hack to halt the CPU in JTAG mode.
    // It writes a value to a volatile memory location
    // Break by jtag and set val to zero to continue.
    loop {
        let val;
        unsafe {
            val = ptr::read_volatile(addr_of!(HALT));
        }

        if val == 0 {
            break;
        }
    }
}
