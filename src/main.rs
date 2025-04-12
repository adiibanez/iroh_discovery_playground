use iroh_mpc_discovery::MultipeerTransport;
use objc2::rc::Retained;
use objc2_foundation::NSString;
use objc2_multipeer_connectivity::MCPeerID;

fn main() {
    println!("Hello, world!");

    // Create peer ID with display name
    let _display_name = unsafe { NSString::from_str("MyDevice") };
    let peer_id: Retained<MCPeerID> = unsafe { MCPeerID::new() };

    let mut transport = MultipeerTransport {
        // peer_id: peer_id.as_ref().clone(), // Convert Retained<MCPeerID> to MCPeerID
        // peer_id: <Retained<MCPeerID> as AsRef<T>>::as_ref(&peer_id).clone(), // Correctly convert Retained<MCPeerID> to MCPeerID
        // peer_id: <Retained<MCPeerID> as AsRef<MCPeerID>>::as_ref(peer_id).clone(),
        peer_id: peer_id,
        session: None,
        advertiser: None,
        browser: None,
    };

    // Start advertising and browsing for peers
    transport.start_advertising("iroh_service");
    transport.start_browsing("iroh_service");

    // First establish connection
    transport.establish_connection();
}
