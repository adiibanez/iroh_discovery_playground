#![allow(unused_unsafe)]
use objc2::rc::Allocated;
use objc2::{Encoding, Message, RefEncode, exception};
use objc2::{MainThreadMarker, class, msg_send, rc::Retained, sel};
use objc2_foundation::{NSAutoreleasePool, NSData, NSError, NSURL};
use objc2_foundation::{NSInputStream, NSObjectProtocol};
use objc2_foundation::{NSProgress, NSString};
use objc2_multipeer_connectivity::MCSessionSendDataMode;
use objc2_multipeer_connectivity::MCSessionState;
use objc2_multipeer_connectivity::{
    MCAdvertiserAssistant, MCBrowserViewController, MCPeerID, MCSession,
};
use objc2_multipeer_connectivity::{
    MCNearbyServiceAdvertiser, MCNearbyServiceBrowser, MCSessionDelegate,
};

use objc2_foundation::NSArray;

use objc2::runtime::ProtocolObject;
use objc2::runtime::{AnyClass, AnyObject};
// use objc2::{Encoding, Message, RefEncode, class};

use objc2::AllocAnyThread;
use objc2::MainThreadOnly;

use std::fmt;
use std::marker::PhantomData;
use std::sync::{Arc, RwLock};

use log::{debug, error, info, trace, warn};

// #[derive(Debug)]
pub struct MultipeerSession {
    service_type: Retained<NSString>,
    peer_id: Retained<MCPeerID>,
    session: Option<Retained<MCSession>>,
    service_advertiser: Option<Retained<MCNearbyServiceAdvertiser>>,
    service_browser: Option<Retained<MCNearbyServiceBrowser>>,
    delegate: Option<Retained<ProtocolObject<dyn MCSessionDelegate>>>,
    #[doc(hidden)]
    on_peer_joined: Option<Box<dyn Fn(&MCPeerID)>>,
    #[doc(hidden)]
    on_peer_left: Option<Box<dyn Fn(&MCPeerID)>>,
    #[doc(hidden)]
    on_data_received: Option<Box<dyn Fn(&NSData, &MCPeerID)>>,
}

// Manual Debug implementation to skip the callback fields
impl fmt::Debug for MultipeerSession {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MultipeerSession")
            .field("service_type", &self.service_type)
            .field("peer_id", &self.peer_id)
            .field("session", &self.session)
            .field("service_advertiser", &self.service_advertiser)
            .field("service_browser", &self.service_browser)
            .field("delegate", &self.delegate)
            // Skip the callback fields
            .finish()
    }
}

impl MultipeerSession {
    pub fn new(
        service_name: &str,
        on_data: impl Fn(&NSData, &MCPeerID) + 'static + std::panic::UnwindSafe,
        on_joined: impl Fn(&MCPeerID) + 'static + std::panic::UnwindSafe,
        on_left: impl Fn(&MCPeerID) + 'static + std::panic::UnwindSafe,
    ) -> Self {
        exception::catch(|| unsafe {
            let _pool = NSAutoreleasePool::new();

            // Format service name
            let formatted_name = format!("iroh-{}", service_name)
                .chars()
                .filter(|c| c.is_ascii_alphanumeric() || *c == '-')
                .take(15)
                .collect::<String>();

            let service_type = NSString::from_str(&formatted_name);

            // Create peer ID with proper memory management
            let device_name = NSString::from_str("rust-peer");
            let peer_id = MCPeerID::initWithDisplayName(MCPeerID::alloc(), &device_name);

            let mut session = Self {
                service_type,
                peer_id,
                session: None,
                service_advertiser: None,
                service_browser: None,
                delegate: None,
                on_peer_joined: Some(Box::new(on_joined)),
                on_peer_left: Some(Box::new(on_left)),
                on_data_received: Some(Box::new(on_data)),
            };

            session.initialize();
            session
        })
        .expect("Failed to initialize MultipeerSession")
    }

    fn initialize(&mut self) {
        // First create all objects outside the catch boundary
        let (session, advertiser, browser, delegate) = unsafe {
            let _pool = NSAutoreleasePool::new();

            // Create MCSession
            let mc_session = MCSession::initWithPeer(MCSession::alloc(), self.peer_id.as_ref());

            // Create delegate
            let delegate_impl = SessionDelegate::new(
                self.on_data_received.take(),
                self.on_peer_joined.take(),
                self.on_peer_left.take(),
            );

            // Box and retain the delegate
            let boxed = Box::new(delegate_impl);
            let raw_ptr = Box::into_raw(boxed);
            let retained = Retained::from_raw(raw_ptr).expect("Failed to retain delegate");
            let delegate_obj = ProtocolObject::from_retained(retained);

            // Set delegate on session
            mc_session.setDelegate(Some(&delegate_obj));

            // Create advertiser
            let adv = MCNearbyServiceAdvertiser::initWithPeer_discoveryInfo_serviceType(
                MCNearbyServiceAdvertiser::alloc(),
                self.peer_id.as_ref(),
                None,
                self.service_type.as_ref(),
            );
            adv.startAdvertisingPeer();

            // Create browser
            let br = MCNearbyServiceBrowser::initWithPeer_serviceType(
                MCNearbyServiceBrowser::alloc(),
                self.peer_id.as_ref(),
                self.service_type.as_ref(),
            );
            br.startBrowsingForPeers();

            (mc_session, adv, br, delegate_obj)
        };

        // Then update self outside the catch boundary
        self.session = Some(session);
        self.service_advertiser = Some(advertiser);
        self.service_browser = Some(browser);
        self.delegate = Some(delegate);
    }

    pub fn send_to_peers(
        &self,
        data: &[u8],
        peers: &[Retained<MCPeerID>], // Changed parameter type
        reliably: bool,
    ) -> Result<(), String> {
        if let Some(session) = &self.session {
            unsafe {
                let ns_data = NSData::from_vec(data.to_vec());

                let _pool = unsafe { NSAutoreleasePool::new() };

                // Convert peers to references
                let peer_refs: Vec<&MCPeerID> = peers.iter().map(|p| p.as_ref()).collect();
                let peer_array = NSArray::from_slice(&peer_refs);

                let mode = if reliably {
                    MCSessionSendDataMode::Reliable
                } else {
                    MCSessionSendDataMode::Unreliable
                };

                session
                    .sendData_toPeers_withMode_error(ns_data.as_ref(), &peer_array, mode)
                    .map_err(|e| e.to_string())
            }
        } else {
            Err("Session not initialized".to_string())
        }
    }

    pub fn connected_peers(&self) -> Vec<Retained<MCPeerID>> {
        // Changed return type
        if let Some(session) = &self.session {
            unsafe {
                let peer_array = session.connectedPeers();
                let count = peer_array.count();
                let mut peers = Vec::with_capacity(count as usize);

                for i in 0..count {
                    let peer = peer_array.objectAtIndex(i);
                    peers.push(peer); // Just store the Retained<MCPeerID> directly
                }
                peers
            }
        } else {
            Vec::new()
        }
    }
}

// Add this after the MultipeerSession struct but before its impl

pub struct SessionDelegate {
    on_data_received: Option<Box<dyn Fn(&NSData, &MCPeerID)>>,
    on_peer_joined: Option<Box<dyn Fn(&MCPeerID)>>,
    on_peer_left: Option<Box<dyn Fn(&MCPeerID)>>,
}

impl SessionDelegate {
    fn new(
        on_data: Option<Box<dyn Fn(&NSData, &MCPeerID)>>,
        on_joined: Option<Box<dyn Fn(&MCPeerID)>>,
        on_left: Option<Box<dyn Fn(&MCPeerID)>>,
    ) -> Self {
        Self {
            on_data_received: on_data,
            on_peer_joined: on_joined,
            on_peer_left: on_left,
        }
    }
}

unsafe impl RefEncode for SessionDelegate {
    const ENCODING_REF: Encoding = Encoding::Object;
}

unsafe impl Message for SessionDelegate {}
unsafe impl NSObjectProtocol for SessionDelegate {}

unsafe impl MCSessionDelegate for SessionDelegate {
    unsafe fn session_peer_didChangeState(
        &self,
        _session: &MCSession,
        peer_id: &MCPeerID,
        state: MCSessionState,
    ) {
        let _pool = unsafe { NSAutoreleasePool::new() };
        match state {
            MCSessionState::Connected => {
                if let Some(cb) = &self.on_peer_joined {
                    cb(peer_id);
                }
            }
            MCSessionState::NotConnected => {
                if let Some(cb) = &self.on_peer_left {
                    cb(peer_id);
                }
            }
            _ => {}
        }
    }

    unsafe fn session_didReceiveData_fromPeer(
        &self,
        _session: &MCSession,
        data: &NSData,
        peer_id: &MCPeerID,
    ) {
        let _pool = unsafe { NSAutoreleasePool::new() };
        if let Some(cb) = &self.on_data_received {
            cb(data, peer_id);
        }
    }

    unsafe fn session_didReceiveStream_withName_fromPeer(
        &self,
        _session: &MCSession,
        _stream: &NSInputStream,
        _stream_name: &NSString,
        _peer_id: &MCPeerID,
    ) {
        let _pool = unsafe { NSAutoreleasePool::new() };
    }

    unsafe fn session_didStartReceivingResourceWithName_fromPeer_withProgress(
        &self,
        _session: &MCSession,
        _resource_name: &NSString,
        _peer_id: &MCPeerID,
        _progress: &NSProgress,
    ) {
        let _pool = unsafe { NSAutoreleasePool::new() };
    }

    unsafe fn session_didFinishReceivingResourceWithName_fromPeer_atURL_withError(
        &self,
        _session: &MCSession,
        _resource_name: &NSString,
        _peer_id: &MCPeerID,
        _location_url: Option<&NSURL>,
        _error: Option<&NSError>,
    ) {
        let _pool = unsafe { NSAutoreleasePool::new() };
    }
}

struct PoolDebug {
    _pool: Retained<NSAutoreleasePool>, // Changed type to Retained<NSAutoreleasePool>
    name: &'static str,
}

impl PoolDebug {
    fn new(name: &'static str) -> Self {
        debug!("Creating pool: {}", name);
        Self {
            _pool: unsafe { NSAutoreleasePool::new() },
            name,
        }
    }
}

impl Drop for PoolDebug {
    fn drop(&mut self) {
        debug!("Dropping pool: {}", self.name);
    }
}
