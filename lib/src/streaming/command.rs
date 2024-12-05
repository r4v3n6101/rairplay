use std::time::Duration;

use tokio::sync::{mpsc, oneshot};

pub fn channel() -> (Dispatcher, Handler) {
    const COMMAND_QUEUE_SIZE: usize = 1;

    let (tx, rx) = mpsc::channel(COMMAND_QUEUE_SIZE);

    (Dispatcher(tx), Handler(rx))
}

pub struct Dispatcher(mpsc::Sender<Message>);

impl Dispatcher {
    fn send_message(&self, message: Message) {
        const COMMAND_SEND_TIME_OUT: Duration = Duration::from_secs(1);

        let sender = self.0.clone();
        tokio::spawn(async move {
            if let Err(err) = sender.send_timeout(message, COMMAND_SEND_TIME_OUT).await {
                tracing::warn!(message = ?err.into_inner(), "couldn't send message");
            }
        });
    }

    pub fn flush(&self) {
        self.send_message(Message::Flush {});
    }

    pub fn set_rate_anchor_time(&self, rate: f32) {
        self.send_message(Message::SetRateAnchorTime { rate })
    }

    pub fn set_volume(&self, value: f32) {
        self.send_message(Message::SetVolume { value })
    }

    pub fn get_volume(&self) -> Option<f32> {
        let (tx, rx) = oneshot::channel();
        self.send_message(Message::GetVolume { channel: tx });
        rx.blocking_recv().ok()
    }
}

pub struct Handler(mpsc::Receiver<Message>);

impl Handler {
    pub async fn receive_message(&mut self) -> Option<Message> {
        self.0.recv().await
    }
}

#[derive(Debug)]
pub enum Message {
    Flush {},
    SetRateAnchorTime {
        rate: f32,
        // rtp_timestamp: u64,
    },
    SetVolume {
        value: f32,
    },
    GetVolume {
        channel: oneshot::Sender<f32>,
    },
}
