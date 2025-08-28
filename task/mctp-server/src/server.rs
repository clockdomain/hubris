use crate::ipc::*;
use idol_runtime::{Leased, R, W};
use userlib::*;

pub struct Server;

impl Server {
    pub fn req(&mut self, msg: &RecvMessage, eid: u8) {
        let _ = msg;
        let _ = eid;
        todo!()
    }
    pub fn listener(&mut self, msg: &RecvMessage, typ: u8) {
        let _ = msg;
        let _ = typ;
        todo!()
    }
    pub fn get_eid(&mut self, msg: &RecvMessage) {
        let _ = msg;
        todo!()
    }
    pub fn set_eid(&mut self, msg: &RecvMessage, eid: u8) {
        let _ = msg;
        let _ = eid;
        todo!()
    }
    pub fn recv(
        &mut self,
        msg: &RecvMessage,
        handle: GenericHandle,
        buf: Leased<W, [u8]>,
    ) {
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
        let _ = msg;
        let _ = handle;
        let _ = typ;
        let _ = tag;
        let _ = ic;
        let _ = buf;
        todo!()
    }
}
