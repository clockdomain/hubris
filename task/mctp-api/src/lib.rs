#![no_std]

use derive_idol_err::IdolError;
use hubpack::SerializedSize;
use serde::Serialize;

#[derive(Clone, Copy, Debug, Serialize, SerializedSize)]
#[repr(C)]
pub struct RecvMetadata {
    pub msg_typ: u8,
    pub msg_ic: bool,
    pub size: u64,
    pub resp_handle: Option<u8>,
}

#[derive(Clone, Copy, Debug, IdolError, counters::Count)]
#[repr(u32)]
pub enum ServerError {
    InternalError = 1,
}

#[derive(
    Clone,
    Copy,
    Debug,
    zerocopy_derive::Immutable,
    zerocopy_derive::IntoBytes,
    zerocopy_derive::FromBytes,
)]
#[repr(transparent)]
pub struct GenericHandle(u8);

include!(concat!(env!("OUT_DIR"), "/client_stub.rs"));
