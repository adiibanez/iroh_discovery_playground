use objc2::{Encoding, Message};
use objc2::{MainThreadMarker, class, msg_send, rc::Retained, sel};
use objc2_foundation::NSData;
use objc2_foundation::NSObjectProtocol;
use objc2_foundation::NSString;
use objc2_multipeer_connectivity::MCSessionDelegate;
use objc2_multipeer_connectivity::MCSessionSendDataMode;
use objc2_multipeer_connectivity::MCSessionState;
use objc2_multipeer_connectivity::{
    MCAdvertiserAssistant, MCBrowserViewController, MCPeerID, MCSession,
};

use objc2::runtime::ProtocolObject;

use objc2::AllocAnyThread;
use objc2::MainThreadOnly;

use std::sync::{Arc, RwLock};

#[derive(Debug)]
pub struct MultipeerTransport {
    pub peer_id: Retained<MCPeerID>, // Changed from MCPeerID to Retained<MCPeerID>
    pub session: Option<Retained<MCSession>>, // Communication session with peers
    pub advertiser: Option<Retained<MCAdvertiserAssistant>>, // Advertise to nearby peers
    pub browser: Option<Retained<MCBrowserViewController>>, // Browse for available peers
    // delegate: ProtocolObject<dyn MCSessionDelegate>, // Changed to Retained
    pub delegate: Option<Retained<ProtocolObject<dyn MCSessionDelegate>>>,
}

impl MultipeerTransport {
    /// Create a new MultipeerTransport with just the required peer_id
    pub fn new(peer_id: Retained<MCPeerID>) -> Self {
        MultipeerTransport {
            peer_id,
            session: None,
            advertiser: None,
            browser: None,
            delegate: None,
        }
    }
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
            // Convert service_type to NSString with proper format
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

            // We need to set minimum and maximum number of peers
            let _: () = msg_send![&browser,
                setMinimumNumberOfPeers: 1_u64
            ];
            let _: () = msg_send![&browser,
                setMaximumNumberOfPeers: 8_u64
            ];

            self.browser = Some(browser);

            // Note: Removed the start call since MCBrowserViewController
            // is a UI component that presents itself
        }
    }

    // Establish a communication session
    /*pub fn _establish_connection(&mut self) {
        unsafe {
            // Create the session delegate
            let delegate = SessionDelegate {};
            // Create protocol object using from_ref
            let delegate_object = ProtocolObject::from_ref(&delegate);

            // Initialize MCSession with our peer_id
            let session = MCSession::initWithPeer(MCSession::alloc(), self.peer_id.as_ref());

            // Set the delegate and store it
            session.setDelegate(Some(delegate_object));
            self.delegate = Some(delegate_object.to_owned()); // Keep the delegate alive
            self.session = Some(session);
        }
    }*/

    // pub fn establish_connection(&mut self) {
    //     unsafe {
    //         // Create the session delegate and wrap it in a protocol object
    //         let delegate = SessionDelegate {};
    //         let delegate_object = ProtocolObject::<dyn MCSessionDelegate>::from(delegate);

    //         // Initialize MCSession with our peer_id
    //         let session = MCSession::initWithPeer(MCSession::alloc(), self.peer_id.as_ref());

    //         // Set the delegate and store it
    //         session.setDelegate(Some(&delegate_object));

    //         // Store owned versions
    //         // self.delegate = Some(Retained::new(delegate_object).expect("REASON"));
    //         self.delegate = Some(delegate_object).expect("REASON");
    //         self.session = Some(session);
    //     }
    // }

    pub fn establish_connection(&mut self) {
        unsafe {
            // Create the session delegate
            let delegate = SessionDelegate {};
            let delegate_box = Box::new(delegate);
            let delegate_ptr = Box::into_raw(delegate_box);

            // Create retained object and protocol object
            let retained = Retained::new(delegate_ptr).expect("Failed to retain delegate");
            let delegate_object = ProtocolObject::from_retained(retained);

            // Initialize MCSession with our peer_id
            let session = MCSession::initWithPeer(MCSession::alloc(), self.peer_id.as_ref());

            // Set the delegate and store it
            session.setDelegate(Some(&delegate_object));
            self.delegate = Some(delegate_object);
            self.session = Some(session);
        }
    }

    pub fn send_message(&self, message: &str) {
        if let Some(session) = &self.session {
            unsafe {
                // Convert string to NSData
                let message_str = NSString::from_str(message);
                let message_data: Retained<NSData> = msg_send![&message_str,  // Added & here
                    dataUsingEncoding: 4_u64  // NSUTF8StringEncoding = 4 as u64
                ];

                let peers = session.connectedPeers();

                // Send data using the proper method signature
                let _ = session.sendData_toPeers_withMode_error(
                    message_data.as_ref(),
                    &peers,
                    MCSessionSendDataMode::Reliable,
                );
            }
        }
    }
}

#[derive(Debug)]
pub struct SessionDelegate {
    // We'll add message handling callback later
}

use objc2::RefEncode;
unsafe impl RefEncode for SessionDelegate {
    // Use proper Encoding type instead of &str
    const ENCODING_REF: Encoding = Encoding::Object;
}

unsafe impl Message for SessionDelegate {}

// unsafe impl Message for SessionDelegate {
//     fn retain(&self) -> Retained<Self>
//     where
//         Self: Sized,
//     {
//         let ptr: *const Self = self;
//         let ptr: *mut Self = ptr as _;
//         // SAFETY:
//         // - The pointer is valid since it came from `&self`.
//         // - The lifetime of the pointer itself is extended, but any lifetime
//         //   that the object may carry is still kept within the type itself.
//         let obj = unsafe { Retained::retain(ptr) };
//         // SAFETY: The pointer came from `&self`, which is always non-null,
//         // and objc_retain always returns the same value.
//         unsafe { obj.unwrap_unchecked() }
//     }
// }

unsafe impl NSObjectProtocol for SessionDelegate {}

unsafe impl MCSessionDelegate for SessionDelegate {
    unsafe fn session_peer_didChangeState(
        &self,
        session: &MCSession,
        peer_id: &MCPeerID,
        state: MCSessionState,
    ) {
        println!("Peer state changed: {:?}", state);
    }

    unsafe fn session_didReceiveData_fromPeer(
        &self,
        _session: &MCSession,
        data: &NSData,
        peer_id: &MCPeerID,
    ) {
        // For now, just print received data
        println!("Received data from peer: {:?}", peer_id);
    }
}
