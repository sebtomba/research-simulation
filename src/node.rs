use crate::sim::{repeat_process, Effect, Process, ProcessId};
use std::time::Duration;
use tokio::sync::RwLock;

pub trait NodeEffect {
    type Item;
    fn delay(duration: Duration) -> Self::Item;
    fn wait() -> Self::Item;
}

// TODO: Make the methods async
pub trait Node<T, M>
where
    T: NodeEffect + Into<Effect>,
{
    fn init(&mut self, id: ProcessId);

    fn receive(&self, sender: ProcessId, message: M) -> Box<dyn Process<T>> {
        self.handle(sender, message)
    }

    fn handle(&self, sender: ProcessId, message: M) -> Box<dyn Process<T>>;

    fn run(&self) -> Box<dyn Process<T>>;
}

pub trait MessageHandler<T, M>
where
    T: NodeEffect + Into<Effect>,
{
    fn handle(&self, node_id: ProcessId, sender: ProcessId, message: M) -> Box<dyn Process<T>>;
}

/// A node that processes messages concurrently.
/// All messages are handled by the message handler.
//
//  Use this node if all messages are to be processed
//  concurrently without blocking. If messages are to be processed
//  sequentially, it may be easier to use the `SequentialNode`.
pub struct ConcurrentNode<H> {
    id: Option<ProcessId>,
    handler: H,
}

impl<T, M, H> Node<T, M> for ConcurrentNode<H>
where
    T: NodeEffect<Item = T> + Into<Effect> + Clone + 'static,
    H: MessageHandler<T, M>,
{
    fn init(&mut self, id: ProcessId) {
        self.id = Some(id);
    }

    fn receive(&self, _sender: ProcessId, _message: M) -> Box<dyn Process<T>> {
        todo!()
    }

    fn handle(&self, sender: ProcessId, message: M) -> Box<dyn Process<T>> {
        self.handler
            .handle(self.id.expect("after node init"), sender, message)
    }

    fn run(&self) -> Box<dyn Process<T>> {
        Box::new(repeat_process(T::wait()))
    }
}

pub struct SequentialNode<H, M> {
    id: Option<ProcessId>,
    handler: H,
    mailbox: RwLock<Vec<(ProcessId, M)>>,
}

impl<T, M, H> Node<T, M> for SequentialNode<H, M>
where
    T: NodeEffect<Item = T> + Into<Effect> + Clone + 'static,
    H: MessageHandler<T, M>,
{
    fn init(&mut self, id: ProcessId) {
        self.id = Some(id);
    }

    fn receive(&self, _sender: ProcessId, _message: M) -> Box<dyn Process<T>> {
        // TODO: Add message to the mailbox
        // This method must be async to use the RwLock
        todo!()
    }

    fn handle(&self, sender: ProcessId, message: M) -> Box<dyn Process<T>> {
        self.handler
            .handle(self.id.expect("after node init"), sender, message)
    }

    fn run(&self) -> Box<dyn Process<T>> {
        // TODO: Handle all messages from the mailbox
        // This method must be async to use the RwLock
        todo!()
    }
}
