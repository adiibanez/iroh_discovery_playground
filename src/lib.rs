// use objc2::runtime::message_receiver::MessageReceiver;
use objc2::{MainThreadMarker, class, msg_send, rc::Retained, sel};
use objc2_foundation::NSString;
use objc2_multipeer_connectivity::{
    MCAdvertiserAssistant, MCBrowserViewController, MCPeerID, MCSession,
};

use std::sync::{Arc, RwLock};

#[derive(Debug)]
pub struct MultipeerTransport {
    // pub peer_id: MCPeerID, // Represents the peer’s unique identifier
    pub peer_id: Retained<MCPeerID>,
    pub session: Option<Retained<MCSession>>, // Communication session with peers
    pub advertiser: Option<Retained<MCAdvertiserAssistant>>, // Advertise to nearby peers
    pub browser: Option<Retained<MCBrowserViewController>>, // Browse for available peers
}

impl MultipeerTransport {
    // Start advertising this peer to nearby devices
    pub fn start_advertising(&mut self, service_type: &str) {
        unsafe {
            // The `new()` method requires unsafe because we are interacting with raw Objective-C API
            let advertiser = MCAdvertiserAssistant::new(); // Create advertiser
            self.advertiser = Some(advertiser);
            // Configure advertiser with service type
            let _: () = msg_send![self.advertiser.as_ref().unwrap(), start];
        }
    }

    // Start browsing for other peers
    pub fn start_browsing(&mut self, service_type: &str) {
        unsafe {
            // Create the browser on the main thread
            // The `MainThreadMarker` is used to indicate this must be executed on the main thread
            let main_thread_marker = MainThreadMarker::new().unwrap();
            let browser = MCBrowserViewController::new(main_thread_marker); // Correct use
            self.browser = Some(browser);
            // Start browsing for peers
            let _: () = msg_send![self.browser.as_ref().unwrap(), start];
        }
    }

    // Establish a communication session
    pub fn establish_connection(&mut self) {
        unsafe {
            // The `new()` method for `MCSession` also requires unsafe since it interacts with Objective-C API
            let session = MCSession::new(); // Create session without peer_id
            self.session = Some(session);
            // Additional session configuration (if any) can go here
        }
    }

    pub fn send_message(&self, message: &str) {
        if let Some(session) = &self.session {
            unsafe {
                // Convert &str to NSString before sending
                let ns_message: Retained<objc2_foundation::NSString> = msg_send![class!(NSString), stringWithUTF8String: message.as_ptr() as *const i8];

                // Get the underlying session object with proper type annotations
                let mc_session: &MCSession = session.as_ref();

                // Create an array with proper type that implements RefEncode
                let peers: [&MCPeerID; 0] = []; // Using fixed-size array instead of slice

                // Send the message using the session with explicit type annotations
                let _: () = msg_send![
                    mc_session,
                    sendData: <Retained<objc2_foundation::NSString> as AsRef<objc2_foundation::NSString>>::as_ref(&ns_message),
                    toPeers: &peers,
                    withMode: 0,
                    error: std::ptr::null_mut() as *mut *mut objc2_foundation::NSError
                ];
            }
        }
    }
}
