use crate::{
    node::{Node, NodeEffect},
    sim::{empty_process, Effect, Process, ProcessId, Simulation},
};
use futures::StreamExt;
use std::{collections::HashMap, time::Duration};
use tokio::sync::RwLock;

struct NetworkState<T: NodeEffect<Item = T>, M> {
    simulation: Simulation<T>,
    nodes: HashMap<ProcessId, Box<dyn Node<T, M>>>,
    delay: Duration,
}

pub struct Network<T: NodeEffect<Item = T>, M>(RwLock<NetworkState<T, M>>);

impl<T, M: Clone> Network<T, M>
where
    T: Into<Effect> + NodeEffect<Item = T> + 'static,
{
    pub fn new(delay: Duration) -> Self {
        let state = NetworkState {
            simulation: Simulation::default(),
            nodes: HashMap::new(),
            delay,
        };
        Self(RwLock::new(state))
    }

    pub async fn num_nodes(&self) -> usize {
        self.0.read().await.nodes.len()
    }

    pub async fn add_node(&self, node: impl Node<T, M> + 'static, start_delay: Duration) {
        let mut state = self.0.write().await;
        let pid = state.simulation.add_process(node.run()).await;
        state.simulation.wakeup_process(start_delay, pid).await;
        state.nodes.insert(pid, Box::new(node));
    }

    pub async fn send(
        &self,
        sender: ProcessId,
        target: ProcessId,
        message: M,
        delay: Option<Duration>,
    ) {
        let state = self.0.read().await;
        let delay = delay.unwrap_or(state.delay);
        let process = self.convey(sender, target, message, delay).await;
        let pid = state.simulation.add_process(process).await;
        state.simulation.wakeup_process(Duration::ZERO, pid).await;
    }

    pub async fn broadcast(&self, sender: ProcessId, message: M, delay: Option<Duration>) {
        let state = self.0.read().await;
        let delay = delay.unwrap_or(state.delay);
        for target in state.nodes.keys() {
            let process = self.convey(sender, *target, message.clone(), delay).await;
            let pid = state.simulation.add_process(process).await;
            state.simulation.wakeup_process(Duration::ZERO, pid).await;
        }
    }

    async fn convey(
        &self,
        sender: ProcessId,
        target: ProcessId,
        message: M,
        delay: Duration,
    ) -> Box<dyn Process<T>> {
        let state = self.0.read().await;
        if let Some(node) = state.nodes.get(&target) {
            Box::new(Self::delay(delay).chain(node.receive(sender, message)))
        } else {
            // TODO: log or panic on missing nodes
            Self::skip()
        }
    }

    pub fn delay(duration: Duration) -> Box<dyn Process<T>> {
        Box::new(futures::stream::iter(vec![T::delay(duration)]))
    }

    pub fn skip() -> Box<dyn Process<T>> {
        Box::new(empty_process())
    }
}
