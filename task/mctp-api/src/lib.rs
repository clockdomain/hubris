#![no_std]

use mctp::{
    Eid, Error, Listener, MsgIC, MsgType, ReqChannel, RespChannel, Tag,
    TagValue,
};
use userlib::TaskId;

#[derive(Clone, Debug)]
pub struct Router {
    ipc: ipc::MCTP,
}
impl From<TaskId> for Router {
    fn from(value: TaskId) -> Self {
        Router { ipc: value.into() }
    }
}

impl Router {
    pub fn req(&self, eid: Eid) -> mctp::Result<RouterReqChannel<'_>> {
        let handle = self.ipc.req(eid.0)?;
        Ok(RouterReqChannel {
            router: self,
            handle,
            eid,
            sent_tag: None,
        })
    }
    pub fn listener(&self, typ: MsgType) -> mctp::Result<RouterListener<'_>> {
        let handle = self.ipc.listener(typ.0)?;
        Ok(RouterListener {
            router: self,
            handle,
        })
    }
    pub fn get_eid(&self) -> Eid {
        Eid(self.ipc.get_eid())
    }
    pub fn set_eid(&self, eid: Eid) -> mctp::Result<()> {
        Ok(self.ipc.set_eid(eid.0)?)
    }
}

#[derive(Debug)]
pub struct RouterReqChannel<'r> {
    router: &'r Router,
    handle: ipc::GenericHandle,
    eid: Eid,
    sent_tag: Option<Tag>,
}

impl ReqChannel for RouterReqChannel<'_> {
    fn send_vectored(
        &mut self,
        typ: mctp::MsgType,
        integrity_check: mctp::MsgIC,
        bufs: &[&[u8]],
    ) -> mctp::Result<()> {
        if self.sent_tag.is_some() {
            return Err(Error::BadArgument);
        }
        // TODO Sending vectored bufs over IPC isn't possible out of the box.
        //      Assembling them into one slice means copying the buffers.
        //      This it not ideal but might be a sufficient for now.
        let _ = typ;
        let _ = integrity_check;
        let _ = bufs;
        todo!("Vectored messages are not supported jet!")
    }

    fn recv<'f>(
        &mut self,
        buf: &'f mut [u8],
    ) -> mctp::Result<(mctp::MsgType, mctp::MsgIC, &'f mut [u8])> {
        let Some(Tag::Owned(tv)) = self.sent_tag else {
            return Err(Error::BadArgument);
        };
        let ipc::RecvMetadata {
            msg_typ,
            msg_ic,
            msg_tag,
            remote_eid,
            size,
            resp_handle: _,
        } = self.router.ipc.recv(self.handle, buf)?;
        debug_assert_eq!(tv.0, msg_tag);
        debug_assert_eq!(self.eid.0, remote_eid);
        let ic = mctp::MsgIC(msg_ic);
        let typ = mctp::MsgType(msg_typ);
        Ok((typ, ic, &mut buf[..size as usize]))
    }

    fn remote_eid(&self) -> Eid {
        self.eid
    }
    fn send(&mut self, typ: mctp::MsgType, buf: &[u8]) -> mctp::Result<()> {
        if self.sent_tag.is_some() {
            return Err(Error::BadArgument);
        }
        let tv = self.router.ipc.send(self.handle, typ.0, None, false, buf)?;
        let tag = Tag::Owned(mctp::TagValue(tv));
        self.sent_tag = Some(tag);
        Ok(())
    }
}

#[derive(Debug)]
pub struct RouterListener<'r> {
    router: &'r Router,
    handle: ipc::GenericHandle,
}
impl Listener for RouterListener<'_> {
    type RespChannel<'a> = RouterRespChannel<'a>
    where
        Self: 'a;

    fn recv<'f>(
        &mut self,
        buf: &'f mut [u8],
    ) -> mctp::Result<(
        mctp::MsgType,
        mctp::MsgIC,
        &'f mut [u8],
        Self::RespChannel<'_>,
    )> {
        let ipc::RecvMetadata {
            msg_typ,
            msg_ic,
            msg_tag,
            remote_eid,
            size,
            resp_handle,
        } = self.router.ipc.recv(self.handle, buf)?;

        let Some(resp_handle) = resp_handle else {
            return Err(Error::InternalError);
        };

        let resp_channel = RouterRespChannel {
            router: self.router,
            handle: resp_handle,
            eid: Eid(remote_eid),
            typ: MsgType(msg_typ),
            tv: TagValue(msg_tag),
        };

        Ok((
            MsgType(msg_typ),
            MsgIC(msg_ic),
            &mut buf[..size as usize],
            resp_channel,
        ))
    }
}

#[derive(Debug)]
pub struct RouterRespChannel<'r> {
    router: &'r Router,
    handle: ipc::GenericHandle,
    eid: Eid,
    typ: MsgType,
    tv: TagValue,
}
impl<'r> RespChannel for RouterRespChannel<'r> {
    type ReqChannel = RouterReqChannel<'r>;

    fn send_vectored(
        &mut self,
        integrity_check: MsgIC,
        bufs: &[&[u8]],
    ) -> mctp::Result<()> {
        // TODO Sending vectored bufs over IPC isn't possible out of the box.
        //      Assembling them into one slice means copying the buffers.
        //      This it not ideal but might be a sufficient for now.
        let _ = integrity_check;
        let _ = bufs;
        todo!("Vectored messages are not supported jet!")
    }

    fn remote_eid(&self) -> Eid {
        self.eid
    }

    fn req_channel(&self) -> mctp::Result<Self::ReqChannel> {
        self.router.req(self.eid)
    }

    fn send(&mut self, buf: &[u8]) -> mctp::Result<()> {
        Ok(self
            .router
            .ipc
            .send(self.handle, self.typ.0, Some(self.tv.0), false, buf)
            .map(|_| ())?)
    }
}

mod ipc {
    //! IPC API and associated types

    use derive_idol_err::IdolError;
    use hubpack::SerializedSize;
    use serde::{Deserialize, Serialize};
    use userlib::*;

    #[derive(Clone, Copy, Debug, Serialize, SerializedSize, Deserialize)]
    #[repr(C)]
    pub struct RecvMetadata {
        pub msg_typ: u8,
        pub msg_ic: bool,
        pub msg_tag: u8,
        pub remote_eid: u8,
        pub size: u64,
        pub resp_handle: Option<GenericHandle>,
    }

    #[derive(Clone, Copy, Debug, FromPrimitive, IdolError, counters::Count)]
    #[repr(u32)]
    #[non_exhaustive]
    pub enum ServerError {
        InternalError = 1,
    }

    impl From<ServerError> for mctp::Error {
        fn from(value: ServerError) -> Self {
            use mctp::Error::*;
            match value {
                ServerError::InternalError => InternalError,
            }
        }
    }

    #[derive(
        Clone,
        Copy,
        Debug,
        zerocopy_derive::Immutable,
        zerocopy_derive::IntoBytes,
        zerocopy_derive::FromBytes,
        Serialize,
        Deserialize,
        SerializedSize,
    )]
    #[repr(transparent)]
    pub struct GenericHandle(u8);

    include!(concat!(env!("OUT_DIR"), "/client_stub.rs"));
}
