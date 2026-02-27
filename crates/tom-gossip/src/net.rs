//! Networking for the `tom-gossip` protocol

use std::{
    collections::{hash_map::Entry, BTreeSet, HashMap, HashSet, VecDeque},
    net::SocketAddr,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

use bytes::Bytes;
use futures_concurrency::stream::{stream_group, StreamGroup};
use futures_util::FutureExt as _;
use tom_connect::{
    endpoint::Connection,
    protocol::{AcceptError, ProtocolHandler},
    Endpoint, EndpointAddr, EndpointId, PublicKey, RelayUrl, Watcher,
};
use irpc::WithChannels;
use n0_error::{e, stack_error};
use n0_future::{
    task::{self, AbortOnDropHandle, JoinSet},
    time::Instant,
    Stream, StreamExt as _,
};
use rand::{rngs::StdRng, SeedableRng};
use serde::{Deserialize, Serialize};
use tokio::sync::{broadcast, mpsc, oneshot};
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, error_span, trace, warn, Instrument};

use self::{
    address_lookup::GossipAddressLookup,
    util::{RecvLoop, SendLoop, Timers},
};
use crate::{
    api::{self, Command, Event, GossipApi, RpcMessage},
    metrics::Metrics,
    proto::{self, HyparviewConfig, PeerData, PlumtreeConfig, Scope, TopicId},
};

mod address_lookup;
mod util;

/// ALPN protocol name
pub const GOSSIP_ALPN: &[u8] = b"/iroh-gossip/1";

/// Channel capacity for the send queue (one per connection)
const SEND_QUEUE_CAP: usize = 64;
/// Channel capacity for the ToActor message queue (single)
const TO_ACTOR_CAP: usize = 64;
/// Channel capacity for the InEvent message queue (single)
const IN_EVENT_CAP: usize = 1024;
/// Channel capacity for broadcast subscriber event queue (one per topic)
const TOPIC_EVENT_CAP: usize = 256;

/// Events emitted from the gossip protocol
pub type ProtoEvent = proto::Event<PublicKey>;
/// Commands for the gossip protocol
pub type ProtoCommand = proto::Command<PublicKey>;

type InEvent = proto::InEvent<PublicKey>;
type OutEvent = proto::OutEvent<PublicKey>;
type Timer = proto::Timer<PublicKey>;
type ProtoMessage = proto::Message<PublicKey>;

/// Publish and subscribe on gossiping topics.
///
/// Each topic is a separate broadcast tree with separate memberships.
///
/// A topic has to be joined before you can publish or subscribe on the topic.
/// To join the swarm for a topic, you have to know the [`PublicKey`] of at least one peer that also joined the topic.
///
/// Messages published on the swarm will be delivered to all peers that joined the swarm for that
/// topic. You will also be relaying (gossiping) messages published by other peers.
///
/// With the default settings, the protocol will maintain up to 5 peer connections per topic.
///
/// Even though the [`Gossip`] is created from a [`Endpoint`], it does not accept connections
/// itself. You should run an accept loop on the [`Endpoint`] yourself, check the ALPN protocol of incoming
/// connections, and if the ALPN protocol equals [`GOSSIP_ALPN`], forward the connection to the
/// gossip actor through [Self::handle_connection].
///
/// The gossip actor will, however, initiate new connections to other peers by itself.
#[derive(Debug, Clone)]
pub struct Gossip {
    pub(crate) inner: Arc<Inner>,
}

impl std::ops::Deref for Gossip {
    type Target = GossipApi;
    fn deref(&self) -> &Self::Target {
        &self.inner.api
    }
}

#[derive(Debug)]
enum LocalActorMessage {
    HandleConnection(Connection),
    Shutdown { reply: oneshot::Sender<()> },
}

#[allow(missing_docs)]
#[stack_error(derive, add_meta)]
#[non_exhaustive]
pub enum Error {
    ActorDropped {},
}

impl<T> From<mpsc::error::SendError<T>> for Error {
    fn from(_value: mpsc::error::SendError<T>) -> Self {
        e!(Error::ActorDropped)
    }
}
impl From<oneshot::error::RecvError> for Error {
    fn from(_value: oneshot::error::RecvError) -> Self {
        e!(Error::ActorDropped)
    }
}

#[derive(Debug)]
pub(crate) struct Inner {
    api: GossipApi,
    local_tx: mpsc::Sender<LocalActorMessage>,
    _actor_handle: AbortOnDropHandle<()>,
    max_message_size: usize,
    metrics: Arc<Metrics>,
}

impl ProtocolHandler for Gossip {
    async fn accept(&self, connection: Connection) -> Result<(), AcceptError> {
        self.handle_connection(connection)
            .await
            .map_err(AcceptError::from_err)?;
        Ok(())
    }

    async fn shutdown(&self) {
        if let Err(err) = self.shutdown().await {
            warn!("error while shutting down gossip: {err:#}");
        }
    }
}

/// Builder to configure and construct [`Gossip`].
#[derive(Debug, Clone)]
pub struct Builder {
    config: proto::Config,
    alpn: Option<Bytes>,
}

impl Builder {
    /// Sets the maximum message size in bytes.
    /// By default this is `4096` bytes.
    pub fn max_message_size(mut self, size: usize) -> Self {
        self.config.max_message_size = size;
        self
    }

    /// Set the membership configuration.
    pub fn membership_config(mut self, config: HyparviewConfig) -> Self {
        self.config.membership = config;
        self
    }

    /// Set the broadcast configuration.
    pub fn broadcast_config(mut self, config: PlumtreeConfig) -> Self {
        self.config.broadcast = config;
        self
    }

    /// Set the ALPN this gossip instance uses.
    ///
    /// It has to be the same for all peers in the network. If you set a custom ALPN,
    /// you have to use the same ALPN when registering the [`Gossip`] on a
    /// router with [`RouterBuilder::accept`].
    ///
    /// [`RouterBuilder::accept`]: tom_connect::protocol::RouterBuilder::accept
    pub fn alpn(mut self, alpn: impl AsRef<[u8]>) -> Self {
        self.alpn = Some(alpn.as_ref().to_vec().into());
        self
    }

    /// Spawn a gossip actor and get a handle for it
    pub fn spawn(self, endpoint: Endpoint) -> Gossip {
        let metrics = Arc::new(Metrics::default());
        let address_lookup = GossipAddressLookup::default();
        endpoint.address_lookup().add(address_lookup.clone());
        let (actor, rpc_tx, local_tx) = Actor::new(
            endpoint,
            self.config,
            metrics.clone(),
            self.alpn,
            address_lookup,
        );
        let me = actor.endpoint.id().fmt_short();
        let max_message_size = actor.state.max_message_size();

        let actor_handle = task::spawn(actor.run().instrument(error_span!("gossip", %me)));

        let api = GossipApi::local(rpc_tx);

        Gossip {
            inner: Inner {
                api,
                local_tx,
                _actor_handle: AbortOnDropHandle::new(actor_handle),
                max_message_size,
                metrics,
            }
            .into(),
        }
    }
}

impl Gossip {
    /// Creates a default `Builder`, with the endpoint set.
    pub fn builder() -> Builder {
        Builder {
            config: Default::default(),
            alpn: None,
        }
    }

    /// Get the maximum message size configured for this gossip actor.
    pub fn max_message_size(&self) -> usize {
        self.inner.max_message_size
    }

    /// Handle an incoming [`Connection`].
    ///
    /// Make sure to check the ALPN protocol yourself before passing the connection.
    pub async fn handle_connection(&self, conn: Connection) -> Result<(), Error> {
        self.inner
            .local_tx
            .send(LocalActorMessage::HandleConnection(conn))
            .await?;
        Ok(())
    }

    /// Shutdown the gossip instance.
    ///
    /// This leaves all topics, sending `Disconnect` messages to peers, and then
    /// stops the gossip actor loop and drops all state and connections.
    pub async fn shutdown(&self) -> Result<(), Error> {
        let (reply, reply_rx) = oneshot::channel();
        self.inner
            .local_tx
            .send(LocalActorMessage::Shutdown { reply })
            .await?;
        reply_rx.await?;
        Ok(())
    }

    /// Returns the metrics tracked for this gossip instance.
    pub fn metrics(&self) -> &Arc<Metrics> {
        &self.inner.metrics
    }
}

/// Actor that sends and handles messages between the connection and main state loops
struct Actor {
    alpn: Bytes,
    /// Protocol state
    state: proto::State<PublicKey, StdRng>,
    /// The endpoint through which we dial peers
    endpoint: Endpoint,
    /// Dial machine to connect to peers
    dialer: Dialer,
    /// Input messages to the actor
    rpc_rx: mpsc::Receiver<RpcMessage>,
    local_rx: mpsc::Receiver<LocalActorMessage>,
    /// Sender for the state input (cloned into the connection loops)
    in_event_tx: mpsc::Sender<InEvent>,
    /// Input events to the state (emitted from the connection loops)
    in_event_rx: mpsc::Receiver<InEvent>,
    /// Queued timers
    timers: Timers<Timer>,
    /// Map of topics to their state.
    topics: HashMap<TopicId, TopicState>,
    /// Map of peers to their state.
    peers: HashMap<EndpointId, PeerState>,
    /// Stream of commands from topic handles.
    command_rx: stream_group::Keyed<TopicCommandStream>,
    /// Internal queue of topic to close because all handles were dropped.
    quit_queue: VecDeque<TopicId>,
    /// Tasks for the connection loops, to keep track of panics.
    connection_tasks: JoinSet<(EndpointId, Connection, Result<(), ConnectionLoopError>)>,
    metrics: Arc<Metrics>,
    topic_event_forwarders: JoinSet<TopicId>,
    address_lookup: GossipAddressLookup,
}

impl Actor {
    fn new(
        endpoint: Endpoint,
        config: proto::Config,
        metrics: Arc<Metrics>,
        alpn: Option<Bytes>,
        address_lookup: GossipAddressLookup,
    ) -> (
        Self,
        mpsc::Sender<RpcMessage>,
        mpsc::Sender<LocalActorMessage>,
    ) {
        let peer_id = endpoint.id();
        let dialer = Dialer::new(endpoint.clone());
        let state = proto::State::new(
            peer_id,
            Default::default(),
            config,
            rand::rngs::StdRng::from_rng(&mut rand::rng()),
        );
        let (rpc_tx, rpc_rx) = mpsc::channel(TO_ACTOR_CAP);
        let (local_tx, local_rx) = mpsc::channel(16);
        let (in_event_tx, in_event_rx) = mpsc::channel(IN_EVENT_CAP);

        let actor = Actor {
            alpn: alpn.unwrap_or_else(|| GOSSIP_ALPN.to_vec().into()),
            endpoint,
            state,
            dialer,
            rpc_rx,
            in_event_rx,
            in_event_tx,
            timers: Timers::new(),
            command_rx: StreamGroup::new().keyed(),
            peers: Default::default(),
            topics: Default::default(),
            quit_queue: Default::default(),
            connection_tasks: Default::default(),
            metrics,
            local_rx,
            topic_event_forwarders: Default::default(),
            address_lookup,
        };

        (actor, rpc_tx, local_tx)
    }

    pub async fn run(mut self) {
        let mut addr_update_stream = self.setup().await;

        let mut i = 0;
        while self.event_loop(&mut addr_update_stream, i).await {
            i += 1;
        }
    }

    /// Performs the initial actor setup to run the [`Actor::event_loop`].
    ///
    /// This updates our current address and return it. It also returns the home relay stream and
    /// direct addr stream.
    async fn setup(&mut self) -> impl Stream<Item = EndpointAddr> + Send + Unpin + use<> {
        let addr_update_stream = self.endpoint.watch_addr().stream();
        let initial_addr = self.endpoint.addr();
        self.handle_addr_update(initial_addr).await;
        addr_update_stream
    }

    /// One event loop processing step.
    ///
    /// None is returned when no further processing should be performed.
    async fn event_loop(
        &mut self,
        addr_updates: &mut (impl Stream<Item = EndpointAddr> + Send + Unpin),
        i: usize,
    ) -> bool {
        self.metrics.actor_tick_main.inc();
        tokio::select! {
            biased;
            conn = self.local_rx.recv() => {
                match conn {
                    Some(LocalActorMessage::Shutdown { reply }) => {
                        debug!("received shutdown message, quit all topics");
                        self.quit_queue.extend(self.topics.keys().copied());
                        self.process_quit_queue().await;
                        debug!("all topics quit, stop gossip actor");
                        reply.send(()).ok();
                        return false;
                    },
                    Some(LocalActorMessage::HandleConnection(conn)) => {
                        self.handle_connection(conn.remote_id(), ConnOrigin::Accept, conn);
                    }
                    None => {
                        debug!("all gossip handles dropped, stop gossip actor");
                        return false;
                    }
                }
            }
            msg = self.rpc_rx.recv() => {
                trace!(?i, "tick: to_actor_rx");
                self.metrics.actor_tick_rx.inc();
                match msg {
                    Some(msg) => {
                        self.handle_rpc_msg(msg, Instant::now()).await;
                    }
                    None => {
                        debug!("all gossip handles dropped, stop gossip actor");
                        return false;
                    }
                }
            },
            Some((key, (topic, command))) = self.command_rx.next(), if !self.command_rx.is_empty() => {
                trace!(?i, "tick: command_rx");
                self.handle_command(topic, key, command).await;
            },
            Some(new_address) = addr_updates.next() => {
                trace!(?i, "tick: new_address");
                self.metrics.actor_tick_endpoint.inc();
                self.handle_addr_update(new_address).await;
            }
            (peer_id, res) = self.dialer.next_conn() => {
                trace!(?i, "tick: dialer");
                self.metrics.actor_tick_dialer.inc();
                match res {
                    Some(Ok(conn)) => {
                        debug!(peer = %peer_id.fmt_short(), "dial successful");
                        self.metrics.actor_tick_dialer_success.inc();
                        self.handle_connection(peer_id, ConnOrigin::Dial, conn);
                    }
                    Some(Err(err)) => {
                        warn!(peer = %peer_id.fmt_short(), "dial failed: {err}");
                        self.metrics.actor_tick_dialer_failure.inc();
                        let peer_state = self.peers.get(&peer_id);
                        let is_active = matches!(peer_state, Some(PeerState::Active { .. }));
                        if !is_active {
                            self.handle_in_event(InEvent::PeerDisconnected(peer_id), Instant::now())
                                .await;
                        }
                    }
                    None => {
                        warn!(peer = %peer_id.fmt_short(), "dial disconnected");
                        self.metrics.actor_tick_dialer_failure.inc();
                    }
                }
            }
            event = self.in_event_rx.recv() => {
                trace!(?i, "tick: in_event_rx");
                self.metrics.actor_tick_in_event_rx.inc();
                let event = event.expect("unreachable: in_event_tx is never dropped before receiver");
                self.handle_in_event(event, Instant::now()).await;
            }
            _ = self.timers.wait_next() => {
                trace!(?i, "tick: timers");
                self.metrics.actor_tick_timers.inc();
                let now = Instant::now();
                while let Some((_instant, timer)) = self.timers.pop_before(now) {
                    self.handle_in_event(InEvent::TimerExpired(timer), now).await;
                }
            }
            Some(res) = self.connection_tasks.join_next(), if !self.connection_tasks.is_empty() => {
                trace!(?i, "tick: connection_tasks");
                let (peer_id, conn, result) = res.expect("connection task panicked");
                self.handle_connection_task_finished(peer_id, conn, result).await;
            }
            Some(res) = self.topic_event_forwarders.join_next(), if !self.topic_event_forwarders.is_empty() => {
                let topic_id = res.expect("topic event forwarder panicked");
                if let Some(state) = self.topics.get_mut(&topic_id)
                    && !state.still_needed()
                {
                    self.quit_queue.push_back(topic_id);
                    self.process_quit_queue().await;
                }
            }
        }

        true
    }

    async fn handle_addr_update(&mut self, endpoint_addr: EndpointAddr) {
        // let peer_data = our_peer_data(&self.endpoint, current_addresses);
        let peer_data = encode_peer_data(&endpoint_addr.into());
        self.handle_in_event(InEvent::UpdatePeerData(peer_data), Instant::now())
            .await
    }

    async fn handle_command(
        &mut self,
        topic: TopicId,
        key: stream_group::Key,
        command: Option<Command>,
    ) {
        debug!(?topic, ?key, ?command, "handle command");
        let Some(state) = self.topics.get_mut(&topic) else {
            // TODO: unreachable?
            warn!("received command for unknown topic");
            return;
        };
        match command {
            Some(command) => {
                let command = match command {
                    Command::Broadcast(message) => ProtoCommand::Broadcast(message, Scope::Swarm),
                    Command::BroadcastNeighbors(message) => {
                        ProtoCommand::Broadcast(message, Scope::Neighbors)
                    }
                    Command::JoinPeers(peers) => ProtoCommand::Join(peers),
                };
                self.handle_in_event(proto::InEvent::Command(topic, command), Instant::now())
                    .await;
            }
            None => {
                state.command_rx_keys.remove(&key);
                if !state.still_needed() {
                    self.quit_queue.push_back(topic);
                    self.process_quit_queue().await;
                }
            }
        }
    }

    fn handle_connection(&mut self, peer_id: EndpointId, origin: ConnOrigin, conn: Connection) {
        let (send_tx, send_rx) = mpsc::channel(SEND_QUEUE_CAP);
        let conn_id = conn.stable_id();

        let queue = match self.peers.entry(peer_id) {
            Entry::Occupied(mut entry) => entry.get_mut().accept_conn(send_tx, conn_id),
            Entry::Vacant(entry) => {
                entry.insert(PeerState::Active {
                    active_send_tx: send_tx,
                    active_conn_id: conn_id,
                    other_conns: Vec::new(),
                });
                Vec::new()
            }
        };

        let max_message_size = self.state.max_message_size();
        let in_event_tx = self.in_event_tx.clone();

        // Spawn a task for this connection
        self.connection_tasks.spawn(
            async move {
                let res = connection_loop(
                    peer_id,
                    conn.clone(),
                    origin,
                    send_rx,
                    in_event_tx,
                    max_message_size,
                    queue,
                )
                .await;
                (peer_id, conn, res)
            }
            .instrument(error_span!("conn", peer = %peer_id.fmt_short())),
        );
    }

    #[tracing::instrument(name = "conn", skip_all, fields(peer = %peer_id.fmt_short()))]
    async fn handle_connection_task_finished(
        &mut self,
        peer_id: EndpointId,
        conn: Connection,
        task_result: Result<(), ConnectionLoopError>,
    ) {
        if conn.close_reason().is_none() {
            conn.close(0u32.into(), b"close from disconnect");
        }
        let reason = conn.close_reason().expect("just closed");
        let error = task_result.err();
        debug!(%reason, ?error, "connection closed");
        if let Some(PeerState::Active {
            active_conn_id,
            other_conns,
            ..
        }) = self.peers.get_mut(&peer_id)
        {
            if conn.stable_id() == *active_conn_id {
                debug!("active send connection closed, mark peer as disconnected");
                self.handle_in_event(InEvent::PeerDisconnected(peer_id), Instant::now())
                    .await;
            } else {
                other_conns.retain(|x| *x != conn.stable_id());
                debug!("remaining {} other connections", other_conns.len() + 1);
            }
        } else {
            debug!("peer already marked as disconnected");
        }
    }

    async fn handle_rpc_msg(&mut self, msg: RpcMessage, now: Instant) {
        trace!("handle to_actor  {msg:?}");
        match msg {
            RpcMessage::Join(msg) => {
                let WithChannels {
                    inner,
                    rx,
                    tx,
                    // TODO(frando): make use of span?
                    span: _,
                } = msg;
                let api::JoinRequest {
                    topic_id,
                    bootstrap,
                } = inner;
                let TopicState {
                    neighbors,
                    event_sender,
                    command_rx_keys,
                } = self.topics.entry(topic_id).or_default();
                let mut sender_dead = false;
                if !neighbors.is_empty() {
                    for neighbor in neighbors.iter() {
                        if let Err(_err) = tx.try_send(Event::NeighborUp(*neighbor)).await {
                            sender_dead = true;
                            break;
                        }
                    }
                }

                if !sender_dead {
                    let fut =
                        topic_subscriber_loop(tx, event_sender.subscribe()).map(move |_| topic_id);
                    self.topic_event_forwarders
                        .spawn(fut.instrument(tracing::Span::current()));
                }
                let command_rx = TopicCommandStream::new(topic_id, Box::pin(rx.into_stream()));
                let key = self.command_rx.insert(command_rx);
                command_rx_keys.insert(key);

                self.handle_in_event(
                    InEvent::Command(
                        topic_id,
                        ProtoCommand::Join(bootstrap.into_iter().collect()),
                    ),
                    now,
                )
                .await;
            }
        }
    }

    async fn handle_in_event(&mut self, event: InEvent, now: Instant) {
        self.handle_in_event_inner(event, now).await;
        self.process_quit_queue().await;
    }

    async fn process_quit_queue(&mut self) {
        while let Some(topic_id) = self.quit_queue.pop_front() {
            self.handle_in_event_inner(
                InEvent::Command(topic_id, ProtoCommand::Quit),
                Instant::now(),
            )
            .await;
            if self.topics.remove(&topic_id).is_some() {
                tracing::debug!(%topic_id, "publishers and subscribers gone; unsubscribing");
            }
        }
    }

    async fn handle_in_event_inner(&mut self, event: InEvent, now: Instant) {
        if matches!(event, InEvent::TimerExpired(_)) {
            trace!(?event, "handle in_event");
        } else {
            debug!(?event, "handle in_event");
        };
        let out = self.state.handle(event, now, Some(&self.metrics));
        for event in out {
            if matches!(event, OutEvent::ScheduleTimer(_, _)) {
                trace!(?event, "handle out_event");
            } else {
                debug!(?event, "handle out_event");
            };
            match event {
                OutEvent::SendMessage(peer_id, message) => {
                    let state = self.peers.entry(peer_id).or_default();
                    match state {
                        PeerState::Active { active_send_tx, .. } => {
                            if let Err(_err) = active_send_tx.send(message).await {
                                // Removing the peer is handled by the in_event PeerDisconnected sent
                                // in [`Self::handle_connection_task_finished`].
                                warn!(
                                    peer = %peer_id.fmt_short(),
                                    "failed to send: connection task send loop terminated",
                                );
                            }
                        }
                        PeerState::Pending { queue } => {
                            if queue.is_empty() {
                                debug!(peer = %peer_id.fmt_short(), "start to dial");
                                self.dialer.queue_dial(peer_id, self.alpn.clone());
                            }
                            queue.push(message);
                        }
                    }
                }
                OutEvent::EmitEvent(topic_id, event) => {
                    let Some(state) = self.topics.get_mut(&topic_id) else {
                        // TODO: unreachable?
                        warn!(?topic_id, "gossip state emitted event for unknown topic");
                        continue;
                    };
                    let TopicState {
                        neighbors,
                        event_sender,
                        ..
                    } = state;
                    match &event {
                        ProtoEvent::NeighborUp(neighbor) => {
                            neighbors.insert(*neighbor);
                        }
                        ProtoEvent::NeighborDown(neighbor) => {
                            neighbors.remove(neighbor);
                        }
                        _ => {}
                    }
                    event_sender.send(event).ok();
                    if !state.still_needed() {
                        self.quit_queue.push_back(topic_id);
                    }
                }
                OutEvent::ScheduleTimer(delay, timer) => {
                    self.timers.insert(now + delay, timer);
                }
                OutEvent::DisconnectPeer(peer_id) => {
                    // signal disconnection by dropping the senders to the connection
                    debug!(peer=%peer_id.fmt_short(), "gossip state indicates disconnect: drop peer");
                    self.peers.remove(&peer_id);
                }
                OutEvent::PeerData(endpoint_id, data) => match decode_peer_data(&data) {
                    Err(err) => warn!("Failed to decode {data:?} from {endpoint_id}: {err}"),
                    Ok(info) => {
                        debug!(peer = ?endpoint_id, "add known addrs: {info:?}");
                        let mut endpoint_addr = EndpointAddr::new(endpoint_id);
                        for addr in info.direct_addresses {
                            endpoint_addr = endpoint_addr.with_ip_addr(addr);
                        }
                        if let Some(relay_url) = info.relay_url {
                            endpoint_addr = endpoint_addr.with_relay_url(relay_url);
                        }

                        self.address_lookup.add(endpoint_addr);
                    }
                },
            }
        }
    }
}

type ConnId = usize;

#[derive(Debug)]
enum PeerState {
    Pending {
        queue: Vec<ProtoMessage>,
    },
    Active {
        active_send_tx: mpsc::Sender<ProtoMessage>,
        active_conn_id: ConnId,
        other_conns: Vec<ConnId>,
    },
}

impl PeerState {
    fn accept_conn(
        &mut self,
        send_tx: mpsc::Sender<ProtoMessage>,
        conn_id: ConnId,
    ) -> Vec<ProtoMessage> {
        match self {
            PeerState::Pending { queue } => {
                let queue = std::mem::take(queue);
                *self = PeerState::Active {
                    active_send_tx: send_tx,
                    active_conn_id: conn_id,
                    other_conns: Vec::new(),
                };
                queue
            }
            PeerState::Active {
                active_send_tx,
                active_conn_id,
                other_conns,
            } => {
                // We already have an active connection. We keep the old connection intact,
                // but only use the new connection for sending from now on.
                // By dropping the `send_tx` of the old connection, the send loop part of
                // the `connection_loop` of the old connection will terminate, which will also
                // notify the peer that the old connection may be dropped.
                other_conns.push(*active_conn_id);
                *active_send_tx = send_tx;
                *active_conn_id = conn_id;
                Vec::new()
            }
        }
    }
}

impl Default for PeerState {
    fn default() -> Self {
        PeerState::Pending { queue: Vec::new() }
    }
}

#[derive(Debug)]
struct TopicState {
    neighbors: BTreeSet<EndpointId>,
    event_sender: broadcast::Sender<ProtoEvent>,
    /// Keys identifying command receivers in [`Actor::command_rx`].
    ///
    /// This represents the receiver side of gossip's publish public API.
    command_rx_keys: HashSet<stream_group::Key>,
}

impl Default for TopicState {
    fn default() -> Self {
        let (event_sender, _) = broadcast::channel(TOPIC_EVENT_CAP);
        Self {
            neighbors: Default::default(),
            command_rx_keys: Default::default(),
            event_sender,
        }
    }
}

impl TopicState {
    /// Check if the topic still has any publisher or subscriber.
    fn still_needed(&self) -> bool {
        // Keep topic alive if either senders or receivers exist.
        // Using || prevents topic closure when senders are dropped while receivers listen.
        !self.command_rx_keys.is_empty() || self.event_sender.receiver_count() > 0
    }

}

/// Whether a connection is initiated by us (Dial) or by the remote peer (Accept)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ConnOrigin {
    Accept,
    Dial,
}

#[allow(missing_docs)]
#[stack_error(derive, add_meta, from_sources, std_sources)]
#[non_exhaustive]
enum ConnectionLoopError {
    #[error(transparent)]
    Write {
        source: self::util::WriteError,
    },
    #[error(transparent)]
    Read {
        source: self::util::ReadError,
    },
    #[error(transparent)]
    Connection {
        #[error(std_err)]
        source: tom_connect::endpoint::ConnectionError,
    },
    ActorDropped {},
}

impl<T> From<mpsc::error::SendError<T>> for ConnectionLoopError {
    fn from(_value: mpsc::error::SendError<T>) -> Self {
        e!(ConnectionLoopError::ActorDropped)
    }
}

async fn connection_loop(
    from: PublicKey,
    conn: Connection,
    origin: ConnOrigin,
    send_rx: mpsc::Receiver<ProtoMessage>,
    in_event_tx: mpsc::Sender<InEvent>,
    max_message_size: usize,
    queue: Vec<ProtoMessage>,
) -> Result<(), ConnectionLoopError> {
    debug!(?origin, "connection established");

    let mut send_loop = SendLoop::new(conn.clone(), send_rx, max_message_size);
    let mut recv_loop = RecvLoop::new(from, conn, in_event_tx, max_message_size);

    let send_fut = send_loop.run(queue).instrument(error_span!("send"));
    let recv_fut = recv_loop.run().instrument(error_span!("recv"));

    let (send_res, recv_res) = tokio::join!(send_fut, recv_fut);
    send_res?;
    recv_res?;
    Ok(())
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
struct AddrInfo {
    relay_url: Option<RelayUrl>,
    direct_addresses: BTreeSet<SocketAddr>,
}

impl From<EndpointAddr> for AddrInfo {
    fn from(endpoint_addr: EndpointAddr) -> Self {
        Self {
            relay_url: endpoint_addr.relay_urls().next().cloned(),
            direct_addresses: endpoint_addr.ip_addrs().cloned().collect(),
        }
    }
}

fn encode_peer_data(info: &AddrInfo) -> PeerData {
    let bytes = postcard::to_stdvec(info).expect("serializing AddrInfo may not fail");
    PeerData::new(bytes)
}

fn decode_peer_data(peer_data: &PeerData) -> Result<AddrInfo, postcard::Error> {
    let bytes = peer_data.as_bytes();
    if bytes.is_empty() {
        return Ok(AddrInfo::default());
    }
    let info = postcard::from_bytes(bytes)?;
    Ok(info)
}

async fn topic_subscriber_loop(
    sender: irpc::channel::mpsc::Sender<Event>,
    mut topic_events: broadcast::Receiver<ProtoEvent>,
) {
    loop {
        tokio::select! {
           biased;
           msg = topic_events.recv() => {
               let event = match msg {
                   Err(broadcast::error::RecvError::Closed) => break,
                   Err(broadcast::error::RecvError::Lagged(_)) => Event::Lagged,
                   Ok(event) => event.into(),
               };
               if sender.send(event).await.is_err() {
                   break;
               }
           }
           _ = sender.closed() => break,
        }
    }
}

/// A stream of commands for a gossip subscription.
type BoxedCommandReceiver =
    n0_future::stream::Boxed<Result<Command, irpc::channel::mpsc::RecvError>>;

#[derive(derive_more::Debug)]
struct TopicCommandStream {
    topic_id: TopicId,
    #[debug("CommandStream")]
    stream: BoxedCommandReceiver,
    closed: bool,
}

impl TopicCommandStream {
    fn new(topic_id: TopicId, stream: BoxedCommandReceiver) -> Self {
        Self {
            topic_id,
            stream,
            closed: false,
        }
    }
}

impl Stream for TopicCommandStream {
    type Item = (TopicId, Option<Command>);
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if self.closed {
            return Poll::Ready(None);
        }
        match Pin::new(&mut self.stream).poll_next(cx) {
            Poll::Ready(Some(Ok(item))) => Poll::Ready(Some((self.topic_id, Some(item)))),
            Poll::Ready(None) | Poll::Ready(Some(Err(_))) => {
                self.closed = true;
                Poll::Ready(Some((self.topic_id, None)))
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

#[derive(Debug)]
struct Dialer {
    endpoint: Endpoint,
    pending: JoinSet<(
        EndpointId,
        Option<Result<Connection, tom_connect::endpoint::ConnectError>>,
    )>,
    pending_dials: HashMap<EndpointId, CancellationToken>,
}

impl Dialer {
    /// Create a new dialer for a [`Endpoint`]
    fn new(endpoint: Endpoint) -> Self {
        Self {
            endpoint,
            pending: Default::default(),
            pending_dials: Default::default(),
        }
    }

    /// Starts to dial a endpoint by [`EndpointId`].
    fn queue_dial(&mut self, endpoint_id: EndpointId, alpn: Bytes) {
        if self.is_pending(endpoint_id) {
            return;
        }
        let cancel = CancellationToken::new();
        self.pending_dials.insert(endpoint_id, cancel.clone());
        let endpoint = self.endpoint.clone();
        self.pending.spawn(
            async move {
                let res = tokio::select! {
                    biased;
                    _ = cancel.cancelled() => None,
                    res = endpoint.connect(endpoint_id, &alpn) => Some(res),
                };
                (endpoint_id, res)
            }
            .instrument(tracing::Span::current()),
        );
    }

    /// Checks if a endpoint is currently being dialed.
    fn is_pending(&self, endpoint: EndpointId) -> bool {
        self.pending_dials.contains_key(&endpoint)
    }

    /// Waits for the next dial operation to complete.
    /// `None` means disconnected
    async fn next_conn(
        &mut self,
    ) -> (
        EndpointId,
        Option<Result<Connection, tom_connect::endpoint::ConnectError>>,
    ) {
        match self.pending_dials.is_empty() {
            false => {
                let (endpoint_id, res) = loop {
                    match self.pending.join_next().await {
                        Some(Ok((endpoint_id, res))) => {
                            self.pending_dials.remove(&endpoint_id);
                            break (endpoint_id, res);
                        }
                        Some(Err(e)) => {
                            error!("next conn error: {:?}", e);
                        }
                        None => {
                            error!("no more pending conns available");
                            std::future::pending().await
                        }
                    }
                };

                (endpoint_id, res)
            }
            true => std::future::pending().await,
        }
    }
}

// Tests stripped — depend on iroh::test_utils (relay server setup).
// Will be restored when tom-connect has its own test utilities.
