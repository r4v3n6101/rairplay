use std::{error::Error, sync::Weak};

pub mod audio;
pub mod null;
pub mod video;

pub trait Device: Send + Sync + 'static {
    type Params;
    type Stream: Stream;

    fn create(&self, params: Self::Params, handle: Weak<dyn ChannelHandle>) -> Self::Stream;
}

pub trait ChannelHandle: Send + Sync + 'static {
    fn close(&self);
}

pub trait Stream: Send + Sync + 'static {
    type Content;

    fn on_data(&self, content: Self::Content);
    fn on_ok(self);
    fn on_err(self, err: Box<dyn Error>);
}
