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

use objc2::AllocAnyThread;
use objc2::MainThreadOnly;

use std::fmt;
use std::marker::PhantomData;
use std::sync::{Arc, RwLock};

use log::{debug, error, info, trace, warn};

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

            let formatted_name = format!("iroh-{}", service_name)
                .chars()
                .filter(|c| c.is_ascii_alphanumeric() || *c == '-')
                .take(15)
                .collect::<String>();

            let service_type = NSString::from_str(&formatted_name);

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
        unsafe {
            let _pool = AutoreleasePool::new();

            let mc_session = {
                let _inner_pool = AutoreleasePool::new();
                MCSession::initWithPeer(MCSession::alloc(), self.peer_id.as_ref())
            };

            let delegate_obj = {
                let _inner_pool = AutoreleasePool::new();

                let delegate_impl = SessionDelegate::new(
                    self.on_data_received.take(),
                    self.on_peer_joined.take(),
                    self.on_peer_left.take(),
                );

                let boxed = Box::new(delegate_impl);
                let raw_ptr = Box::into_raw(boxed);
                let retained = Retained::from_raw(raw_ptr).expect("Failed to retain delegate");
                ProtocolObject::from_retained(retained)
            };

            {
                let _inner_pool = AutoreleasePool::new();
                mc_session.setDelegate(Some(&delegate_obj));
            }

            let advertiser = {
                let _inner_pool = AutoreleasePool::new();
                let adv = MCNearbyServiceAdvertiser::initWithPeer_discoveryInfo_serviceType(
                    MCNearbyServiceAdvertiser::alloc(),
                    self.peer_id.as_ref(),
                    None,
                    self.service_type.as_ref(),
                );
                adv.startAdvertisingPeer();
                adv
            };

            let browser = {
                let _inner_pool = AutoreleasePool::new();
                let br = MCNearbyServiceBrowser::initWithPeer_serviceType(
                    MCNearbyServiceBrowser::alloc(),
                    self.peer_id.as_ref(),
                    self.service_type.as_ref(),
                );
                br.startBrowsingForPeers();
                br
            };

            {
                let _inner_pool = AutoreleasePool::new();
                self.session = Some(mc_session);
                self.service_advertiser = Some(advertiser);
                self.service_browser = Some(browser);
                self.delegate = Some(delegate_obj);
            }
        }
    }

    pub fn send_to_peers(
        &self,
        data: &[u8],
        peers: &[Retained<MCPeerID>],
        reliably: bool,
    ) -> Result<(), String> {
        if let Some(session) = &self.session {
            unsafe {
                let _pool = AutoreleasePool::new();

                let ns_data = NSData::from_vec(data.to_vec());
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
        if let Some(session) = &self.session {
            unsafe {
                let _pool = AutoreleasePool::new();

                let peer_array = unsafe { session.connectedPeers() };
                let count = unsafe { peer_array.count() };
                let mut peers = Vec::with_capacity(count as usize);

                for i in 0..count {
                    let peer = unsafe { peer_array.objectAtIndex(i) };
                    peers.push(peer);
                }
                peers
            }
        } else {
            Vec::new()
        }
    }
}

// #[thread_kind = SafeMainThreadOnly]
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
        unsafe {
            let _pool = AutoreleasePool::new();
            match state {
                MCSessionState::Connected => {
                    if let Some(cb) = &self.on_peer_joined {
                        unsafe { cb(peer_id) };
                    }
                }
                MCSessionState::NotConnected => {
                    if let Some(cb) = &self.on_peer_left {
                        unsafe { cb(peer_id) };
                    }
                }
                _ => {}
            }
        }
    }

    unsafe fn session_didReceiveData_fromPeer(
        &self,
        _session: &MCSession,
        data: &NSData,
        peer_id: &MCPeerID,
    ) {
        unsafe {
            let _pool = AutoreleasePool::new();
            if let Some(cb) = &self.on_data_received {
                unsafe { cb(data, peer_id) };
            }
        }
    }

    unsafe fn session_didReceiveStream_withName_fromPeer(
        &self,
        _session: &MCSession,
        _stream: &NSInputStream,
        _stream_name: &NSString,
        _peer_id: &MCPeerID,
    ) {
        unsafe {
            let _pool = AutoreleasePool::new();
        }
    }

    unsafe fn session_didStartReceivingResourceWithName_fromPeer_withProgress(
        &self,
        _session: &MCSession,
        _resource_name: &NSString,
        _peer_id: &MCPeerID,
        _progress: &NSProgress,
    ) {
        unsafe {
            let _pool = AutoreleasePool::new();
        }
    }

    unsafe fn session_didFinishReceivingResourceWithName_fromPeer_atURL_withError(
        &self,
        _session: &MCSession,
        _resource_name: &NSString,
        _peer_id: &MCPeerID,
        _location_url: Option<&NSURL>,
        _error: Option<&NSError>,
    ) {
        unsafe {
            let _pool = AutoreleasePool::new();
        }
    }
}

struct AutoreleasePool {
    _pool: Retained<NSAutoreleasePool>,
}

impl AutoreleasePool {
    unsafe fn new() -> Self {
        Self {
            _pool: unsafe { NSAutoreleasePool::new() },
        }
    }
}
