#![allow(unused_imports)]
#![allow(dead_code)]
#![allow(unused_variables)]
// #![allow(deprecated)]
// #![allow(unused_must_use)]
#![allow(non_local_definitions)]
// #[cfg(not(clippy))]
#![feature(mpmc_channel)]
#![allow(clippy::too_many_arguments)]

use iroh_discovery_playground::MultipeerTransport;
use multipeer_session::MultipeerSession;
use objc2::AllocAnyThread;
use objc2::{MainThreadMarker, exception};
use objc2_foundation::NSString;
use objc2_multipeer_connectivity::MCPeerID;
use std::io::Error;

mod multipeer_session;

use env_logger::{Builder, Env};

fn main() {
    let session = MultipeerSession::new(
        "example-service",
        |data, peer| println!("Received data from peer: {:?}", peer),
        |peer| println!("Peer joined: {:?}", peer),
        |peer| println!("Peer left: {:?}", peer),
    );

    // Send data to peers
    if let Ok(()) = session.send_to_peers(b"Hello!", &session.connected_peers(), true) {
        println!("Message sent successfully");
    }
}

fn main_() {
    println!("Hello, world!");

    Builder::from_env(Env::default().default_filter_or("debug"))
        .format_timestamp_millis()
        .init();

    match exception::catch(|| {
        let main_thread = unsafe { MainThreadMarker::new_unchecked() };

        // Create peer ID with display name
        let display_name = NSString::from_str("MyDevice");
        let peer_id = unsafe { MCPeerID::initWithDisplayName(MCPeerID::alloc(), &display_name) };

        let mut transport = MultipeerTransport::new(peer_id);
        /* {
        peer_id, // Now expects Retained<MCPeerID>
        session: None,
        advertiser: None,
        browser: None,
        };*/

        transport.establish_connection();
        // transport.start_advertising("mpcservice");
        // transport.start_browsing("mpcservice");

        // std::thread::sleep(std::time::Duration::from_secs(2));

        // let random_message: String = thread_rng()
        //     .sample_iter(&Alphanumeric)
        //     .take(10)
        //     .map(char::from)
        //     .collect();

        // transport.send_message(&random_message);
        // println!("Sent random message: {}", random_message);

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
