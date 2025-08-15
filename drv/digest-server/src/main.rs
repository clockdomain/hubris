#![no_std]
#![no_main]

use drv_digest_api::{DigestError};
use idol_runtime::{Leased, LenLimit, RequestError, R, W};
use userlib::*;

use openprot_hal_blocking::digest::{
    DigestInit, DigestOp,
    Sha2_256, Sha2_384, Sha2_512, Digest
};
use openprot_platform_mock::hash::MockDigestDevice;

// Re-export the API that was generated from digest.idol.
include!(concat!(env!("OUT_DIR"), "/server_stub.rs"));

// Maximum number of concurrent digest sessions
const MAX_SESSIONS: usize = 8;

// Session timeout in ticks (adjust as needed)
const SESSION_TIMEOUT_TICKS: u64 = 10_000;

// Session buffer size - keep this reasonable
const SESSION_BUFFER_SIZE: usize = 512;

// Session states for different hash algorithms
#[derive(Copy, Clone)]
pub enum SessionAlgorithm {
    Free,
    Sha256,
    Sha384, 
    Sha512,
}

// Session data stored separately from the server struct
#[derive(Copy, Clone)]
pub struct SessionData {
    algorithm: SessionAlgorithm,
    buffer: [u8; SESSION_BUFFER_SIZE],
    length: usize,
    timeout: Option<u64>,
}

impl Default for SessionData {
    fn default() -> Self {
        Self {
            algorithm: SessionAlgorithm::Free,
            buffer: [0u8; SESSION_BUFFER_SIZE],
            length: 0,
            timeout: None,
        }
    }
}

// Global session storage - allocated statically to avoid stack overflow
static mut SESSION_STORAGE: [SessionData; MAX_SESSIONS] = [SessionData {
    algorithm: SessionAlgorithm::Free,
    buffer: [0u8; SESSION_BUFFER_SIZE],
    length: 0,
    timeout: None,
}; MAX_SESSIONS];

// Server implementation with MockDevice backend
pub struct ServerImpl {
    hardware: MockDigestDevice,
}

impl ServerImpl {
    pub fn new() -> Self {
        Self {
            hardware: MockDigestDevice::new(),
        }
    }

    fn find_free_session(&self) -> Result<usize, DigestError> {
        unsafe {
            for (index, session) in SESSION_STORAGE.iter().enumerate() {
                if matches!(session.algorithm, SessionAlgorithm::Free) {
                    return Ok(index);
                }
            }
        }
        Err(DigestError::TooManySessions)
    }

    fn validate_session(&self, session_id: u32) -> Result<usize, DigestError> {
        let index = session_id as usize;
        if index >= MAX_SESSIONS {
            return Err(DigestError::InvalidSession);
        }
        
        unsafe {
            if matches!(SESSION_STORAGE[index].algorithm, SessionAlgorithm::Free) {
                return Err(DigestError::InvalidSession);
            }
        }
        
        Ok(index)
    }

    fn cleanup_expired_sessions(&mut self) {
        let current_time = sys_get_timer().now;
        
        unsafe {
            for session in SESSION_STORAGE.iter_mut() {
                if let Some(timeout_time) = session.timeout {
                    if current_time > timeout_time {
                        session.algorithm = SessionAlgorithm::Free;
                        session.length = 0;
                        session.timeout = None;
                    }
                }
            }
        }
    }

    fn update_session_timeout(&mut self, index: usize) {
        unsafe {
            SESSION_STORAGE[index].timeout = Some(sys_get_timer().now + SESSION_TIMEOUT_TICKS);
        }
    }

    // Static dispatch methods - no runtime algorithm selection
    fn compute_sha256_hash(&mut self, data: &[u8]) -> Result<Digest<8>, DigestError> {
        let mut ctx = self.hardware.init(Sha2_256).map_err(|_| DigestError::HardwareFailure)?;
        ctx.update(data).map_err(|_| DigestError::HardwareFailure)?;
        ctx.finalize().map_err(|_| DigestError::HardwareFailure)
    }
    
    fn compute_sha384_hash(&mut self, data: &[u8]) -> Result<Digest<12>, DigestError> {
        let mut ctx = self.hardware.init(Sha2_384).map_err(|_| DigestError::HardwareFailure)?;
        ctx.update(data).map_err(|_| DigestError::HardwareFailure)?;
        ctx.finalize().map_err(|_| DigestError::HardwareFailure)
    }
    
    fn compute_sha512_hash(&mut self, data: &[u8]) -> Result<Digest<16>, DigestError> {
        let mut ctx = self.hardware.init(Sha2_512).map_err(|_| DigestError::HardwareFailure)?;
        ctx.update(data).map_err(|_| DigestError::HardwareFailure)?;
        ctx.finalize().map_err(|_| DigestError::HardwareFailure)
    }
}

impl InOrderDigestImpl for ServerImpl {
    fn init_sha256(
        &mut self,
        _msg: &RecvMessage,
    ) -> Result<u32, RequestError<DigestError>> {
        self.cleanup_expired_sessions();
        
        let index = self.find_free_session().map_err(RequestError::Runtime)?;
        
        unsafe {
            SESSION_STORAGE[index].algorithm = SessionAlgorithm::Sha256;
            SESSION_STORAGE[index].length = 0;
        }
        self.update_session_timeout(index);
        
        Ok(index as u32)
    }

    fn init_sha384(
        &mut self,
        _msg: &RecvMessage,
    ) -> Result<u32, RequestError<DigestError>> {
        self.cleanup_expired_sessions();
        
        let index = self.find_free_session().map_err(RequestError::Runtime)?;
        
        unsafe {
            SESSION_STORAGE[index].algorithm = SessionAlgorithm::Sha384;
            SESSION_STORAGE[index].length = 0;
        }
        self.update_session_timeout(index);
        
        Ok(index as u32)
    }

    fn init_sha512(
        &mut self,
        _msg: &RecvMessage,
    ) -> Result<u32, RequestError<DigestError>> {
        self.cleanup_expired_sessions();
        
        let index = self.find_free_session().map_err(RequestError::Runtime)?;
        
        unsafe {
            SESSION_STORAGE[index].algorithm = SessionAlgorithm::Sha512;
            SESSION_STORAGE[index].length = 0;
        }
        self.update_session_timeout(index);
        
        Ok(index as u32)
    }

    fn init_sha3_256(
        &mut self,
        _msg: &RecvMessage,
    ) -> Result<u32, RequestError<DigestError>> {
        // SHA-3 not supported by MockDigestDevice
        Err(RequestError::Runtime(DigestError::UnsupportedAlgorithm))
    }

    fn init_sha3_384(
        &mut self,
        _msg: &RecvMessage,
    ) -> Result<u32, RequestError<DigestError>> {
        // SHA-3 not supported by MockDigestDevice
        Err(RequestError::Runtime(DigestError::UnsupportedAlgorithm))
    }

    fn init_sha3_512(
        &mut self,
        _msg: &RecvMessage,
    ) -> Result<u32, RequestError<DigestError>> {
        // SHA-3 not supported by MockDigestDevice
        Err(RequestError::Runtime(DigestError::UnsupportedAlgorithm))
    }

    fn update(
        &mut self,
        _msg: &RecvMessage,
        session_id: u32,
        len: u32,
        data: LenLimit<Leased<R, [u8]>, 1024>,
    ) -> Result<(), RequestError<DigestError>> {
        let index = self.validate_session(session_id).map_err(RequestError::Runtime)?;
        self.update_session_timeout(index);
        
        let len = len as usize;
        if len > data.len() || len > 1024 {
            return Err(RequestError::Runtime(DigestError::InvalidInputLength));
        }
        
        // Read data into a temporary buffer
        let mut buffer = [0u8; 1024];
        data.read_range(0..len, &mut buffer[..len])
            .map_err(|_| RequestError::Runtime(DigestError::InvalidInputLength))?;
            
        // Append to session data
        unsafe {
            let session = &mut SESSION_STORAGE[index];
            if session.length + len > SESSION_BUFFER_SIZE {
                return Err(RequestError::Runtime(DigestError::MemoryAllocationFailure));
            }
            
            session.buffer[session.length..session.length + len].copy_from_slice(&buffer[..len]);
            session.length += len;
        }
        
        Ok(())
    }

    fn finalize_sha256(
        &mut self,
        _msg: &RecvMessage,
        session_id: u32,
        digest: Leased<W, [u32; 8]>,
    ) -> Result<(), RequestError<DigestError>> {
        let index = self.validate_session(session_id).map_err(RequestError::Runtime)?;
        
        let (session_data, data_len) = unsafe {
            let session = &mut SESSION_STORAGE[index];
            if !matches!(session.algorithm, SessionAlgorithm::Sha256) {
                return Err(RequestError::Runtime(DigestError::InvalidSession));
            }
            
            // Get a copy of the data and reset the session
            let mut data = [0u8; SESSION_BUFFER_SIZE];
            let len = session.length;
            data[..len].copy_from_slice(&session.buffer[..len]);
            
            session.algorithm = SessionAlgorithm::Free;
            session.length = 0;
            session.timeout = None;
            
            (data, len)
        };
        
        let hash_result = self.compute_sha256_hash(&session_data[..data_len]).map_err(RequestError::Runtime)?;
        
        digest.write(hash_result.value).map_err(|_| RequestError::Runtime(DigestError::InvalidInputLength))?;
        Ok(())
    }

    fn finalize_sha384(
        &mut self,
        _msg: &RecvMessage,
        session_id: u32,
        digest: Leased<W, [u32; 12]>,
    ) -> Result<(), RequestError<DigestError>> {
        let index = self.validate_session(session_id).map_err(RequestError::Runtime)?;
        
        let (session_data, data_len) = unsafe {
            let session = &mut SESSION_STORAGE[index];
            if !matches!(session.algorithm, SessionAlgorithm::Sha384) {
                return Err(RequestError::Runtime(DigestError::InvalidSession));
            }
            
            // Get a copy of the data and reset the session
            let mut data = [0u8; SESSION_BUFFER_SIZE];
            let len = session.length;
            data[..len].copy_from_slice(&session.buffer[..len]);
            
            session.algorithm = SessionAlgorithm::Free;
            session.length = 0;
            session.timeout = None;
            
            (data, len)
        };
        
        let hash_result = self.compute_sha384_hash(&session_data[..data_len]).map_err(RequestError::Runtime)?;
        
        digest.write(hash_result.value).map_err(|_| RequestError::Runtime(DigestError::InvalidInputLength))?;
        Ok(())
    }

    fn finalize_sha512(
        &mut self,
        _msg: &RecvMessage,
        session_id: u32,
        digest: Leased<W, [u32; 16]>,
    ) -> Result<(), RequestError<DigestError>> {
        let index = self.validate_session(session_id).map_err(RequestError::Runtime)?;
        
        let (session_data, data_len) = unsafe {
            let session = &mut SESSION_STORAGE[index];
            if !matches!(session.algorithm, SessionAlgorithm::Sha512) {
                return Err(RequestError::Runtime(DigestError::InvalidSession));
            }
            
            // Get a copy of the data and reset the session
            let mut data = [0u8; SESSION_BUFFER_SIZE];
            let len = session.length;
            data[..len].copy_from_slice(&session.buffer[..len]);
            
            session.algorithm = SessionAlgorithm::Free;
            session.length = 0;
            session.timeout = None;
            
            (data, len)
        };
        
        let hash_result = self.compute_sha512_hash(&session_data[..data_len]).map_err(RequestError::Runtime)?;
        
        digest.write(hash_result.value).map_err(|_| RequestError::Runtime(DigestError::InvalidInputLength))?;
        Ok(())
    }

    fn finalize_sha3_256(
        &mut self,
        _msg: &RecvMessage,
        _session_id: u32,
        _digest: Leased<W, [u32; 8]>,
    ) -> Result<(), RequestError<DigestError>> {
        Err(RequestError::Runtime(DigestError::UnsupportedAlgorithm))
    }

    fn finalize_sha3_384(
        &mut self,
        _msg: &RecvMessage,
        _session_id: u32,
        _digest: Leased<W, [u32; 12]>,
    ) -> Result<(), RequestError<DigestError>> {
        Err(RequestError::Runtime(DigestError::UnsupportedAlgorithm))
    }

    fn finalize_sha3_512(
        &mut self,
        _msg: &RecvMessage,
        _session_id: u32,
        _digest: Leased<W, [u32; 16]>,
    ) -> Result<(), RequestError<DigestError>> {
        Err(RequestError::Runtime(DigestError::UnsupportedAlgorithm))
    }

    fn reset(
        &mut self,
        _msg: &RecvMessage,
        session_id: u32,
    ) -> Result<(), RequestError<DigestError>> {
        let index = self.validate_session(session_id).map_err(RequestError::Runtime)?;
        
        unsafe {
            SESSION_STORAGE[index].algorithm = SessionAlgorithm::Free;
            SESSION_STORAGE[index].length = 0;
            SESSION_STORAGE[index].timeout = None;
        }
        
        Ok(())
    }

    // One-shot digest operations
    fn digest_oneshot_sha256(
        &mut self,
        _msg: &RecvMessage,
        len: u32,
        data: LenLimit<Leased<R, [u8]>, 1024>,
        digest_out: Leased<W, [u32; 8]>,
    ) -> Result<(), RequestError<DigestError>> {
        let len = len as usize;
        if len > data.len() || len > 1024 {
            return Err(RequestError::Runtime(DigestError::InvalidInputLength));
        }
        
        let mut buffer = [0u8; 1024];
        data.read_range(0..len, &mut buffer[..len])
            .map_err(|_| RequestError::Runtime(DigestError::InvalidInputLength))?;
            
        let hash_result = self.compute_sha256_hash(&buffer[..len]).map_err(RequestError::Runtime)?;
        
        digest_out.write(hash_result.value).map_err(|_| RequestError::Runtime(DigestError::InvalidInputLength))?;
        Ok(())
    }

    fn digest_oneshot_sha384(
        &mut self,
        _msg: &RecvMessage,
        len: u32,
        data: LenLimit<Leased<R, [u8]>, 1024>,
        digest_out: Leased<W, [u32; 12]>,
    ) -> Result<(), RequestError<DigestError>> {
        let len = len as usize;
        if len > data.len() || len > 1024 {
            return Err(RequestError::Runtime(DigestError::InvalidInputLength));
        }
        
        let mut buffer = [0u8; 1024];
        data.read_range(0..len, &mut buffer[..len])
            .map_err(|_| RequestError::Runtime(DigestError::InvalidInputLength))?;
            
        let hash_result = self.compute_sha384_hash(&buffer[..len]).map_err(RequestError::Runtime)?;
        
        digest_out.write(hash_result.value).map_err(|_| RequestError::Runtime(DigestError::InvalidInputLength))?;
        Ok(())
    }

    fn digest_oneshot_sha512(
        &mut self,
        _msg: &RecvMessage,
        len: u32,
        data: LenLimit<Leased<R, [u8]>, 1024>,
        digest_out: Leased<W, [u32; 16]>,
    ) -> Result<(), RequestError<DigestError>> {
        let len = len as usize;
        if len > data.len() || len > 1024 {
            return Err(RequestError::Runtime(DigestError::InvalidInputLength));
        }
        
        let mut buffer = [0u8; 1024];
        data.read_range(0..len, &mut buffer[..len])
            .map_err(|_| RequestError::Runtime(DigestError::InvalidInputLength))?;
            
        let hash_result = self.compute_sha512_hash(&buffer[..len]).map_err(RequestError::Runtime)?;
        
        digest_out.write(hash_result.value).map_err(|_| RequestError::Runtime(DigestError::InvalidInputLength))?;
        Ok(())
    }
}

impl idol_runtime::NotificationHandler for ServerImpl {
    fn current_notification_mask(&self) -> u32 {
        // We don't use notifications in this implementation
        0
    }

    fn handle_notification(&mut self, _bits: u32) {
        // No notifications to handle
    }
}

#[export_name = "main"]
fn main() -> ! {
    // Initialize the server
    let mut server = ServerImpl::new();
    let mut buffer = [0u8; idl::INCOMING_SIZE];
    
    // Run the server using the standard dispatch pattern
    loop {
        idol_runtime::dispatch(&mut buffer, &mut server);
    }
}

mod idl {
    use drv_digest_api::DigestError;
    include!(concat!(env!("OUT_DIR"), "/server_stub.rs"));
}