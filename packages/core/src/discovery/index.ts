export { NetworkTopology } from './network-topology.js';
export type { PeerInfo, PeerStatus, NodeRole } from './network-topology.js';
export { HeartbeatManager } from './heartbeat.js';
export type { HeartbeatEvents, HeartbeatSender } from './heartbeat.js';

// Peer Gossip (Story 7.1 - Bootstrap Fade)
export { PeerGossip, isPeerGossipMessage } from './peer-gossip.js';
export type {
  GossipPeerInfo,
  PeerGossipMessage,
  PeerGossipEvents,
  PeerGossipConfig,
} from './peer-gossip.js';

// Ephemeral Subnets (Story 7.2 - Sliding Genesis)
export { EphemeralSubnetManager } from './ephemeral-subnet.js';
export type {
  SubnetInfo,
  CommunicationEdge,
  SubnetEvents,
  SubnetConfig,
} from './ephemeral-subnet.js';
