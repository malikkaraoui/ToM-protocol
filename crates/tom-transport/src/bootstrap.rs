use tom_connect::EndpointAddr;
use tom_connect::address_lookup::DiscoveryEvent as MdnsDiscoveryEvent;

/// Best-effort bootstrap signals surfaced by the transport layer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BootstrapHint {
    /// A peer was discovered on the local network via mDNS.
    MdnsDiscovered {
        endpoint_addr: EndpointAddr,
    },
}

/// Convert an mDNS discovery event into a runtime-consumable bootstrap hint.
pub(crate) fn mdns_event_to_hint(event: MdnsDiscoveryEvent) -> Option<BootstrapHint> {
    match event {
        MdnsDiscoveryEvent::Discovered { endpoint_info, .. } => {
            Some(BootstrapHint::MdnsDiscovered {
                endpoint_addr: endpoint_info.to_endpoint_addr(),
            })
        }
        MdnsDiscoveryEvent::Expired { .. } => None,
    }
}