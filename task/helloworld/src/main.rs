// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

#![no_std]
#![no_main]

use userlib::*;
use drv_digest_api::{Digest, DigestError};

task_slot!(UART, uart_driver);
task_slot!(DIGEST, digest_server);

#[export_name = "main"]
fn main() -> ! {
    uart_send(b"Hello, world from AST1060!\r\n");
    uart_send(b"Testing digest server...\r\n");
    
    // Test the digest server
    test_digest_server();
    
    loop {
        let mut buf = [0u8; 128];
        hl::sleep_for(1000); // Sleep for 1 second
        
        if uart_read(&mut buf) {
            uart_send(b"Received: ");
            uart_send(&buf);
            uart_send(b"\r\n");
            
            // Hash the received data
            hash_received_data(&buf);
        }
    }
}

fn test_digest_server() {
    let digest = Digest::from(DIGEST.get_task_id());
    
    // Test one-shot SHA-256
    uart_send(b"Testing one-shot SHA-256...\r\n");
    let test_data = b"Hello, digest!";
    let mut result = [0u32; 8];
    
    match digest.digest_oneshot_sha256(test_data.len() as u32, test_data, &mut result) {
        Ok(_) => {
            uart_send(b"SHA-256 result: ");
            print_hash(&result[..8]);
            uart_send(b"\r\n");
        }
        Err(e) => {
            uart_send(b"SHA-256 error: ");
            print_error(e);
            uart_send(b"\r\n");
        }
    }
    
    // Test session-based SHA-256
    uart_send(b"Testing session-based SHA-256...\r\n");
    match test_session_based_digest() {
        Ok(hash) => {
            uart_send(b"Session SHA-256 result: ");
            print_hash(&hash[..8]);
            uart_send(b"\r\n");
        }
        Err(e) => {
            uart_send(b"Session error: ");
            print_error(e);
            uart_send(b"\r\n");
        }
    }
    
    // Test SHA-384
    uart_send(b"Testing SHA-384...\r\n");
    let mut result384 = [0u32; 12];
    match digest.digest_oneshot_sha384(test_data.len() as u32, test_data, &mut result384) {
        Ok(_) => {
            uart_send(b"SHA-384 result: ");
            print_hash(&result384[..8]); // Print first 8 words only for brevity
            uart_send(b"...\r\n");
        }
        Err(e) => {
            uart_send(b"SHA-384 error: ");
            print_error(e);
            uart_send(b"\r\n");
        }
    }
    
    // Test SHA-512
    uart_send(b"Testing SHA-512...\r\n");
    let mut result512 = [0u32; 16];
    match digest.digest_oneshot_sha512(test_data.len() as u32, test_data, &mut result512) {
        Ok(_) => {
            uart_send(b"SHA-512 result: ");
            print_hash(&result512[..8]); // Print first 8 words only for brevity
            uart_send(b"...\r\n");
        }
        Err(e) => {
            uart_send(b"SHA-512 error: ");
            print_error(e);
            uart_send(b"\r\n");
        }
    }
    
    uart_send(b"Digest server testing complete!\r\n");
}

fn test_session_based_digest() -> Result<[u32; 8], DigestError> {
    let digest = Digest::from(DIGEST.get_task_id());
    
    // Initialize session
    let session_id = digest.init_sha256()?;
    
    // Update with multiple chunks
    let chunk1 = b"Hello, ";
    let chunk2 = b"session-based ";
    let chunk3 = b"digest!";
    
    digest.update(session_id, chunk1.len() as u32, chunk1)?;
    digest.update(session_id, chunk2.len() as u32, chunk2)?;
    digest.update(session_id, chunk3.len() as u32, chunk3)?;
    
    // Finalize
    let mut result = [0u32; 8];
    digest.finalize_sha256(session_id, &mut result)?;
    
    Ok(result)
}

fn hash_received_data(data: &[u8]) {
    let digest = Digest::from(DIGEST.get_task_id());
    
    // Find the actual length of the data (stop at first null byte)
    let data_len = data.iter().position(|&b| b == 0).unwrap_or(data.len());
    if data_len == 0 {
        return;
    }
    
    let mut result = [0u32; 8];
    match digest.digest_oneshot_sha256(data_len as u32, &data[..data_len], &mut result) {
        Ok(_) => {
            uart_send(b"Hash of received data: ");
            print_hash(&result[..8]);
            uart_send(b"\r\n");
        }
        Err(_) => {
            uart_send(b"Failed to hash received data\r\n");
        }
    }
}

fn print_hash(hash: &[u32]) {
    for &word in hash {
        print_hex_word(word);
    }
}

fn print_hex_word(word: u32) {
    let bytes = word.to_be_bytes();
    for byte in bytes {
        print_hex_byte(byte);
    }
}

fn print_hex_byte(byte: u8) {
    let hex_chars = b"0123456789ABCDEF";
    uart_send(&[hex_chars[(byte >> 4) as usize]]);
    uart_send(&[hex_chars[(byte & 0xF) as usize]]);
}

fn print_error(error: DigestError) {
    match error {
        DigestError::InvalidSession => uart_send(b"Invalid session"),
        DigestError::TooManySessions => uart_send(b"Too many sessions"),
        DigestError::InvalidInputLength => uart_send(b"Invalid input length"),
        DigestError::UnsupportedAlgorithm => uart_send(b"Unsupported algorithm"),
        DigestError::InitializationError => uart_send(b"Initialization error"),
        DigestError::UpdateError => uart_send(b"Update error"),
        DigestError::FinalizationError => uart_send(b"Finalization error"),
        DigestError::HardwareFailure => uart_send(b"Hardware failure"),
        _ => uart_send(b"Unknown digest error"),
    }
}

fn uart_send(text: &[u8]) {
    let peer = UART.get_task_id();

    const OP_WRITE: u16 = 1;
    let (code, _) =
        sys_send(peer, OP_WRITE, &[], &mut [], &[Lease::from(text)]);
    assert_eq!(0, code);
}

fn uart_read(text: &mut [u8]) -> bool {
    let peer = UART.get_task_id();
    const OP_READ: u16 = 2;

    let mut response = [0u8; 4];
    let (code, _) = sys_send(peer, OP_READ, &[], &mut response, &mut [Lease::from(text)]);

    code == 0
}
