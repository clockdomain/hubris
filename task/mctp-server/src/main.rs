// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

#![no_std]
#![no_main]

use userlib::*;

mod serial;
mod server;

#[export_name = "main"]
fn main() -> ! {
    let mut msg_buf = [0; ipc::INCOMING_SIZE];
    let mut server =
        server::Server::new(mctp::Eid(42), 0, serial::SerialSender {});

    loop {
        let msg = sys_recv_open(&mut msg_buf, 0);
        handle_mctp_msg(&msg_buf, msg, &mut server);
    }
}

mod ipc {
    use counters::*;
    pub use mctp_api::ipc::*;

    include!(concat!(env!("OUT_DIR"), "/server_stub.rs"));
}

fn handle_mctp_msg<S: mctp_stack::Sender>(
    msg_buf: &[u8],
    recv_msg: RecvMessage,
    server: &mut server::Server<S>,
) {
    use hubpack::deserialize;
    use idol_runtime::Leased;
    use zerocopy::FromBytes;
    let Some(op) = ipc::MCTPOperation::from_u32(recv_msg.operation) else {
        // TODO check which cases and unwraps have to be handled better.
        return;
    };
    match op {
        ipc::MCTPOperation::req => {
            let eid = ipc::MCTP_req_ARGS::ref_from_bytes(msg_buf)
                .unwrap_lite()
                .eid;
            server.req(&recv_msg, eid);
        }
        ipc::MCTPOperation::listener => {
            let typ = ipc::MCTP_listener_ARGS::ref_from_bytes(msg_buf)
                .unwrap_lite()
                .typ;
            server.listener(&recv_msg, typ);
        }
        ipc::MCTPOperation::get_eid => {
            server.get_eid(&recv_msg);
        }
        ipc::MCTPOperation::set_eid => {
            let eid = ipc::MCTP_set_eid_ARGS::ref_from_bytes(msg_buf)
                .unwrap_lite()
                .eid;
            server.set_eid(&recv_msg, eid);
        }
        ipc::MCTPOperation::recv => {
            let (recv_args, _): (ipc::MCTP_recv_ARGS, _) =
                deserialize(msg_buf).unwrap_lite();
            let lease = Leased::write_only_slice(recv_msg.sender, 0, None)
                .unwrap_lite();
            server.recv(&recv_msg, recv_args.handle, lease);
        }
        ipc::MCTPOperation::send => {
            let (send_args, _): (ipc::MCTP_send_ARGS, _) =
                deserialize(msg_buf).unwrap_lite();
            let ic = send_args.raw_ic != 0;
            let lease =
                Leased::read_only_slice(recv_msg.sender, 0, None).unwrap_lite();
            server.send(
                &recv_msg,
                send_args.handle,
                send_args.typ,
                send_args.tag,
                ic,
                lease,
            );
        }
    }
}
