#![allow(unused_imports)]
#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(non_local_definitions)]
#![allow(clippy::too_many_arguments)]
#![allow(unused_unsafe)]

use objc2::rc::Allocated;
use objc2::{Encoding, Message, RefEncode};
use objc2::{MainThreadMarker, class, define_class, msg_send, rc::Retained, sel};
use objc2_foundation::{NSAutoreleasePool, NSData, NSError, NSObject, NSURL};
use objc2_foundation::{NSInputStream, NSObjectProtocol};
use objc2_foundation::{NSProgress, NSString};
use objc2_multipeer_connectivity::MCSessionDelegate;
use objc2_multipeer_connectivity::MCSessionSendDataMode;
use objc2_multipeer_connectivity::MCSessionState;
use objc2_multipeer_connectivity::{
    MCAdvertiserAssistant, MCBrowserViewController, MCPeerID, MCSession,
};

use objc2::AllocAnyThread;
use objc2::DefinedClass;
use objc2::MainThreadOnly;
use objc2::runtime::ProtocolObject;
use objc2::runtime::{AnyClass, AnyObject};

use std::cell::{Cell, RefCell};
use std::fmt;
use std::marker::PhantomData;
use std::ptr;
use std::sync::{Arc, RwLock};

use log::{debug, error, info, trace, warn};

// Define state for our delegate
pub struct SessionDelegateState {
    transport: *mut MultipeerTransport,
}

// Use define_class! macro to create our delegate class
define_class!(
    #[unsafe(super(NSObject))]
    #[name = "IrohSessionDelegate"]
    #[ivars = SessionDelegateState]
    pub struct SessionDelegate;

    unsafe impl NSObjectProtocol for SessionDelegate {}

    unsafe impl MCSessionDelegate for SessionDelegate {
        #[unsafe(method(session:peer:didChangeState:))]
        fn session_peer_didChangeState(
            &self,
            session: &MCSession,
            peer_id: &MCPeerID,
            state: MCSessionState,
        ) {
            debug!("Peer {:?} state changed to {:?}", peer_id, state);

            // Access transport if needed
            let transport_ptr = self.ivars().transport;
            if !transport_ptr.is_null() {
                let transport = unsafe { &mut *transport_ptr };
                debug!("Transport reference available in didChangeState");
            }
        }

        #[unsafe(method(session:didReceiveData:fromPeer:))]
        fn session_didReceiveData_fromPeer(
            &self,
            session: &MCSession,
            data: &NSData,
            peer_id: &MCPeerID,
        ) {
            debug!("Received data from peer: {:?}", peer_id);

            // Process received data
            let transport_ptr = self.ivars().transport;
            if !transport_ptr.is_null() {
                let transport = unsafe { &mut *transport_ptr };
                // Handle received data with transport
            }
        }

        #[unsafe(method(session:didReceiveStream:withName:fromPeer:))]
        fn session_didReceiveStream_withName_fromPeer(
            &self,
            session: &MCSession,
            stream: &NSInputStream,
            stream_name: &NSString,
            peer_id: &MCPeerID,
        ) {
            debug!("Received stream from peer");
        }

        #[unsafe(method(session:didStartReceivingResourceWithName:fromPeer:withProgress:))]
        fn session_didStartReceivingResourceWithName_fromPeer_withProgress(
            &self,
            session: &MCSession,
            resource_name: &NSString,
            peer_id: &MCPeerID,
            progress: &NSProgress,
        ) {
            debug!("Started receiving resource");
        }

        #[unsafe(method(session:didFinishReceivingResourceWithName:fromPeer:atURL:withError:))]
        fn session_didFinishReceivingResourceWithName_fromPeer_atURL_withError(
            &self,
            session: &MCSession,
            resource_name: &NSString,
            peer_id: &MCPeerID,
            local_url: Option<&NSURL>,
            error: Option<&NSError>,
        ) {
            debug!("Finished receiving resource");
        }
    }
);

// Manual Debug implementation for SessionDelegate
impl fmt::Debug for SessionDelegate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("SessionDelegate").finish()
    }
}

#[derive(Debug)]
pub struct MultipeerTransport {
    pub peer_id: Retained<MCPeerID>,
    pub session: Option<Retained<MCSession>>,
    pub advertiser: Option<Retained<MCAdvertiserAssistant>>,
    pub browser: Option<Retained<MCBrowserViewController>>,
    pub delegate: Option<Retained<SessionDelegate>>,
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

            // Ensure session exists
            if let Some(session) = &self.session {
                // Initialize advertiser with proper parameters
                let advertiser = MCAdvertiserAssistant::initWithServiceType_discoveryInfo_session(
                    MCAdvertiserAssistant::alloc(),
                    &service_type,
                    None, // No discovery info
                    session.as_ref(),
                );

                self.advertiser = Some(advertiser);

                // Start advertising
                let _: () = msg_send![self.advertiser.as_ref().unwrap(), start];
            } else {
                debug!("Cannot start advertising: session not established");
            }
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

            // Ensure session exists
            if let Some(session) = &self.session {
                let mainthread_marker = MainThreadMarker::new_unchecked();

                // Initialize browser with proper parameters
                let browser = MCBrowserViewController::initWithServiceType_session(
                    MCBrowserViewController::alloc(mainthread_marker),
                    &service_type,
                    session.as_ref(),
                );

                // We need to set minimum and maximum number of peers
                let _: () = msg_send![&browser,
                    setMinimumNumberOfPeers: 1_u64
                ];
                let _: () = msg_send![&browser,
                    setMaximumNumberOfPeers: 8_u64
                ];

                self.browser = Some(browser);
            } else {
                debug!("Cannot start browsing: session not established");
            }
        }
    }

    pub fn establish_connection(&mut self) {
        debug!("Entering establish_connection");
        unsafe {
            let _pool = NSAutoreleasePool::new();

            debug!("Creating MCSession");
            let session = MCSession::initWithPeer(MCSession::alloc(), self.peer_id.as_ref());

            debug!("Creating delegate");
            // Create our delegate instance and set the transport pointer
            let delegate = SessionDelegate::alloc().set_ivars(SessionDelegateState {
                transport: self as *mut _,
            });
            let delegate: Retained<SessionDelegate> = msg_send![super(delegate), init];

            debug!("Setting delegate on session");

            // Use objc_msgSend directly to set the delegate
            // This bypasses Rust's type system and lets the Objective-C runtime handle the protocol conversion
            unsafe {
                let _: () = msg_send![session, setDelegate: delegate.as_ref()];
            }

            debug!("Storing session and delegate");
            self.session = Some(session);
            self.delegate = Some(delegate);
        }
    }

    pub fn send_message(&self, message: &str) {
        if let Some(session) = &self.session {
            unsafe {
                // Convert string to NSData
                let message_str = NSString::from_str(message);
                let message_data: Retained<NSData> = msg_send![&message_str,
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
        } else {
            debug!("Cannot send message: session not established");
        }
    }
}
