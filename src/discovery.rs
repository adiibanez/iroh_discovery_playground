use std::sync::Arc;
use iroh::endpoint::{Endpoint, Discovery};
use std::sync::RwLock;

impl Endpoint {
    pub fn discovery_apple_mpc(mut self, service_type: &str) -> Self {
        // Initialize the Multipeer Transport
        let mut mpc_transport = MultipeerTransport {
            peer_id: MCPeerID::new("IrohMpcDiscovery"), // Use an appropriate name for the peer
            session: None,
            advertiser: None,
            browser: None,
        };

        // Start advertising the peer
        mpc_transport.start_advertising(service_type);

        // Start browsing for other peers
        mpc_transport.start_browsing(service_type);

        // Store the MPC transport in the endpoint (you might want to store it as part of the state)
        self.mpc_transport = Some(mpc_transport);

        self
    }
}