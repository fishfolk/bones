//! Networking support for the bones framework.

use bytes::Bytes;

/// A handle to a [`NetworkTransport`] address.
///
/// This is not an actual address, but the backend will map this to a real address.
#[derive(PartialEq, Eq, Hash, Debug)]
pub struct Address(pub usize);

/// Trait implemented by network transport backends.
pub trait NetworkTransport {
    /// Connect to a network endpoint.
    ///
    /// The format of the address is dependent on the backend implementation.
    fn connect(&self, address: &str) -> anyhow::Result<Address>;

    /// Close an exiting connection.
    fn close(&self, target: NetworkTarget);

    /// Get a list of active connections.
    fn connected(&self) -> Vec<Address>;

    /// Accept connections for other clients.
    fn accept_connect(&self) -> anyhow::Result<Option<Address>>;

    /// Send a network packet unreliably.
    fn send_unreliable(&self, address: NetworkTarget, bytes: Bytes) -> anyhow::Result<()>;
    /// Send a network packet reliably.
    fn send_reliable(&self, address: NetworkTarget, bytes: Bytes) -> anyhow::Result<()>;
    /// Receive a network packet unreliably.
    fn recv_unreliable(&self) -> anyhow::Result<Option<IncomingPacket>>;
    /// Receieve a network packet reliably.
    fn recv_reliable(&self) -> anyhow::Result<Option<IncomingPacket>>;
}

/// The recipient of a network address.
pub enum NetworkTarget {
    /// Send the message to all connnected endpoints ( broadcast ).
    All,
    /// Send the message to a specific address.
    Address(Address),
}

/// A packet that has come in over the network.
pub struct IncomingPacket {
    /// The address of the endpoint that sent the message.
    pub from: Address,
    /// The message data.
    pub data: Bytes,
}
