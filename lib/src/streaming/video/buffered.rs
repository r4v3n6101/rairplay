use std::{
    ffi::{c_int, c_void},
    io,
    net::SocketAddr,
    ptr,
};

use tokio::{
    io::AsyncReadExt,
    net::{TcpListener, TcpStream, ToSocketAddrs},
};

use crate::crypto::video::AesCipher as AesVideoCipher;

use super::packet::VideoHeader;

extern "C" {
    fn init_ctx(avcc: *mut u8, avcclen: c_int) -> *mut c_void;
    fn free_ctx(ctx: *mut *mut c_void);
    fn decode_frame(avctx: *mut c_void, pkt: *mut c_void, pktlen: c_int);
}

struct Ctx {
    avctx: *mut c_void,
}

unsafe impl Send for Ctx {}
unsafe impl Sync for Ctx {}

async fn processor(mut stream: TcpStream, mut cipher: Option<AesVideoCipher>) {
    let mut cipher = cipher.unwrap();

    let mut ctx = Ctx {
        avctx: ptr::null_mut(),
    };
    loop {
        let mut header = VideoHeader::empty();
        if let Err(err) = stream.read_exact(&mut *header).await {
            tracing::warn!(%err, "can't read video packet header");
            break;
        }

        let mut payload = vec![0u8; header.payload_len() as usize];
        if let Err(err) = stream.read_exact(&mut payload).await {
            tracing::warn!(%err, "can't read video packet payload");
            break;
        }

        match header.payload_type() {
            1 => unsafe {
                ctx.avctx = init_ctx(payload.as_mut_ptr(), payload.len() as _);
            },
            0 | 4096 => {
                cipher.decrypt(&mut payload);
                unsafe {
                    decode_frame(ctx.avctx, payload.as_mut_ptr() as _, payload.len() as _);
                }
            }
            5 => {
                // Just skip, I have no sense what is this
            }
            payload_type => {
                tracing::info!(%payload_type, payload_len=%payload.len(), "unknown video header type");
            }
        }
    }

    if !ctx.avctx.is_null() {
        unsafe {
            free_ctx(&mut ctx.avctx);
        }
    }
}

pub struct Channel {
    local_addr: SocketAddr,
}

impl Channel {
    pub async fn create(
        bind_addr: impl ToSocketAddrs,
        cipher: Option<AesVideoCipher>,
    ) -> io::Result<Self> {
        let listener = TcpListener::bind(bind_addr).await?;
        let local_addr = listener.local_addr()?;

        tokio::spawn(async move {
            match listener.accept().await {
                Ok((stream, remote_addr)) => {
                    tracing::info!(%local_addr, %remote_addr, "accepting connection");
                    processor(stream, cipher).await;
                    // TODO : what if done with error?
                    tracing::info!(%local_addr, %remote_addr, "video stream done");
                }
                Err(err) => {
                    tracing::warn!(%err, %local_addr, "failed to accept connection");
                }
            }
        });

        Ok(Channel { local_addr })
    }

    pub fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }
}
