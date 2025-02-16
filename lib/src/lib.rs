#![warn(clippy::pedantic)]

mod info;
mod rtsp;
mod streaming;
mod util;

pub use {info::*, rtsp::svc_router};
