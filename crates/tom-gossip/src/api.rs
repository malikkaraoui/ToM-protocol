//! Public API for using tom-gossip
//!
//! The API is usable both locally and over RPC.

use std::{
    collections::{BTreeSet, HashSet},
    pin::Pin,
    task::{Context, Poll},
};

use bytes::Bytes;
use tom_base::EndpointId;
use irpc::{channel::mpsc, rpc_requests, Client};
use n0_error::{e, stack_error};
use n0_future::{Stream, StreamExt, TryStreamExt};
use serde::{Deserialize, Serialize};

use crate::proto::{DeliveryScope, TopicId};

/// Default channel capacity for topic subscription channels (one per topic)
const TOPIC_EVENTS_DEFAULT_CAP: usize = 2048;
/// Channel capacity for topic command send channels.
const TOPIC_COMMANDS_CAP: usize = 64;

/// Input messages for the gossip actor.
#[rpc_requests(message = RpcMessage, rpc_feature = "rpc")]
#[derive(Debug, Serialize, Deserialize)]
pub(crate) enum Request {
    #[rpc(tx=mpsc::Sender<Event>, rx=mpsc::Receiver<Command>)]
    Join(JoinRequest),
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct JoinRequest {
    pub topic_id: TopicId,
    pub bootstrap: BTreeSet<EndpointId>,
}

#[allow(missing_docs)]
#[stack_error(derive, add_meta, from_sources)]
#[non_exhaustive]
pub enum ApiError {
    #[error(transparent)]
    Rpc { source: irpc::Error },
    /// The gossip topic was closed.
    #[error("topic closed")]
    Closed,
}

impl From<irpc::channel::SendError> for ApiError {
    fn from(value: irpc::channel::SendError) -> Self {
        irpc::Error::from(value).into()
    }
}

impl From<irpc::channel::mpsc::RecvError> for ApiError {
    fn from(value: irpc::channel::mpsc::RecvError) -> Self {
        irpc::Error::from(value).into()
    }
}

impl From<irpc::channel::oneshot::RecvError> for ApiError {
    fn from(value: irpc::channel::oneshot::RecvError) -> Self {
        irpc::Error::from(value).into()
    }
}

/// API to control a [`Gossip`] instance.
///
/// This has methods to subscribe and join gossip topics, which return handles to publish
/// and receive messages on topics.
///
/// [`Gossip`] derefs to [`GossipApi`], so all functions on [`GossipApi`] are directly callable
/// from [`Gossip`].
///
/// [`Gossip`]: crate::net::Gossip
#[derive(Debug, Clone)]
pub struct GossipApi {
    client: Client<Request>,
}

impl GossipApi {
    #[cfg(feature = "net")]
    pub(crate) fn local(tx: tokio::sync::mpsc::Sender<RpcMessage>) -> Self {
        let local = irpc::LocalSender::<Request>::from(tx);
        Self {
            client: local.into(),
        }
    }

    // RPC methods (connect, listen) stripped — no `rpc` feature in tom-gossip.

    /// Join a gossip topic with options.
    ///
    /// Returns a [`GossipTopic`] instantly. To wait for at least one connection to be established,
    /// you can await [`GossipTopic::joined`].
    ///
    /// Messages will be queued until a first connection is available. If the internal channel becomes full,
    /// the oldest messages will be dropped from the channel.
    pub async fn subscribe_with_opts(
        &self,
        topic_id: TopicId,
        opts: JoinOptions,
    ) -> Result<GossipTopic, ApiError> {
        let req = JoinRequest {
            topic_id,
            bootstrap: opts.bootstrap,
        };
        let (tx, rx) = self
            .client
            .bidi_streaming(req, TOPIC_COMMANDS_CAP, opts.subscription_capacity)
            .await?;
        Ok(GossipTopic::new(tx, rx))
    }

    /// Join a gossip topic with the default options and wait for at least one active connection.
    pub async fn subscribe_and_join(
        &self,
        topic_id: TopicId,
        bootstrap: Vec<EndpointId>,
    ) -> Result<GossipTopic, ApiError> {
        let mut sub = self
            .subscribe_with_opts(topic_id, JoinOptions::with_bootstrap(bootstrap))
            .await?;
        sub.joined().await?;
        Ok(sub)
    }

    /// Join a gossip topic with the default options.
    ///
    /// Note that this will not wait for any bootstrap endpoint to be available.
    /// To ensure the topic is connected to at least one endpoint, use [`GossipTopic::joined`]
    /// or [`Self::subscribe_and_join`]
    pub async fn subscribe(
        &self,
        topic_id: TopicId,
        bootstrap: Vec<EndpointId>,
    ) -> Result<GossipTopic, ApiError> {
        let sub = self
            .subscribe_with_opts(topic_id, JoinOptions::with_bootstrap(bootstrap))
            .await?;

        Ok(sub)
    }
}

/// Sender for a gossip topic.
#[derive(Debug, Clone)]
pub struct GossipSender(mpsc::Sender<Command>);

impl GossipSender {
    pub(crate) fn new(sender: mpsc::Sender<Command>) -> Self {
        Self(sender)
    }

    /// Broadcasts a message to all endpoints.
    pub async fn broadcast(&self, message: Bytes) -> Result<(), ApiError> {
        self.send(Command::Broadcast(message)).await?;
        Ok(())
    }

    /// Broadcasts a message to our direct neighbors.
    pub async fn broadcast_neighbors(&self, message: Bytes) -> Result<(), ApiError> {
        self.send(Command::BroadcastNeighbors(message)).await?;
        Ok(())
    }

    /// Joins a set of peers.
    pub async fn join_peers(&self, peers: Vec<EndpointId>) -> Result<(), ApiError> {
        self.send(Command::JoinPeers(peers)).await?;
        Ok(())
    }

    async fn send(&self, command: Command) -> Result<(), irpc::channel::SendError> {
        self.0.send(command).await?;
        Ok(())
    }
}

/// Subscribed gossip topic.
///
/// This handle is a [`Stream`] of [`Event`]s from the topic, and can be used to send messages.
///
/// Once the [`GossipTopic`] is dropped, the network actor will leave the gossip topic.
///
/// It may be split into sender and receiver parts with [`Self::split`]. In this case, the topic will
/// be left once both the [`GossipSender`] and [`GossipReceiver`] halves are dropped.
#[derive(Debug)]
pub struct GossipTopic {
    sender: GossipSender,
    receiver: GossipReceiver,
}

impl GossipTopic {
    pub(crate) fn new(sender: mpsc::Sender<Command>, receiver: mpsc::Receiver<Event>) -> Self {
        let sender = GossipSender::new(sender);
        Self {
            sender,
            receiver: GossipReceiver::new(receiver),
        }
    }

    /// Splits `self` into [`GossipSender`] and [`GossipReceiver`] parts.
    pub fn split(self) -> (GossipSender, GossipReceiver) {
        (self.sender, self.receiver)
    }

    /// Sends a message to all peers.
    pub async fn broadcast(&mut self, message: Bytes) -> Result<(), ApiError> {
        self.sender.broadcast(message).await
    }

    /// Sends a message to our direct neighbors in the swarm.
    pub async fn broadcast_neighbors(&mut self, message: Bytes) -> Result<(), ApiError> {
        self.sender.broadcast_neighbors(message).await
    }

    /// Lists our current direct neighbors.
    pub fn neighbors(&self) -> impl Iterator<Item = EndpointId> + '_ {
        self.receiver.neighbors()
    }

    /// Waits until we are connected to at least one endpoint.
    ///
    /// See [`GossipReceiver::joined`] for details.
    pub async fn joined(&mut self) -> Result<(), ApiError> {
        self.receiver.joined().await
    }

    /// Returns `true` if we are connected to at least one endpoint.
    pub fn is_joined(&self) -> bool {
        self.receiver.is_joined()
    }
}

impl Stream for GossipTopic {
    type Item = Result<Event, ApiError>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.receiver).poll_next(cx)
    }
}

/// Receiver for gossip events on a topic.
///
/// This is a [`Stream`] of [`Event`]s emitted from the topic.
#[derive(derive_more::Debug)]
pub struct GossipReceiver {
    #[debug("BoxStream")]
    stream: Pin<Box<dyn Stream<Item = Result<Event, ApiError>> + Send + Sync + 'static>>,
    neighbors: HashSet<EndpointId>,
}

impl GossipReceiver {
    pub(crate) fn new(events_rx: mpsc::Receiver<Event>) -> Self {
        let stream = events_rx.into_stream().map_err(ApiError::from);
        let stream = Box::pin(stream);
        Self {
            stream,
            neighbors: Default::default(),
        }
    }

    /// Lists our current direct neighbors.
    pub fn neighbors(&self) -> impl Iterator<Item = EndpointId> + '_ {
        self.neighbors.iter().copied()
    }

    /// Waits until we are connected to at least one endpoint.
    ///
    /// Progresses the event stream to the first [`Event::NeighborUp`] event.
    ///
    /// Note that this consumes this initial `NeighborUp` event. If you want to track
    /// neighbors, use [`Self::neighbors`] after awaiting [`Self::joined`], and then
    /// continue to track `NeighborUp` events on the event stream.
    pub async fn joined(&mut self) -> Result<(), ApiError> {
        while !self.is_joined() {
            let _event = self.next().await.ok_or(e!(ApiError::Closed))??;
        }
        Ok(())
    }

    /// Returns `true` if we are connected to at least one endpoint.
    pub fn is_joined(&self) -> bool {
        !self.neighbors.is_empty()
    }
}

impl Stream for GossipReceiver {
    type Item = Result<Event, ApiError>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let item = std::task::ready!(Pin::new(&mut self.stream).poll_next(cx));
        if let Some(Ok(item)) = &item {
            match item {
                Event::NeighborUp(endpoint_id) => {
                    self.neighbors.insert(*endpoint_id);
                }
                Event::NeighborDown(endpoint_id) => {
                    self.neighbors.remove(endpoint_id);
                }
                _ => {}
            }
        }
        Poll::Ready(item)
    }
}

/// Events emitted from a gossip topic.
///
/// These are the events emitted from a [`GossipReceiver`].
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Serialize, Deserialize)]
pub enum Event {
    /// We have a new, direct neighbor in the swarm membership layer for this topic.
    NeighborUp(EndpointId),
    /// We dropped direct neighbor in the swarm membership layer for this topic.
    NeighborDown(EndpointId),
    /// We received a gossip message for this topic.
    Received(Message),
    /// We missed some messages because our [`GossipReceiver`] was not progressing fast enough.
    Lagged,
}

impl From<crate::proto::Event<EndpointId>> for Event {
    fn from(event: crate::proto::Event<EndpointId>) -> Self {
        match event {
            crate::proto::Event::NeighborUp(endpoint_id) => Self::NeighborUp(endpoint_id),
            crate::proto::Event::NeighborDown(endpoint_id) => Self::NeighborDown(endpoint_id),
            crate::proto::Event::Received(message) => Self::Received(Message {
                content: message.content,
                scope: message.scope,
                delivered_from: message.delivered_from,
            }),
        }
    }
}

/// A gossip message
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, derive_more::Debug, Serialize, Deserialize)]
pub struct Message {
    /// The content of the message
    #[debug("Bytes({})", self.content.len())]
    pub content: Bytes,
    /// The scope of the message.
    /// This tells us if the message is from a direct neighbor or actual gossip.
    pub scope: DeliveryScope,
    /// The endpoint that delivered the message. This is not the same as the original author.
    pub delivered_from: EndpointId,
}

/// Command for a gossip topic.
#[derive(Serialize, Deserialize, derive_more::Debug, Clone)]
pub enum Command {
    /// Broadcasts a message to all endpoints in the swarm.
    Broadcast(#[debug("Bytes({})", _0.len())] Bytes),
    /// Broadcasts a message to all direct neighbors.
    BroadcastNeighbors(#[debug("Bytes({})", _0.len())] Bytes),
    /// Connects to a set of peers.
    JoinPeers(Vec<EndpointId>),
}

/// Options for joining a gossip topic.
#[derive(Serialize, Deserialize, Debug)]
pub struct JoinOptions {
    /// The initial bootstrap endpoints.
    pub bootstrap: BTreeSet<EndpointId>,
    /// The maximum number of messages that can be buffered in a subscription.
    ///
    /// If this limit is reached, the subscriber will receive a `Lagged` response,
    /// the message will be dropped, and the subscriber will be closed.
    ///
    /// This is to prevent a single slow subscriber from blocking the dispatch loop.
    /// If a subscriber is lagging, it should be closed and re-opened.
    pub subscription_capacity: usize,
}

impl JoinOptions {
    /// Creates [`JoinOptions`] with the provided bootstrap endpoints and the default subscription
    /// capacity.
    pub fn with_bootstrap(endpoints: impl IntoIterator<Item = EndpointId>) -> Self {
        Self {
            bootstrap: endpoints.into_iter().collect(),
            subscription_capacity: TOPIC_EVENTS_DEFAULT_CAP,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::api::GossipTopic;

    // test_rpc stripped — depends on iroh::test_utils (relay server setup).
    // Will be restored when tom-connect has its own test utilities.

    #[test]
    fn ensure_gossip_topic_is_sync() {
        #[allow(unused)]
        fn get() -> GossipTopic {
            unimplemented!()
        }
        #[allow(unused)]
        fn check(_t: impl Sync) {}
        #[allow(unused)]
        fn foo() {
            check(get());
        }
    }
}
