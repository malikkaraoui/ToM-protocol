//! The connected client.

use tokio::sync::mpsc;
use tom_protocol::{GroupId, GroupInfo, GroupInvite, GroupMemberRole, MetricsSnapshot, NodeId, RuntimeHandle};
use tom_transport::EndpointAddr;

use crate::error::TomSdkError;
use crate::event::Event;

/// A connected ToM node.
///
/// Created by [`crate::TomClientBuilder::connect`]. Cheap operations — every
/// method is a non-blocking message to the protocol runtime. Consume network
/// activity through [`TomClient::next_event`].
pub struct TomClient {
    handle: RuntimeHandle,
    local_addr: EndpointAddr,
    events: mpsc::Receiver<Event>,
}

impl TomClient {
    pub(crate) fn new(
        handle: RuntimeHandle,
        local_addr: EndpointAddr,
        events: mpsc::Receiver<Event>,
    ) -> Self {
        Self { handle, local_addr, events }
    }

    /// This node's identity. The public key IS the network address.
    pub fn id(&self) -> NodeId {
        self.handle.local_id()
    }

    /// Opaque connectivity ticket for this node (JSON string).
    ///
    /// Share it out-of-band (QR code, copy/paste) and feed it to
    /// [`TomClient::add_peer_ticket`] on the other side to connect two nodes
    /// without any discovery infrastructure.
    ///
    /// # Errors
    ///
    /// [`TomSdkError::InvalidTicket`] if serialization fails (unexpected).
    pub fn ticket(&self) -> Result<String, TomSdkError> {
        serde_json::to_string(&self.local_addr)
            .map_err(|e| TomSdkError::InvalidTicket(e.to_string()))
    }

    /// Register a peer from its connectivity ticket.
    ///
    /// # Errors
    ///
    /// [`TomSdkError::InvalidTicket`] if the ticket does not parse.
    pub async fn add_peer_ticket(&self, ticket: &str) -> Result<(), TomSdkError> {
        let addr: EndpointAddr = serde_json::from_str(ticket)
            .map_err(|e| TomSdkError::InvalidTicket(e.to_string()))?;
        self.handle.add_peer_addr(addr).await;
        Ok(())
    }

    /// Register a peer by identity only — connectivity is resolved through
    /// discovery (relay, DHT, gossip).
    pub async fn add_peer(&self, node_id: NodeId) {
        self.handle.add_peer(node_id).await;
    }

    /// Send raw bytes to a peer. The runtime handles relay selection,
    /// encryption, signing and delivery tracking.
    ///
    /// # Errors
    ///
    /// [`TomSdkError::Protocol`] if the runtime is shut down.
    pub async fn send(&self, to: NodeId, payload: Vec<u8>) -> Result<(), TomSdkError> {
        self.handle.send_message(to, payload).await?;
        Ok(())
    }

    /// Send a UTF-8 text message to a peer.
    ///
    /// # Errors
    ///
    /// Same as [`TomClient::send`].
    pub async fn send_text(&self, to: NodeId, text: &str) -> Result<(), TomSdkError> {
        self.send(to, text.as_bytes().to_vec()).await
    }

    /// Send a read receipt for a received message
    /// (see [`crate::Message::id`]).
    ///
    /// # Errors
    ///
    /// [`TomSdkError::Protocol`] if the runtime is shut down.
    pub async fn send_read_receipt(
        &self,
        to: NodeId,
        message_id: String,
    ) -> Result<(), TomSdkError> {
        self.handle.send_read_receipt(to, message_id).await?;
        Ok(())
    }

    /// Identities of currently connected peers.
    pub async fn connected_peers(&self) -> Vec<NodeId> {
        self.handle.connected_peers().await
    }

    /// Next network event. Returns `None` after [`TomClient::shutdown`].
    ///
    /// This is the single consumption point for everything the node
    /// receives: messages, delivery statuses, peers, groups.
    pub async fn next_event(&mut self) -> Option<Event> {
        self.events.recv().await
    }

    // ── Groups ──────────────────────────────────────────────────────

    /// Create a group hosted by `hub_id` (use [`TomClient::id`] to self-host)
    /// and invite `members`.
    ///
    /// # Errors
    ///
    /// [`TomSdkError::Protocol`] if the runtime is shut down.
    pub async fn create_group(
        &self,
        name: impl Into<String>,
        hub_id: NodeId,
        members: Vec<NodeId>,
    ) -> Result<(), TomSdkError> {
        self.handle.create_group(name.into(), hub_id, members).await?;
        Ok(())
    }

    /// Accept a pending group invitation.
    ///
    /// # Errors
    ///
    /// [`TomSdkError::Protocol`] if the runtime is shut down.
    pub async fn accept_invite(&self, group_id: GroupId) -> Result<(), TomSdkError> {
        self.handle.accept_invite(group_id).await?;
        Ok(())
    }

    /// Decline a pending group invitation.
    ///
    /// # Errors
    ///
    /// [`TomSdkError::Protocol`] if the runtime is shut down.
    pub async fn decline_invite(&self, group_id: GroupId) -> Result<(), TomSdkError> {
        self.handle.decline_invite(group_id).await?;
        Ok(())
    }

    /// Leave a group.
    ///
    /// # Errors
    ///
    /// [`TomSdkError::Protocol`] if the runtime is shut down.
    pub async fn leave_group(&self, group_id: GroupId) -> Result<(), TomSdkError> {
        self.handle.leave_group(group_id).await?;
        Ok(())
    }

    /// Send a text message to a group.
    ///
    /// # Errors
    ///
    /// [`TomSdkError::Protocol`] if the runtime is shut down.
    pub async fn send_group_message(
        &self,
        group_id: GroupId,
        text: impl Into<String>,
    ) -> Result<(), TomSdkError> {
        self.handle.send_group_message(group_id, text.into()).await?;
        Ok(())
    }

    /// Invite a member to an existing group (admin only).
    ///
    /// # Errors
    ///
    /// [`TomSdkError::Protocol`] if the runtime is shut down.
    pub async fn invite_member(
        &self,
        group_id: GroupId,
        member: NodeId,
    ) -> Result<(), TomSdkError> {
        self.handle.invite_member(group_id, member).await?;
        Ok(())
    }

    /// Kick a member from a group (admin only).
    ///
    /// # Errors
    ///
    /// [`TomSdkError::Protocol`] if the runtime is shut down.
    pub async fn kick_member(&self, group_id: GroupId, member: NodeId) -> Result<(), TomSdkError> {
        self.handle.kick_member(group_id, member).await?;
        Ok(())
    }

    /// Change a member's role in a group (admin only).
    ///
    /// # Errors
    ///
    /// [`TomSdkError::Protocol`] if the runtime is shut down.
    pub async fn update_member_role(
        &self,
        group_id: GroupId,
        member: NodeId,
        role: GroupMemberRole,
    ) -> Result<(), TomSdkError> {
        self.handle.update_member_role(group_id, member, role).await?;
        Ok(())
    }

    /// Groups we currently belong to.
    pub async fn groups(&self) -> Vec<GroupInfo> {
        self.handle.groups().await
    }

    /// Pending group invitations.
    pub async fn pending_invites(&self) -> Vec<GroupInvite> {
        self.handle.pending_invites().await
    }

    // ── Lifecycle ───────────────────────────────────────────────────

    /// Snapshot of protocol metrics (messages, relays, encryption counters).
    pub fn metrics(&self) -> MetricsSnapshot {
        self.handle.metrics()
    }

    /// Gracefully stop the node. After this, [`TomClient::next_event`]
    /// drains the remaining events then returns `None`.
    pub async fn shutdown(&self) {
        self.handle.shutdown().await;
    }
}
