use iroh_mpc_discovery::MultipeerTransport;
use objc2::AllocAnyThread;
use objc2::{MainThreadMarker, exception, rc::Retained};
use objc2_foundation::NSString;
use objc2_multipeer_connectivity::{MCPeerID, MCSession};
use std::io::Error;

fn main() {
    println!("Hello, world!");

    match exception::catch(|| {
        let main_thread = unsafe { MainThreadMarker::new_unchecked() };

        // Create peer ID with display name
        let display_name = NSString::from_str("MyDevice");
        let peer_id = unsafe { MCPeerID::initWithDisplayName(MCPeerID::alloc(), &display_name) };

        let mut transport = MultipeerTransport {
            peer_id, // Now expects Retained<MCPeerID>
            session: None,
            advertiser: None,
            browser: None,
        };

        transport.establish_connection();
        transport.start_advertising("mpcservice");
        transport.start_browsing("mpcservice");

        Ok::<MultipeerTransport, Error>(transport)
    }) {
        Ok(transport) => {
            println!("Successfully initialized MultipeerConnectivity");
            std::thread::sleep(std::time::Duration::from_secs(30));
        }
        Err(error) => {
            eprintln!("Failed to initialize MultipeerConnectivity: {:?}", error);
            std::process::exit(1);
        }
    }
}
