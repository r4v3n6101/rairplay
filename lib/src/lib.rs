#![warn(clippy::pedantic)]

mod crypto;
mod info;
mod rtsp;
mod streaming;

pub use {info::*, rtsp::svc_router};
