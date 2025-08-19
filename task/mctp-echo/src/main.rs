// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! A demo task that echoes MCTP messages.
//!
//! The task configures the MCTP server for EID 8
//! and starts listening for MCTP Message Type `1` (PLDM) packets.
//! Received messages are echoed as is through the response channel.

#![no_std]
#![no_main]

use mctp::{Eid, Listener, MsgType, RespChannel};
use mctp_api::{MctpListener, Stack};
use userlib::*;

task_slot!(MCTP, mctp);

#[export_name = "main"]
fn main() -> ! {
    let stack = mctp_api::Stack::from(mctp);

    stack.set_eid(Eid(8)).unwrap_lite();
    let listener = stack.listener(MsgType(1)).unwrap_lite();
    let mut recv_buf = [0; 255];

    loop {
        let (_, _, msg, resp) = listener.recv(&mut recv_buf).unwrap_lite();

        match resp.send(buf) {
            Ok(_) => {}
            Err(e) => {
                // Error sending response to peer
            }
        }
    }
}
