use tokio::sync::mpsc::UnboundedSender;
use crate::scheduler::Tick;

#[derive(Clone)]
pub enum Event {
    TickExecuted { tick: Tick },
    RequestFinished { ok: bool, latency_ms: u32 },
    InFlight { value: i32 },
    RunFinished,
}

#[derive(Clone)]
pub struct EventSink<E> {
    tx: Option<UnboundedSender<E>>,
}

impl<E> EventSink<E> {
    /// No-op sink
    pub fn noop() -> Self {
        Self { tx: None }
    }

    /// Real sink
    pub fn new(tx: UnboundedSender<E>) -> Self {
        Self { tx: Some(tx) }
    }

    /// Best-effort send
    #[inline]
    pub fn send(&self, ev: E) {
        if let Some(tx) = &self.tx {
            let _ = tx.send(ev);
        }
    }

    #[inline]
    pub fn is_enabled(&self) -> bool {
        self.tx.is_some()
    }
}