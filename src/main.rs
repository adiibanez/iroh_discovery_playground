use std::io::Error;

use iroh_mpc_discovery::MultipeerTransport;
use objc2::{AllocAnyThread, MainThreadMarker, exception, rc::Retained};
use objc2_foundation::NSString;
use objc2_multipeer_connectivity::MCPeerID;

fn main() {
    println!("Hello, world!");

    // Wrap all Objective-C interactions in a catch block
    match exception::catch(|| {
        // Get main thread marker - required for UIKit/AppKit operations
        let main_thread = unsafe { MainThreadMarker::new_unchecked() };

        // Create display name and peer ID safely on main thread
        let display_name = NSString::from_str("MyDevice");
        // let peer_id: Retained<MCPeerID> = unsafe { MCPeerID::new() };
        let peer_id: Retained<MCPeerID> =
            unsafe { MCPeerID::initWithDisplayName(MCPeerID::alloc(), &display_name) };

        let mut transport = MultipeerTransport {
            peer_id,
            session: None,
            advertiser: None,
            browser: None,
        };

        // Establish connection and start services
        transport.establish_connection();
        transport.start_advertising("iroh_service");
        transport.start_browsing("iroh_service");

        Ok::<MultipeerTransport, Error>(transport)
    }) {
        Ok(transport) => {
            println!("Successfully initialized MultipeerConnectivity");
            // Keep the program running to allow connections
            std::thread::sleep(std::time::Duration::from_secs(30));
        }
        Err(error) => {
            eprintln!("Failed to initialize MultipeerConnectivity: {:?}", error);
            std::process::exit(1);
        }
    }
}
