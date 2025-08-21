use crate::ipc::*;

pub struct Server;

impl crate::ipc::InOrderMCTPImpl for Server {
    fn req(
        &mut self,
        msg: &userlib::RecvMessage,
        eid: u8,
    ) -> Result<GenericHandle, idol_runtime::RequestError<ServerError>>
    where
        ServerError: idol_runtime::IHaveConsideredServerDeathWithThisErrorType,
    {
        todo!()
    }

    fn listener(
        &mut self,
        msg: &userlib::RecvMessage,
        typ: u8,
    ) -> Result<GenericHandle, idol_runtime::RequestError<ServerError>>
    where
        ServerError: idol_runtime::IHaveConsideredServerDeathWithThisErrorType,
    {
        todo!()
    }

    fn get_eid(
        &mut self,
        msg: &userlib::RecvMessage,
    ) -> Result<u8, idol_runtime::RequestError<core::convert::Infallible>> {
        todo!()
    }

    fn set_eid(
        &mut self,
        msg: &userlib::RecvMessage,
        eid: u8,
    ) -> Result<(), idol_runtime::RequestError<ServerError>>
    where
        ServerError: idol_runtime::IHaveConsideredServerDeathWithThisErrorType,
    {
        todo!()
    }

    fn recv(
        &mut self,
        msg: &userlib::RecvMessage,
        handle: GenericHandle,
        buf: idol_runtime::Leased<idol_runtime::W, [u8]>,
    ) -> Result<RecvMetadata, idol_runtime::RequestError<ServerError>>
    where
        ServerError: idol_runtime::IHaveConsideredServerDeathWithThisErrorType,
    {
        todo!()
    }

    fn send(
        &mut self,
        msg: &userlib::RecvMessage,
        handle: GenericHandle,
        typ: u8,
        tag: Option<u8>,
        ic: bool,
        buf: idol_runtime::Leased<idol_runtime::R, [u8]>,
    ) -> Result<u8, idol_runtime::RequestError<ServerError>>
    where
        ServerError: idol_runtime::IHaveConsideredServerDeathWithThisErrorType,
    {
        todo!()
    }
}

impl idol_runtime::NotificationHandler for Server {
    fn current_notification_mask(&self) -> u32 {
        // No notifications atm
        0
    }

    fn handle_notification(&mut self, bits: u32) {
        unreachable!()
    }
}
