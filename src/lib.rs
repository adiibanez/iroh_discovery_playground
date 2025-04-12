use objc2::{MainThreadMarker, class, msg_send, rc::Retained, sel};
use objc2_foundation::NSString;
use objc2_multipeer_connectivity::{
    MCAdvertiserAssistant, MCBrowserViewController, MCPeerID, MCSession,
};

use objc2::AllocAnyThread;
use objc2::MainThreadOnly;

use std::sync::{Arc, RwLock};

#[derive(Debug)]
pub struct MultipeerTransport {
    pub peer_id: Retained<MCPeerID>, // Changed from MCPeerID to Retained<MCPeerID>
    pub session: Option<Retained<MCSession>>, // Communication session with peers
    pub advertiser: Option<Retained<MCAdvertiserAssistant>>, // Advertise to nearby peers
    pub browser: Option<Retained<MCBrowserViewController>>, // Browse for available peers
}

impl MultipeerTransport {
    // Start advertising this peer to nearby devices
    pub fn start_advertising(&mut self, service_type: &str) {
        unsafe {
            // Convert service_type to NSString with proper format
            // Format must be: up to 15 characters long and contain only letters, numbers, and hyphens
            let formatted_type = format!("iroh-{}", service_type)
                .chars()
                .filter(|c| c.is_ascii_alphanumeric() || *c == '-')
                .take(15)
                .collect::<String>();

            let service_type = NSString::from_str(&formatted_type);

            // Initialize advertiser with proper parameters
            let advertiser = MCAdvertiserAssistant::initWithServiceType_discoveryInfo_session(
                MCAdvertiserAssistant::alloc(),
                &service_type,
                None, // No discovery info
                self.session.as_ref().unwrap().as_ref(),
            );

            self.advertiser = Some(advertiser);

            // Start advertising
            let _: () = msg_send![self.advertiser.as_ref().unwrap(), start];
        }
    }

    pub fn start_browsing(&mut self, service_type: &str) {
        unsafe {
            // Convert service_type to NSString
            let formatted_type = format!("iroh-{}", service_type)
                .chars()
                .filter(|c| c.is_ascii_alphanumeric() || *c == '-')
                .take(15)
                .collect::<String>();

            let service_type = NSString::from_str(&formatted_type);

            let mainthread_marker = unsafe { MainThreadMarker::new_unchecked() };

            // Initialize browser with proper parameters
            let browser = MCBrowserViewController::initWithServiceType_session(
                MCBrowserViewController::alloc(mainthread_marker),
                &service_type,
                self.session.as_ref().unwrap().as_ref(),
            );

            self.browser = Some(browser);

            // Start browsing
            let _: () = msg_send![self.browser.as_ref().unwrap(), start];
        }
    }

    // Establish a communication session
    pub fn establish_connection(&mut self) {
        unsafe {
            // Initialize MCSession with our peer_id
            let session = MCSession::initWithPeer(MCSession::alloc(), self.peer_id.as_ref());
            self.session = Some(session);
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
