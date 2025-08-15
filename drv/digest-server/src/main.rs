// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! # Digest Server
//!
//! Hardware-accelerated cryptographic digest service for the Hubris operating system.
//! Implements the digest.idol interface to provide SHA-2 family hash computations
//! with session-based and one-shot APIs.
//!
//! ## Supported Algorithms
//! - SHA-256: 256-bit hash (8 × 32-bit words output)
//! - SHA-384: 384-bit hash (12 × 32-bit words output)  
//! - SHA-512: 512-bit hash (16 × 32-bit words output)
//!
//! ## Hardware Backends
//! - `mock`: Software mock implementation for testing
//! - `stm32h7`: STM32H7 hardware acceleration (future)
//!
//! ## Architecture
//! ```text
//! Client Task → IPC (Idol) → Digest Server → Hardware Backend
//! ```

#![no_std]
#![no_main]

use core::convert::Infallible;
use heapless::FnvIndexMap;
use idol_runtime::{ClientError, Leased, LenLimit, NotificationHandler, RequestError, R, W};
use userlib::*;
use drv_digest_api::DigestError;
use ringbuf::*;
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout};

// Import the generated server stub
include!(concat!(env!("OUT_DIR"), "/server_stub.rs"));

/// Maximum number of concurrent digest sessions
const MAX_SESSIONS: usize = 8;

/// Maximum data size per update operation (bytes)
const MAX_UPDATE_SIZE: usize = 1024;

/// Digest algorithm enumeration
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum DigestAlgorithm {
    Sha256,
    Sha384,
    Sha512,
    Sha3_256,
    Sha3_384,
    Sha3_512,
}

/// Digest computation context for session state
#[derive(Debug)]
pub struct DigestContext {
    /// Algorithm being computed
    algorithm: DigestAlgorithm,
    /// Internal hash state (enough space for SHA-512)
    state: [u32; 16],
    /// Number of bytes processed so far
    bytes_processed: u64,
    /// Initialization status
    initialized: bool,
}

impl DigestContext {
    /// Create a new digest context for the specified algorithm
    pub fn new(algorithm: DigestAlgorithm) -> Self {
        Self {
            algorithm,
            state: [0u32; 16],
            bytes_processed: 0,
            initialized: true,
        }
    }

    /// Reset the context to its initial state
    pub fn reset(&mut self) {
        self.state.fill(0);
        self.bytes_processed = 0;
        self.initialized = true;
    }

    /// Update the context with new data
    pub fn update(&mut self, data: &[u8]) -> Result<(), DigestError> {
        if !self.initialized {
            return Err(DigestError::NotInitialized);
        }

        // Mock implementation: just update the byte counter
        // In a real implementation, this would feed data to the hardware
        self.bytes_processed += data.len() as u64;
        
        // Simple hash simulation: XOR all bytes into state[0]
        for &byte in data {
            self.state[0] ^= byte as u32;
        }
        
        Ok(())
    }

    /// Finalize the digest and produce the hash result
    pub fn finalize(&mut self, output: &mut [u32]) -> Result<(), DigestError> {
        if !self.initialized {
            return Err(DigestError::NotInitialized);
        }

        let expected_words = match self.algorithm {
            DigestAlgorithm::Sha256 | DigestAlgorithm::Sha3_256 => 8,
            DigestAlgorithm::Sha384 | DigestAlgorithm::Sha3_384 => 12,
            DigestAlgorithm::Sha512 | DigestAlgorithm::Sha3_512 => 16,
        };

        if output.len() < expected_words {
            return Err(DigestError::InvalidOutputSize);
        }

        // Mock hash result based on algorithm and processed bytes
        let base_value: u32 = match self.algorithm {
            DigestAlgorithm::Sha256 => 0x6a09e667,
            DigestAlgorithm::Sha384 => 0xcbbb9d5d,
            DigestAlgorithm::Sha512 => 0x6a09e667,
            _ => return Err(DigestError::UnsupportedAlgorithm),
        };

        // Generate mock hash result
        for i in 0..expected_words {
            output[i] = base_value.wrapping_add(self.state[0]).wrapping_add(i as u32);
        }

        // Include the byte count in the final word for uniqueness
        output[expected_words - 1] = output[expected_words - 1]
            .wrapping_add(self.bytes_processed as u32);

        Ok(())
    }
}

/// Digest session state
#[derive(Debug)]
struct DigestSession {
    /// The digest computation context
    context: DigestContext,
}

/// Main digest server implementation
struct ServerImpl {
    /// Active digest sessions mapped by session ID
    sessions: FnvIndexMap<u32, DigestSession, MAX_SESSIONS>,
    /// Next session ID to allocate
    next_session_id: u32,
}

impl ServerImpl {
    /// Create a new digest server
    fn new() -> Self {
        Self {
            sessions: FnvIndexMap::new(),
            next_session_id: 1,
        }
    }

    /// Allocate a new digest session
    fn allocate_session(&mut self, algorithm: DigestAlgorithm) -> Result<u32, DigestError> {
        if self.sessions.len() >= MAX_SESSIONS {
            return Err(DigestError::TooManySessions);
        }

        let session_id = self.next_session_id;
        self.next_session_id = self.next_session_id.wrapping_add(1);

        let session = DigestSession {
            context: DigestContext::new(algorithm),
        };

        self.sessions
            .insert(session_id, session)
            .map_err(|_| DigestError::TooManySessions)?;

        ringbuf_entry!(Trace::SessionAllocated(session_id));
        Ok(session_id)
    }

    /// Get a mutable reference to a session
    fn get_session_mut(&mut self, session_id: u32) -> Result<&mut DigestSession, DigestError> {
        self.sessions
            .get_mut(&session_id)
            .ok_or(DigestError::InvalidSession)
    }

    /// Remove a session
    fn remove_session(&mut self, session_id: u32) -> Result<(), DigestError> {
        self.sessions
            .remove(&session_id)
            .ok_or(DigestError::InvalidSession)?;
        
        ringbuf_entry!(Trace::SessionFinalized(session_id));
        Ok(())
    }

    /// Perform a one-shot digest operation
    fn digest_oneshot(&mut self, algorithm: DigestAlgorithm, data: &[u8], output: &mut [u32]) -> Result<(), DigestError> {
        let mut context = DigestContext::new(algorithm);
        context.update(data)?;
        context.finalize(output)?;
        
        ringbuf_entry!(Trace::OneShot(data.len() as u32));
        Ok(())
    }
}

impl NotificationHandler for ServerImpl {
    fn current_notification_mask(&self) -> u32 {
        0 // We don't use notifications
    }

    fn handle_notification(&mut self, _bits: u32) {
        // No notifications to handle
    }
}

/// Implementation of the Idol-generated digest interface
impl InOrderDigestImpl for ServerImpl {
    /// Initialize SHA-256 digest session
    fn init_sha256(
        &mut self,
        _: &RecvMessage,
    ) -> Result<u32, RequestError<DigestError>> {
        let session_id = self.allocate_session(DigestAlgorithm::Sha256)?;
        Ok(session_id)
    }

    /// Initialize SHA-384 digest session
    fn init_sha384(
        &mut self,
        _: &RecvMessage,
    ) -> Result<u32, RequestError<DigestError>> {
        let session_id = self.allocate_session(DigestAlgorithm::Sha384)?;
        Ok(session_id)
    }

    /// Initialize SHA-512 digest session
    fn init_sha512(
        &mut self,
        _: &RecvMessage,
    ) -> Result<u32, RequestError<DigestError>> {
        let session_id = self.allocate_session(DigestAlgorithm::Sha512)?;
        Ok(session_id)
    }

    /// Initialize SHA3-256 digest session (not implemented)
    fn init_sha3_256(
        &mut self,
        _: &RecvMessage,
    ) -> Result<u32, RequestError<DigestError>> {
        Err(DigestError::UnsupportedAlgorithm.into())
    }

    /// Initialize SHA3-384 digest session (not implemented)
    fn init_sha3_384(
        &mut self,
        _: &RecvMessage,
    ) -> Result<u32, RequestError<DigestError>> {
        Err(DigestError::UnsupportedAlgorithm.into())
    }

    /// Initialize SHA3-512 digest session (not implemented)
    fn init_sha3_512(
        &mut self,
        _: &RecvMessage,
    ) -> Result<u32, RequestError<DigestError>> {
        Err(DigestError::UnsupportedAlgorithm.into())
    }

    /// Update digest session with new data
    fn update(
        &mut self,
        _: &RecvMessage,
        session_id: u32,
        len: u32,
        data: LenLimit<Leased<R, [u8]>, MAX_UPDATE_SIZE>,
    ) -> Result<(), RequestError<DigestError>> {
        // Read data from leased memory
        let mut buffer = [0u8; MAX_UPDATE_SIZE];
        let actual_len = core::cmp::min(len as usize, data.len());
        data.read_range(0..actual_len, &mut buffer[..actual_len])
            .map_err(|_| RequestError::Fail(ClientError::WentAway))?;

        // Get the session and update it
        let session = self.get_session_mut(session_id)?;
        session.context.update(&buffer[..actual_len])?;

        ringbuf_entry!(Trace::DigestUpdate(session_id, actual_len as u32));
        Ok(())
    }

    /// Finalize SHA-256 digest session
    fn finalize_sha256(
        &mut self,
        _: &RecvMessage,
        session_id: u32,
        digest_out: Leased<W, [u32; 8]>,
    ) -> Result<(), RequestError<DigestError>> {
        // Get the session and finalize it
        let session = self.get_session_mut(session_id)?;
        let mut result = [0u32; 8];
        session.context.finalize(&mut result)?;

        // Write result to leased memory using write method
        digest_out.write(result)
            .map_err(|_| RequestError::Fail(ClientError::WentAway))?;

        // Remove the session
        self.remove_session(session_id)?;
        Ok(())
    }

    /// Finalize SHA-384 digest session
    fn finalize_sha384(
        &mut self,
        _: &RecvMessage,
        session_id: u32,
        digest_out: Leased<W, [u32; 12]>,
    ) -> Result<(), RequestError<DigestError>> {
        let session = self.get_session_mut(session_id)?;
        let mut result = [0u32; 12];
        session.context.finalize(&mut result)?;

        digest_out.write(result)
            .map_err(|_| RequestError::Fail(ClientError::WentAway))?;

        self.remove_session(session_id)?;
        Ok(())
    }

    /// Finalize SHA-512 digest session
    fn finalize_sha512(
        &mut self,
        _: &RecvMessage,
        session_id: u32,
        digest_out: Leased<W, [u32; 16]>,
    ) -> Result<(), RequestError<DigestError>> {
        let session = self.get_session_mut(session_id)?;
        let mut result = [0u32; 16];
        session.context.finalize(&mut result)?;

        digest_out.write(result)
            .map_err(|_| RequestError::Fail(ClientError::WentAway))?;

        self.remove_session(session_id)?;
        Ok(())
    }

    /// Finalize SHA3-256 digest session (not implemented)
    fn finalize_sha3_256(
        &mut self,
        _: &RecvMessage,
        _session_id: u32,
        _digest_out: Leased<W, [u32; 8]>,
    ) -> Result<(), RequestError<DigestError>> {
        Err(DigestError::UnsupportedAlgorithm.into())
    }

    /// Finalize SHA3-384 digest session (not implemented)
    fn finalize_sha3_384(
        &mut self,
        _: &RecvMessage,
        _session_id: u32,
        _digest_out: Leased<W, [u32; 12]>,
    ) -> Result<(), RequestError<DigestError>> {
        Err(DigestError::UnsupportedAlgorithm.into())
    }

    /// Finalize SHA3-512 digest session (not implemented)
    fn finalize_sha3_512(
        &mut self,
        _: &RecvMessage,
        _session_id: u32,
        _digest_out: Leased<W, [u32; 16]>,
    ) -> Result<(), RequestError<DigestError>> {
        Err(DigestError::UnsupportedAlgorithm.into())
    }

    /// Reset a digest session
    fn reset(
        &mut self,
        _: &RecvMessage,
        session_id: u32,
    ) -> Result<(), RequestError<DigestError>> {
        let session = self.get_session_mut(session_id)?;
        session.context.reset();
        Ok(())
    }

    /// One-shot SHA-256 digest operation
    fn digest_oneshot_sha256(
        &mut self,
        _: &RecvMessage,
        len: u32,
        data: LenLimit<Leased<R, [u8]>, MAX_UPDATE_SIZE>,
        digest_out: Leased<W, [u32; 8]>,
    ) -> Result<(), RequestError<DigestError>> {
        // Read data from leased memory
        let mut buffer = [0u8; MAX_UPDATE_SIZE];
        let actual_len = core::cmp::min(len as usize, data.len());
        data.read_range(0..actual_len, &mut buffer[..actual_len])
            .map_err(|_| RequestError::Fail(ClientError::WentAway))?;

        // Perform one-shot digest
        let mut result = [0u32; 8];
        self.digest_oneshot(DigestAlgorithm::Sha256, &buffer[..actual_len], &mut result)?;

        // Write result to leased memory
        digest_out.write(result)
            .map_err(|_| RequestError::Fail(ClientError::WentAway))?;

        Ok(())
    }

    /// One-shot SHA-384 digest operation
    fn digest_oneshot_sha384(
        &mut self,
        _: &RecvMessage,
        len: u32,
        data: LenLimit<Leased<R, [u8]>, MAX_UPDATE_SIZE>,
        digest_out: Leased<W, [u32; 12]>,
    ) -> Result<(), RequestError<DigestError>> {
        let mut buffer = [0u8; MAX_UPDATE_SIZE];
        let actual_len = core::cmp::min(len as usize, data.len());
        data.read_range(0..actual_len, &mut buffer[..actual_len])
            .map_err(|_| RequestError::Fail(ClientError::WentAway))?;

        let mut result = [0u32; 12];
        self.digest_oneshot(DigestAlgorithm::Sha384, &buffer[..actual_len], &mut result)?;

        digest_out.write(result)
            .map_err(|_| RequestError::Fail(ClientError::WentAway))?;

        Ok(())
    }

    /// One-shot SHA-512 digest operation
    fn digest_oneshot_sha512(
        &mut self,
        _: &RecvMessage,
        len: u32,
        data: LenLimit<Leased<R, [u8]>, MAX_UPDATE_SIZE>,
        digest_out: Leased<W, [u32; 16]>,
    ) -> Result<(), RequestError<DigestError>> {
        let mut buffer = [0u8; MAX_UPDATE_SIZE];
        let actual_len = core::cmp::min(len as usize, data.len());
        data.read_range(0..actual_len, &mut buffer[..actual_len])
            .map_err(|_| RequestError::Fail(ClientError::WentAway))?;

        let mut result = [0u32; 16];
        self.digest_oneshot(DigestAlgorithm::Sha512, &buffer[..actual_len], &mut result)?;

        digest_out.write(result)
            .map_err(|_| RequestError::Fail(ClientError::WentAway))?;

        Ok(())
    }
}

/// Trace events for debugging and monitoring
#[derive(Copy, Clone, PartialEq)]
enum Trace {
    None,
    SessionAllocated(u32),
    SessionFinalized(u32),
    DigestUpdate(u32, u32), // session_id, data_len
    OneShot(u32),           // data_len
}

ringbuf!(Trace, 16, Trace::None);

/// Main entry point
#[export_name = "main"]
fn main() -> ! {
    let mut server = ServerImpl::new();
    let mut buffer = [0u8; 1024];

    ringbuf_entry!(Trace::None);

    loop {
        idol_runtime::dispatch(&mut buffer, &mut server);
    }
}
