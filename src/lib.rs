use objc2::rc::Allocated;
use objc2::{Encoding, Message, RefEncode};
use objc2::{MainThreadMarker, class, msg_send, rc::Retained, sel};
use objc2_foundation::NSObjectProtocol;
use objc2_foundation::NSString;
use objc2_foundation::{NSAutoreleasePool, NSData};
use objc2_multipeer_connectivity::MCSessionDelegate;
use objc2_multipeer_connectivity::MCSessionSendDataMode;
use objc2_multipeer_connectivity::MCSessionState;
use objc2_multipeer_connectivity::{
    MCAdvertiserAssistant, MCBrowserViewController, MCPeerID, MCSession,
};

use objc2::runtime::ProtocolObject;
use objc2::runtime::{AnyClass, AnyObject};
// use objc2::{Encoding, Message, RefEncode, class};

use objc2::AllocAnyThread;
use objc2::MainThreadOnly;

use std::marker::PhantomData;
use std::sync::{Arc, RwLock};

use log::{debug, error, info, trace, warn};

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

    pub fn establish_connection(&mut self) {
        debug!("Entering establish_connection");
        unsafe {
            let _pool = NSAutoreleasePool::new();

            debug!("Creating MCSession");
            let session = {
                let alloc = MCSession::alloc();
                debug!(
                    "MCSession alloc: {:p}",
                    Allocated::<MCSession>::as_ptr(&alloc)
                );
                let session = MCSession::initWithPeer(alloc, self.peer_id.as_ref());
                debug!(
                    "MCSession init: {:p}",
                    Retained::<MCSession>::as_ptr(&session)
                );
                session
            };

            debug!("Creating delegate");
            let delegate_object = {
                let delegate = SessionDelegate {};
                let delegate_box = Box::new(delegate);
                let delegate_ptr = Box::into_raw(delegate_box);
                debug!("Delegate ptr: {:p}", delegate_ptr);

                let retained = Retained::from_raw(delegate_ptr).expect("Failed to retain delegate");
                debug!(
                    "Retained delegate: {:p}",
                    Retained::<SessionDelegate>::as_ptr(&retained)
                );

                ProtocolObject::from_retained(retained)
            };
            debug!(
                "Protocol object: {:p}",
                Retained::<ProtocolObject<_>>::as_ptr(&delegate_object)
            );

            debug!("Setting delegate");
            session.setDelegate(Some(&delegate_object));

            debug!("Storing session and delegate");
            self.session = Some(session);
            self.delegate = Some(delegate_object);
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
    _marker: PhantomData<*const ()>, // Add marker to prevent Send/Sync
}

impl SessionDelegate {
    fn new() -> Self {
        Self {
            _marker: PhantomData,
        }
    }
}

unsafe impl RefEncode for SessionDelegate {
    const ENCODING_REF: Encoding = Encoding::Object;
}

unsafe impl Message for SessionDelegate {}
unsafe impl NSObjectProtocol for SessionDelegate {}

// Update establish_connection
impl MultipeerTransport {
    pub fn establish_connection(&mut self) {
        debug!("Entering establish_connection");
        unsafe {
            let _pool = NSAutoreleasePool::new();

            debug!("Creating MCSession");
            let session = MCSession::initWithPeer(MCSession::alloc(), self.peer_id.as_ref());

            debug!("Creating delegate");
            let delegate_object = {
                let delegate = SessionDelegate::new();
                let delegate_box = Box::new(delegate);
                let delegate_ptr = Box::into_raw(delegate_box);

                // Create retained object first
                let retained = Retained::from_raw(delegate_ptr).expect("Failed to retain delegate");

                // Convert to protocol object and store immediately
                let proto_obj = ProtocolObject::from_retained(retained);
                // Store delegate first to ensure it lives long enough
                self.delegate = Some(proto_obj.clone());
                proto_obj
            };

            debug!("Setting delegate");
            // Set delegate after storing it
            session.setDelegate(Some(&delegate_object));

            debug!("Storing session");
            self.session = Some(session);
        }
    }
}
