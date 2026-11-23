//! Playback abstraction used by decrypted audio and video streams.

use std::{error::Error, future::Future, io, sync::Weak};

pub mod audio;
pub mod null;
pub mod video;

/// Factory for creating per-session playback streams.
pub trait Device: Send + Sync + 'static {
    /// Parameters passed when a stream is created.
    type Params;
    /// Concrete stream type produced by this device.
    type Stream: Stream;
    /// Error returned if stream creation fails.
    type Error: Error;

    /// Creates a playback stream for a new channel.
    ///
    /// `id` is the AirPlay stream identifier. `handle` can be used to request
    /// that the underlying channel be closed.
    fn create(
        &self,
        id: u64,
        params: Self::Params,
        handle: Weak<dyn ChannelHandle>,
    ) -> impl Future<Output = Result<Self::Stream, Self::Error>> + Send;
}

/// Control handle for an active transport channel.
pub trait ChannelHandle: Send + Sync + 'static {
    /// Requests the channel to close.
    fn close(&self);
}

/// Sink for decrypted stream data.
pub trait Stream: Send + Sync + 'static {
    /// Packet type delivered to this stream.
    type Content;
    /// Error returned if packet delivery fails.
    type Error: Error + Send + Sync + 'static;

    /// Delivers one packet.
    ///
    /// Runs on the network task and should return promptly.
    /// The backend may queue or drop the packet and return [`Ok(())`](Result::Ok).
    ///
    /// An error ends this stream.
    fn on_data(&self, content: Self::Content) -> Result<(), Self::Error>;

    /// Reports why the channel ended and consumes the stream.
    fn on_end(self, reason: EndReason<Self::Error>);
}

/// Why a playback stream ended.
#[derive(Debug)]
pub enum EndReason<E> {
    /// The transport finished, including a peer EOF, reset, or abort.
    Finished,
    /// The receiver closed the channel or cancelled its setup.
    Cancelled,
    /// Accepting a media connection or processing the stream failed.
    Transport(io::Error),
    /// [`Stream::on_data`] returned an error.
    Backend(E),
}
