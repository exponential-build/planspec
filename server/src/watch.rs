use std::sync::Arc;
use tokio::sync::broadcast;

use crate::storage::WatchEvent;

const CHANNEL_CAPACITY: usize = 1024;

/// Broadcasts watch events to subscribers
#[derive(Clone)]
pub struct WatchBroadcaster {
    sender: Arc<broadcast::Sender<WatchEvent>>,
}

impl WatchBroadcaster {
    pub fn new() -> Self {
        let (sender, _) = broadcast::channel(CHANNEL_CAPACITY);
        Self {
            sender: Arc::new(sender),
        }
    }

    /// Send an event to all subscribers
    pub fn send(&self, event: WatchEvent) {
        // Ignore send errors (no subscribers)
        let _ = self.sender.send(event);
    }

    /// Subscribe to events
    pub fn subscribe(&self) -> broadcast::Receiver<WatchEvent> {
        self.sender.subscribe()
    }
}

impl Default for WatchBroadcaster {
    fn default() -> Self {
        Self::new()
    }
}
