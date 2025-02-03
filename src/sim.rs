use futures::{Stream, StreamExt};
use std::{
    cmp::{Ordering, Reverse},
    collections::{BinaryHeap, HashMap},
    time::Duration,
};
use tokio::sync::RwLock;

#[derive(Debug, Copy, Clone)]
pub enum Effect {
    Wait,
    Delay(Duration),
    Wakeup { time: Duration, process: ProcessId },
}

#[derive(Debug, Copy, Clone)]
pub struct Event {
    /// Time interval between the current simulation time and the event schedule
    time: Duration,
    /// Process to execute when the event occur
    process: ProcessId,
}

impl Event {
    pub fn new(time: Duration, process: ProcessId) -> Self {
        Self { time, process }
    }

    pub fn time(&self) -> Duration {
        self.time
    }

    pub fn process(&self) -> ProcessId {
        self.process
    }
}

impl PartialOrd for Event {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.time.partial_cmp(&other.time)
    }
}

impl Ord for Event {
    fn cmp(&self, other: &Self) -> Ordering {
        self.time.cmp(&other.time)
    }
}

impl PartialEq for Event {
    fn eq(&self, other: &Self) -> bool {
        self.time == other.time
    }
}

impl Eq for Event {}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct ProcessId(usize);

pub trait Process<T: Into<Effect>>: Stream<Item = T> + Unpin {}

pub fn empty_process<T: Into<Effect>>() -> impl Process<T> {
    futures::stream::empty::<T>()
}

pub fn repeat_process<T: Into<Effect> + Clone>(item: T) -> impl Process<T> {
    futures::stream::repeat(item)
}

impl<S: Stream<Item = T> + Unpin, T: Into<Effect>> Process<T> for S {}

struct SimulationState<T> {
    time: Duration,
    steps: usize,
    process_id: usize,
    processes: HashMap<ProcessId, Box<dyn Process<T>>>,
    future_events: BinaryHeap<Reverse<Event>>,
}

impl<T: Into<Effect>> SimulationState<T> {
    pub fn time(&self) -> Duration {
        self.time
    }

    pub fn add_process(&mut self, process: Box<dyn Process<T>>) -> ProcessId {
        self.process_id += 1;
        let id = ProcessId(self.process_id);
        self.processes.insert(id, process);
        id
    }

    pub fn wakeup_process(&mut self, time: Duration, process: ProcessId) {
        self.future_events.push(Reverse(Event::new(time, process)));
    }
}

pub struct Simulation<T>(RwLock<SimulationState<T>>);

impl<T: Into<Effect>> Simulation<T> {
    /// Create a new `Simulation` environment.
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn add_process(&self, process: Box<dyn Process<T>>) -> ProcessId {
        self.0.write().await.add_process(process)
    }

    pub async fn wakeup_process(&self, time: Duration, process: ProcessId) {
        self.0.write().await.wakeup_process(time, process);
    }

    /// Returns the current simulation time
    pub async fn time(&self) -> Duration {
        self.0.read().await.time()
    }

    pub async fn step(&self) -> Option<Event> {
        self.update_step().await;

        let event = self.next_event().await;

        if let Some(event) = event {
            if let Some(result) = self.resume_process(&event.process).await {
                match result.into() {
                    Effect::Wait => {}
                    Effect::Delay(d) => {
                        let time = self.time().await + d;
                        self.wakeup_process(time, event.process).await;
                    }
                    Effect::Wakeup { time, process } => {
                        let time = self.time().await + time;
                        self.wakeup_process(time, process).await;
                    }
                }
            } else {
                self.remove_process(&event.process).await;
            }
        }

        event
    }

    async fn update_step(&self) {
        let mut state = self.0.write().await;
        state.steps += 1;
    }

    async fn next_event(&self) -> Option<Event> {
        let mut state = self.0.write().await;
        if let Some(Reverse(event)) = state.future_events.pop() {
            state.time = event.time();
            Some(event)
        } else {
            None
        }
    }

    async fn resume_process(&self, process_id: &ProcessId) -> Option<T> {
        let mut state = self.0.write().await;
        let process = state
            .processes
            .get_mut(process_id)
            .expect("Tried to resume a completed process");

        process.next().await
    }

    async fn remove_process(&self, process_id: &ProcessId) {
        let mut state = self.0.write().await;
        state.processes.remove(process_id);
    }
}

impl<T> Default for Simulation<T> {
    fn default() -> Self {
        let state = SimulationState {
            time: Duration::ZERO,
            steps: 0,
            process_id: 0,
            processes: HashMap::new(),
            future_events: BinaryHeap::new(),
        };

        Self(RwLock::new(state))
    }
}
