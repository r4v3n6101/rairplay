use tokio::sync::broadcast;

pub struct Dispatcher(broadcast::Sender<Message>);

impl Default for Dispatcher {
    fn default() -> Self {
        const COMMAND_QUEUE_SIZE: usize = 10;

        Self(broadcast::Sender::new(COMMAND_QUEUE_SIZE))
    }
}

impl Dispatcher {
    pub fn new_handler(&self) -> Handler {
        Handler(self.0.subscribe())
    }

    pub fn flush(&self) {
        self.send_message(Message::Flush {});
    }

    pub fn set_rate_anchor_time(&self, rate: f32) {
        self.send_message(Message::SetRateAnchorTime { rate })
    }

    fn send_message(&self, message: Message) {
        let _ = self
            .0
            .send(message)
            .inspect_err(|err| tracing::warn!(%err, "couldn't send message"));
    }
}

pub struct Handler(broadcast::Receiver<Message>);

impl Handler {
    pub async fn receive_message(&mut self) -> Option<Message> {
        self.0
            .recv()
            .await
            .inspect_err(|err| tracing::warn!(%err, "skipped some message"))
            .ok()
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    Flush {},
    SetRateAnchorTime {
        rate: f32,
        // rtp_timestamp: u64,
    },
}
