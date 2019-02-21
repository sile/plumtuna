use crate::study::Message;
use std::mem;
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct SubscribeId(u32);
impl SubscribeId {
    pub fn new() -> Self {
        SubscribeId(0)
    }

    pub fn next(&mut self) -> SubscribeId {
        let x = SubscribeId(self.0);
        self.0 += 1;
        x
    }
}
impl From<u32> for SubscribeId {
    fn from(f: u32) -> Self {
        Self(f)
    }
}

#[derive(Debug)]
pub struct Subscriber {
    expiry_time: Duration,
    messages: Vec<Message>,
}
impl Subscriber {
    pub fn new(now: Duration) -> Self {
        Subscriber {
            messages: Vec::new(),
            expiry_time: now + Duration::from_secs(60 * 60), // TODO
        }
    }

    pub fn push_message(&mut self, message: Message) {
        self.messages.push(message);
    }

    pub fn pop_messages(&mut self) -> Vec<Message> {
        mem::replace(&mut self.messages, Vec::new())
    }

    pub fn heartbeat(&mut self, now: Duration) {
        self.expiry_time = now + Duration::from_secs(60 * 60); // TODO
    }

    pub fn has_expired(&self, now: Duration) -> bool {
        self.expiry_time < now
    }
}
