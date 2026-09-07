//! Main-world message adapters for the independently owned network world.
//! Incoming batches contain application data only, not transport metadata.

use std::marker::PhantomData;
use std::sync::mpsc::{SendError, Sender};

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use lightyear::prelude::{Channel, Message as NetworkMessage, MessageSender as TransportSender};

use super::worker::{MainUpdate, NetworkCommand};

/// Application-owned FIFO, initialized by message registration in the main world.
#[derive(Resource)]
pub struct Inbox<M: NetworkMessage> {
    messages: Vec<M>,
}

impl<M: NetworkMessage> Default for Inbox<M> {
    fn default() -> Self {
        Self::new(Vec::new())
    }
}

impl<M: NetworkMessage> Inbox<M> {
    pub fn new(messages: Vec<M>) -> Self {
        Self { messages }
    }

    pub fn receive(&mut self) -> std::vec::Drain<'_, M> {
        self.messages.drain(..)
    }

    pub fn has_messages(&self) -> bool {
        !self.messages.is_empty()
    }

    pub fn num_messages(&self) -> usize {
        self.messages.len()
    }
}

/// Connection lifecycle inserts/removes the worker sender to enable outgoing work.
#[derive(Resource, Default)]
pub struct ConnectionSender {
    pub sender: Option<Sender<NetworkCommand>>,
}

impl ConnectionSender {
    pub fn new(sender: Option<Sender<NetworkCommand>>) -> Self {
        Self { sender }
    }
}

/// Query-compatible view over the single registered application inbox.
#[derive(SystemParam)]
pub struct MessageReceivers<'w, 's, M: NetworkMessage> {
    inbox: ResMut<'w, Inbox<M>>,
    marker: PhantomData<&'s M>,
}

impl<M: NetworkMessage> MessageReceivers<'_, '_, M> {
    /// The adapter always contains its registered inbox, including when drained.
    pub fn is_empty(&self) -> bool {
        false
    }

    pub fn iter_mut(&mut self) -> std::iter::Once<&mut Inbox<M>> {
        std::iter::once(&mut *self.inbox)
    }
}

/// Yields one sending endpoint while connected, otherwise none.
#[derive(SystemParam)]
pub struct MessageSenders<'w, 's, M: NetworkMessage> {
    connection: Res<'w, ConnectionSender>,
    marker: PhantomData<&'s M>,
}

impl<M: NetworkMessage> MessageSenders<'_, '_, M> {
    pub fn is_empty(&self) -> bool {
        self.connection.sender.is_none()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = WorkerMessageSender<'_, M>> {
        self.connection
            .sender
            .as_ref()
            .map(|commands| WorkerMessageSender {
                commands,
                marker: PhantomData,
            })
            .into_iter()
    }
}

/// Transfers owned messages to the worker without requiring `M: Clone`.
pub struct WorkerMessageSender<'a, M: NetworkMessage> {
    commands: &'a Sender<NetworkCommand>,
    marker: PhantomData<M>,
}

impl<M: NetworkMessage> WorkerMessageSender<'_, M> {
    /// A closed worker is fatal, rather than silently dropping a queued request.
    pub fn send<C: Channel>(&mut self, message: M) {
        let command = NetworkCommand::Apply(Box::new(move |world| {
            send_in_worker::<M, C>(world, message);
        }));
        self.commands.send(command).unwrap_or_else(|_| {
            panic!(
                "network worker command channel closed while sending {}",
                std::any::type_name::<M>()
            );
        });
    }
}

fn send_in_worker<M: NetworkMessage, C: Channel>(world: &mut World, message: M) {
    let mut query = world.query::<&mut TransportSender<M>>();
    let mut sender = query.single_mut(world).unwrap_or_else(|error| {
        panic!(
            "expected one worker sender for {}: {error}",
            std::any::type_name::<M>()
        );
    });
    sender.send::<C>(message);
}

/// Queue a received batch for append after previously published batches.
/// Transport tick/channel metadata is intentionally not part of this data-only API.
pub fn publish_incoming<M: NetworkMessage>(
    updates: &Sender<MainUpdate>,
    messages: Vec<M>,
) -> Result<(), SendError<MainUpdate>> {
    if messages.is_empty() {
        return Ok(());
    }
    updates.send(Box::new(move |world| {
        world.resource_mut::<Inbox<M>>().messages.extend(messages);
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::system::SystemState;

    #[derive(serde::Serialize, serde::Deserialize)]
    struct Number(u32);
    struct TestChannel;

    #[test]
    fn inbox_drains_messages_in_fifo_order() {
        let mut inbox = Inbox::new(vec![Number(3), Number(1), Number(4)]);
        assert!(inbox.has_messages());
        assert_eq!(inbox.num_messages(), 3);
        assert_eq!(
            inbox.receive().map(|message| message.0).collect::<Vec<_>>(),
            [3, 1, 4]
        );
        assert!(!inbox.has_messages());
        assert_eq!(inbox.num_messages(), 0);
    }

    #[test]
    fn receiver_adapter_yields_one_inbox_even_when_empty() {
        let mut world = World::new();
        world.init_resource::<Inbox<Number>>();
        let mut state = SystemState::<MessageReceivers<Number>>::new(&mut world);
        let mut receivers = state.get_mut(&mut world);
        assert!(!receivers.is_empty());
        let mut inboxes = receivers.iter_mut();
        assert!(!inboxes.next().unwrap().has_messages());
        assert!(inboxes.next().is_none());
    }

    #[test]
    fn disconnected_adapter_yields_no_sender() {
        let mut world = World::new();
        world.init_resource::<ConnectionSender>();
        let mut state = SystemState::<MessageSenders<Number>>::new(&mut world);
        let mut senders = state.get_mut(&mut world);
        assert!(senders.is_empty());
        assert!(senders.iter_mut().next().is_none());
    }

    #[test]
    #[should_panic(expected = "network worker command channel closed")]
    fn sending_to_a_closed_worker_fails_explicitly() {
        let (sender, receiver) = std::sync::mpsc::channel();
        drop(receiver);
        let mut world = World::new();
        world.insert_resource(ConnectionSender::new(Some(sender)));
        let mut state = SystemState::<MessageSenders<Number>>::new(&mut world);
        let mut senders = state.get_mut(&mut world);
        assert!(!senders.is_empty());
        senders
            .iter_mut()
            .next()
            .unwrap()
            .send::<TestChannel>(Number(9));
    }

    #[test]
    fn published_batches_append_after_existing_messages_in_fifo_order() {
        let (sender, receiver) = std::sync::mpsc::channel();
        let mut world = World::new();
        world.insert_resource(Inbox::new(vec![Number(1)]));
        publish_incoming(&sender, vec![Number(2), Number(3)]).unwrap();
        publish_incoming(&sender, vec![Number(4)]).unwrap();
        assert_eq!(world.resource::<Inbox<Number>>().num_messages(), 1);
        receiver.recv().unwrap()(&mut world);
        receiver.recv().unwrap()(&mut world);
        let values: Vec<_> = world
            .resource_mut::<Inbox<Number>>()
            .receive()
            .map(|message| message.0)
            .collect();
        assert_eq!(values, [1, 2, 3, 4]);
    }

    #[test]
    fn connected_adapter_delivers_owned_non_clone_messages_through_transport() {
        use lightyear::prelude::{Link, MessageReceiver as TransportReceiver};
        let (mut worker, peer) = transport_fixture();
        let (commands, receiver) = std::sync::mpsc::channel();
        let mut main = World::new();
        main.insert_resource(ConnectionSender::new(Some(commands)));
        let mut state = SystemState::<MessageSenders<Number>>::new(&mut main);
        {
            let mut senders = state.get_mut(&mut main);
            let mut iter = senders.iter_mut();
            let mut sender = iter.next().unwrap();
            assert!(iter.next().is_none());
            sender.send::<TestChannel>(Number(8));
            sender.send::<TestChannel>(Number(5));
        }
        for _ in 0..2 {
            let NetworkCommand::Apply(command) = receiver.recv().unwrap() else {
                panic!("expected a queued message, not worker shutdown");
            };
            command(worker.world_mut());
        }
        worker.world_mut().run_schedule(PostUpdate);
        {
            let mut entity = worker.world_mut().entity_mut(peer);
            let mut link = entity.get_mut::<Link>().unwrap();
            let packets: Vec<_> = link.send.drain().collect();
            assert!(!packets.is_empty(), "expected actual transport packets");
            for packet in packets {
                link.recv.push_raw(packet);
            }
        }
        worker.world_mut().run_schedule(PreUpdate);
        let messages: Vec<_> = worker
            .world_mut()
            .entity_mut(peer)
            .get_mut::<TransportReceiver<Number>>()
            .unwrap()
            .receive()
            .map(|message| message.0)
            .collect();
        assert_eq!(messages, [8, 5]);
    }

    fn transport_fixture() -> (App, Entity) {
        use lightyear::prelude::client::ClientPlugins;
        use lightyear::prelude::{
            AppChannelExt, AppMessageExt, ChannelMode, ChannelRegistry, ChannelSettings, Connected,
            Link, Linked, MessageReceiver as TransportReceiver, PeerId, RemoteId, Transport,
        };
        let mut app = App::new();
        app.add_plugins(bevy::state::app::StatesPlugin);
        app.add_plugins(ClientPlugins::default());
        app.register_message::<Number>();
        app.add_channel::<TestChannel>(ChannelSettings {
            mode: ChannelMode::OrderedReliable(Default::default()),
            ..Default::default()
        });
        app.finish();
        app.cleanup();
        let registry = app.world().resource::<ChannelRegistry>();
        let mut transport = Transport::default();
        transport.add_sender_from_registry::<TestChannel>(registry);
        transport.add_receiver_from_registry::<TestChannel>(registry);
        let peer = app
            .world_mut()
            .spawn((
                Link::default(),
                transport,
                Linked,
                Connected,
                RemoteId(PeerId::Local(0)),
                TransportSender::<Number>::default(),
                TransportReceiver::<Number>::default(),
            ))
            .id();
        (app, peer)
    }

    #[test]
    fn publishing_to_a_closed_main_channel_returns_an_error() {
        let (sender, receiver) = std::sync::mpsc::channel();
        drop(receiver);
        assert!(publish_incoming(&sender, vec![Number(9)]).is_err());
    }
}
