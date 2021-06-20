/// The Interface used to collect metrics from the Tunneler-Software.
/// This allows for the usage of a variety of different Metrics-Systems
/// as long as they can provide an implementation of this Interface.
pub trait Metrics {
    /// This is called every time a Message is received
    fn received_msg(&self) {}
    /// This is called every time a message was received with the size of the
    /// Data contained in the Message (not the size of the entire Message).
    fn recv_bytes(&self, _recv: u64) {}

    /// This is called every time a Message is send
    fn send_msg(&self) {}
    /// This is called every time a message is send with the size of the Data
    /// contained in the Message (not the size of the entire Message).
    fn send_bytes(&self, _send: u64) {}
}
