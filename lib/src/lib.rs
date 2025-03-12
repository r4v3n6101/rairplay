#![warn(clippy::pedantic)]

mod crypto;
mod device;
// TODO : rename it
mod info;
mod rtsp;
mod streaming;
mod util;

pub use {info::*, rtsp::svc_router};
