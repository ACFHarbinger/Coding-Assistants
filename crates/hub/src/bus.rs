//! In-process event bus (`platform.md` P1, #325).
//!
//! Backend code publishes through [`EventSink`] / [`EventBus`] instead of
//! `tauri::AppHandle`. Multiple subscribers receive the same event: the
//! desktop Tauri emitter is one of them, not the bus itself.
//!
//! Bounded and non-blocking: [`InProcessBus::publish`] never waits on a
//! subscriber. A lagging consumer whose buffer is full drops the **new**
//! event (`try_send` `Full`); a dropped receiver is pruned.
//!
//! ## `app.emit` inventory (this slice)
//!
//! | Site | Topic | Slice |
//! | --- | --- | --- |
//! | `agent/orchestrator.rs`, `client/llm.rs` | `agent-event` | migrated |
//! | `agent/orchestrator.rs` | `agent-memory-recall` | migrated |
//! | `commands/hub/store.rs` | `hub:agents-changed` | migrated |
//! | `pty.rs` | `pty-output:{id}`, `pty-exit:{id}` | deferred (session-scoped GUI bytes) |
//! | `server/tcp_server.rs` | `android-*` | deferred (inbound-to-GUI, not fan-out) |

use serde::Serialize;
use serde_json::Value;
use std::sync::mpsc::{self, Receiver, RecvError, SyncSender, TryRecvError, TrySendError};
use std::sync::{Arc, Mutex};

/// Default per-subscriber buffer. Matches a typical `broadcast` capacity.
pub const DEFAULT_CAPACITY: usize = 256;

pub const TOPIC_AGENT_EVENT: &str = "agent-event";
pub const TOPIC_AGENT_MEMORY_RECALL: &str = "agent-memory-recall";
pub const TOPIC_AGENTS_CHANGED: &str = "hub:agents-changed";

/// A named backend event. Topic strings match today's Tauri event names so
/// the desktop subscriber can forward 1:1.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct BusEvent {
    pub topic: String,
    pub payload: Value,
}

impl BusEvent {
    pub fn new(topic: impl Into<String>, payload: Value) -> Self {
        Self {
            topic: topic.into(),
            payload,
        }
    }

    pub fn json(topic: impl Into<String>, payload: impl Serialize) -> Self {
        Self::new(topic, serde_json::to_value(payload).unwrap_or(Value::Null))
    }

    pub fn empty(topic: impl Into<String>) -> Self {
        Self::new(topic, Value::Null)
    }
}

/// Publish side. Call sites depend on this, not on `tauri::AppHandle`.
pub trait EventSink: Send + Sync {
    fn publish(&self, event: BusEvent);
}

/// Subscribe side. Multiple in-process consumers (Tauri GUI, TCP, tests).
pub trait EventBus: EventSink {
    fn subscribe(&self) -> BusReceiver;
}

/// One subscriber's receive end. Blocking [`BusReceiver::recv`] is for
/// dedicated forwarder threads; tests use [`BusReceiver::try_recv`].
pub struct BusReceiver {
    rx: Receiver<BusEvent>,
}

impl BusReceiver {
    pub fn recv(&self) -> Result<BusEvent, RecvError> {
        self.rx.recv()
    }

    pub fn try_recv(&self) -> Result<BusEvent, TryRecvError> {
        self.rx.try_recv()
    }
}

/// Bounded in-process fan-out. Cloning shares the subscriber list.
#[derive(Clone)]
pub struct InProcessBus {
    inner: Arc<Mutex<Vec<SyncSender<BusEvent>>>>,
    capacity: usize,
}

impl Default for InProcessBus {
    fn default() -> Self {
        Self::new()
    }
}

impl InProcessBus {
    pub fn new() -> Self {
        Self::with_capacity(DEFAULT_CAPACITY)
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            inner: Arc::new(Mutex::new(Vec::new())),
            capacity: capacity.max(1),
        }
    }

    /// Convenience for call sites that already have a serializable payload.
    pub fn emit(&self, topic: impl Into<String>, payload: impl Serialize) {
        self.publish(BusEvent::json(topic, payload));
    }

    fn lock_subs(&self) -> std::sync::MutexGuard<'_, Vec<SyncSender<BusEvent>>> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }

    #[cfg(test)]
    fn subscriber_count(&self) -> usize {
        self.lock_subs().len()
    }
}

impl EventSink for InProcessBus {
    fn publish(&self, event: BusEvent) {
        let mut subs = self.lock_subs();
        subs.retain(|tx| match tx.try_send(event.clone()) {
            Ok(()) => true,
            Err(TrySendError::Full(_)) => true,
            Err(TrySendError::Disconnected(_)) => false,
        });
    }
}

impl EventBus for InProcessBus {
    fn subscribe(&self) -> BusReceiver {
        let (tx, rx) = mpsc::sync_channel(self.capacity);
        self.lock_subs().push(tx);
        BusReceiver { rx }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> BusEvent {
        BusEvent::json(
            TOPIC_AGENT_EVENT,
            serde_json::json!({
                "source": "Planner",
                "event_type": "thought",
                "content": "hi",
            }),
        )
    }

    #[test]
    fn two_subscribers_receive_the_same_event() {
        let bus = InProcessBus::new();
        let a = bus.subscribe();
        let b = bus.subscribe();
        bus.publish(sample());
        let ea = a.try_recv().expect("first subscriber");
        let eb = b.try_recv().expect("second subscriber");
        assert_eq!(ea, eb);
        assert_eq!(ea.topic, TOPIC_AGENT_EVENT);
        assert_eq!(ea.payload["content"], "hi");
        assert!(a.try_recv().is_err());
        assert!(b.try_recv().is_err());
    }

    #[test]
    fn publisher_does_not_block_when_a_subscriber_lags() {
        let bus = InProcessBus::with_capacity(1);
        let rx = bus.subscribe();
        bus.emit("t", 1);
        bus.emit("t", 2);
        let got = rx.try_recv().expect("queued first event");
        assert_eq!(got.payload, serde_json::json!(1));
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn disconnected_subscriber_is_pruned() {
        let bus = InProcessBus::new();
        {
            let _rx = bus.subscribe();
            assert_eq!(bus.subscriber_count(), 1);
        }
        bus.emit("t", ());
        assert_eq!(bus.subscriber_count(), 0);
        let rx = bus.subscribe();
        bus.emit("t", "ok");
        assert_eq!(rx.try_recv().unwrap().payload, serde_json::json!("ok"));
    }

    #[test]
    fn empty_agents_changed_payload_is_null() {
        let bus = InProcessBus::new();
        let rx = bus.subscribe();
        bus.publish(BusEvent::empty(TOPIC_AGENTS_CHANGED));
        let event = rx.try_recv().unwrap();
        assert_eq!(event.topic, TOPIC_AGENTS_CHANGED);
        assert!(event.payload.is_null());
    }
}
