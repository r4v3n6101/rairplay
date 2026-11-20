//! Playback abstraction used by decrypted audio and video streams.

use std::{error::Error, future::Future, sync::Weak};

pub mod audio;
pub mod null;
pub mod video;

/// Factory for creating per-session playback streams.
///
/// A device is long-lived. The crate calls [`Device::create`] whenever a new
/// audio or video channel is negotiated.
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
///
/// A stream receives packet payloads through [`Stream::on_data`] and is then
/// completed with either [`Stream::on_ok`] or [`Stream::on_err`].
pub trait Stream: Send + Sync + 'static {
    /// Packet type delivered to this stream.
    type Content;

    /// Delivers one decrypted packet.
    fn on_data(&self, content: Self::Content);
    /// Signals clean stream termination.
    fn on_ok(self);
    /// Signals stream termination due to an error.
    fn on_err(self, err: Box<dyn Error>);
}
