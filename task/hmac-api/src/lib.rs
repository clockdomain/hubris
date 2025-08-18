// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! API crate for the 'hmac' task.

#![no_std]

use derive_idol_err::IdolError;
use hubpack::SerializedSize;
use serde::{Deserialize, Serialize};
use userlib::{sys_send, FromPrimitive};

#[derive(
    Copy,
    Clone,
    Debug,
    FromPrimitive,
    Eq,
    PartialEq,
    IdolError,
    counters::Count,
)]
#[repr(u32)]
pub enum MacError {
    /// Invalid session ID provided
    InvalidSession = 1,
    /// No free sessions available 
    TooManySessions = 2,
    /// Input data length is invalid
    InvalidInputLength = 3,
    /// Invalid key length provided
    InvalidKeyLength = 4,
    /// Hardware failure occurred
    HardwareFailure = 5,
    /// Unsupported algorithm requested
    UnsupportedAlgorithm = 6,
    /// Task was restarted
    TaskRestarted = 7,
    /// Memory lease error
    BadLease = 8,
    /// Session algorithm mismatch
    AlgorithmMismatch = 9,
}

/// Hash algorithms supported for HMAC operations
#[derive(
    Copy, Clone, Debug, Deserialize, Eq, PartialEq, Serialize, SerializedSize,
)]
pub enum HmacAlgorithm {
    Sha256,
    Sha384,  
    Sha512,
}

/// Maximum key length supported (128 bytes)
pub const MAX_KEY_LENGTH: usize = 128;

/// Maximum data length per update operation (1024 bytes)
pub const MAX_DATA_LENGTH: usize = 1024;

/// HMAC-SHA256 output size (32 bytes)
pub const HMAC_SHA256_SIZE: usize = 32;

/// HMAC-SHA384 output size (48 bytes)
pub const HMAC_SHA384_SIZE: usize = 48;

/// HMAC-SHA512 output size (64 bytes)
pub const HMAC_SHA512_SIZE: usize = 64;

include!(concat!(env!("OUT_DIR"), "/client_stub.rs"));
