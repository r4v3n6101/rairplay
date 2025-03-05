#![warn(clippy::pedantic)]

mod crypto;
// TODO : change this name
mod device;
mod info;
mod rtsp;
mod streaming;
mod util;

pub use {info::*, rtsp::svc_router};
