// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

fn main() {
    let backend = if cfg!(feature = "mock") {
        "mock"
    } else if cfg!(feature = "stm32") {
        "stm32" 
    } else {
        panic!("Must specify exactly one backend: 'mock' or 'stm32'");
    };

    println!("cargo:rustc-cfg=i2c_backend=\"{}\"", backend);

    // Only generate I2C config for STM32 backend
    #[cfg(feature = "stm32")]
    {
        build_util::expose_target_board();
        let task_name = std::env::var("CARGO_PKG_NAME").unwrap();
        build_i2c::codegen(&task_name);
    }
}