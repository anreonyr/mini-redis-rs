use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};
use tokio::sync::mpsc::UnboundedSender;

/// A pub/sub message pushed to all subscribers of a channel.
#[derive(Clone, Debug)]
pub struct Message {
    pub channel: String,
    pub payload: String,
}

static CHANNELS: LazyLock<Mutex<HashMap<String, Vec<UnboundedSender<Message>>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// Register a sender for the given channels. Returns the number of channels subscribed.
pub fn subscribe(sender: UnboundedSender<Message>, channels: &[String]) -> usize {
    let mut map = CHANNELS.lock().unwrap();
    for channel in channels {
        map.entry(channel.clone()).or_default().push(sender.clone());
    }
    channels.len()
}

/// Unregister a sender from the given channels. If channels is empty, unregister from all.
pub fn unsubscribe(sender: &UnboundedSender<Message>, channels: &[String]) -> usize {
    let mut map = CHANNELS.lock().unwrap();
    if channels.is_empty() {
        let count = map.values().map(|v| v.len()).sum();
        map.clear();
        return count;
    }
    let mut count = 0;
    for channel in channels {
        if let Some(senders) = map.get_mut(channel) {
            let before = senders.len();
            senders.retain(|s| !s.same_channel(sender));
            count += before - senders.len();
            if senders.is_empty() {
                map.remove(channel);
            }
        }
    }
    count
}

/// Publish a message to all subscribers of a channel. Returns the number of recipients.
pub fn publish(channel: &str, payload: &str) -> usize {
    let mut map = CHANNELS.lock().unwrap();
    let Some(senders) = map.get_mut(channel) else {
        return 0;
    };
    let msg = Message {
        channel: channel.to_string(),
        payload: payload.to_string(),
    };
    // Prune dead senders while delivering
    senders.retain(|sender| sender.send(msg.clone()).is_ok());
    let count = senders.len();
    if senders.is_empty() {
        map.remove(channel);
    }
    count
}
