use crate::ipc::*;
use idol_runtime::{Leased, R, W};
use mctp_stack::Router;
use userlib::*;
use zerocopy::IntoBytes;

pub struct Server<S: mctp_stack::Sender> {
    stack: Router<S>,
}

impl<S: mctp_stack::Sender> Server<S> {
    pub fn new(own_eid: mctp::Eid, now_millis: u64, outbound: S) -> Self {
        let stack = Router::new(own_eid, now_millis, outbound);
        Self { stack }
    }
    pub fn req(&mut self, msg: &RecvMessage, eid: u8) {
        match self.stack.req(mctp::Eid(eid)) {
            Ok(handle) => {
                let handle = GenericHandle(handle.0 as u8); // TODO better mapping
                sys_reply(msg.sender, 0, handle.as_bytes())
            }
            Err(e) => {
                sys_reply(msg.sender, ServerError::from(e).into(), &[]);
            }
        }
    }
    pub fn listener(&mut self, msg: &RecvMessage, typ: u8) {
        match self.stack.listener(mctp::MsgType(typ)) {
            Ok(handle) => {
                let handle = GenericHandle(handle.0 as u8); // TODO better mapping
                sys_reply(msg.sender, 0, handle.as_bytes())
            }
            Err(e) => {
                sys_reply(msg.sender, ServerError::from(e).into(), &[]);
            }
        }
    }
    pub fn get_eid(&mut self, msg: &RecvMessage) {
        sys_reply(msg.sender, 0, self.stack.get_eid().0.as_bytes());
    }
    pub fn set_eid(&mut self, msg: &RecvMessage, eid: u8) {
        match self.stack.set_eid(mctp::Eid(eid)) {
            Ok(handle) => sys_reply(msg.sender, 0, &[]),
            Err(e) => {
                sys_reply(msg.sender, ServerError::from(e).into(), &[]);
            }
        }
    }
    pub fn recv(
        &mut self,
        msg: &RecvMessage,
        handle: GenericHandle,
        buf: Leased<W, [u8]>,
    ) {
        // TODO:
        // Check for a message for the given handle, if there is one, copy it into buf and reply.
        // Otherwise postpone the reply until a message arrives or a timeout occurs.
        // For this the RecvMessage must be stored.
        // On inbound message arrival, all postponed recv calls must be checked for a match.
        // Likewise timeouts have to be handled once we have a timeout mechanism.
        let _ = msg;
        let _ = handle;
        let _ = buf;
        todo!()
    }
    pub fn send(
        &mut self,
        msg: &RecvMessage,
        handle: GenericHandle,
        typ: u8,
        tag: Option<u8>,
        ic: bool,
        buf: Leased<R, [u8]>,
    ) {
        // The Router currently supports blocking send only, so this should be quite easy.
        // TODO figure out if handling incoming packets while sending is neccessary.
        let _ = msg;
        let _ = handle;
        let _ = typ;
        let _ = tag;
        let _ = ic;
        let _ = buf;
        todo!()
    }
}
