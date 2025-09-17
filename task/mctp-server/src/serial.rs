use mctp::Result;

pub struct SerialSender;

impl mctp_stack::Sender for SerialSender {
    fn send(
        &mut self,
        fragmenter: mctp_stack::Fragmenter,
        payload: &[u8],
    ) -> Result<mctp::Tag> {
        todo!()
    }
    fn get_mtu(&self) -> usize {
        todo!()
    }
}
