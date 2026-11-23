# rairplay

`rairplay` is a Rust AirPlay receiver library focused on AirPlay 2 control flow, pairing, FairPlay setup, and decrypted audio/video delivery into user-provided playback backends.

It is a library crate, not a ready-to-run application. You bring the networking entrypoint, device implementations, persistence strategy, and any media decoding or rendering you need on top.

## Current Scope

- RTSP service for AirPlay session control
- AirPlay audio stream handling
- AirPlay video stream handling
- FairPlay v3 integration through bundled `shairplay` sources
- Legacy pairing support
- HomeKit pairing support
- Pluggable audio and video playback devices
- Dual-stack transport listener for IPv4 and IPv6

## Build Notes

Add the library to a Cargo project with `cargo add rairplay`. The package includes the FairPlay C sources and headers. Building it requires Rust and a C compiler.

From the workspace root, initialize the pinned submodule before building or packaging from a Git checkout.

```bash
git submodule update --init -- 3rdparty/shairplay
cargo test --workspace
```

`nix develop` opens a development shell with the Rust toolchain from `rust-toolchain` and a C compiler.

To use an alternative copy of the FairPlay sources, set `FAIRPLAY3_SRC` to its directory.

```bash
FAIRPLAY3_SRC=/path/to/shairplay/src/lib/playfair cargo build
```

## Minimal Integration

The crate is centered around three pieces:

- `config::Config` describes the receiver identity, pairing mode, features, and playback backends
- `ServiceFactory` builds an Axum service per incoming AirPlay connection
- `transport::DualStackListenerWithRtspRemap` accepts AirPlay-compatible TCP connections on IPv4 and IPv6

Minimal skeleton:

```rust
use std::{net::{Ipv4Addr, Ipv6Addr, SocketAddrV4, SocketAddrV6}, sync::Arc};

use rairplay::{
    ServiceFactory,
    config::{Config, DefaultKeychain},
    playback::{
        audio::{AudioPacket, AudioParams},
        null::NullDevice,
        video::{VideoPacket, VideoParams},
    },
    transport::DualStackListenerWithRtspRemap,
};

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Arc::new(Config {
        keychain: DefaultKeychain::default(),
        audio: rairplay::config::Audio {
            device: NullDevice::<AudioParams, AudioPacket>::default(),
            ..Default::default()
        },
        video: rairplay::config::Video {
            device: NullDevice::<VideoParams, VideoPacket>::default(),
            ..Default::default()
        },
        ..Default::default()
    });

    let listener = DualStackListenerWithRtspRemap::bind(
        SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 7000),
        SocketAddrV6::new(Ipv6Addr::UNSPECIFIED, 7000, 0, 0),
    )?;

    axum::serve(listener, ServiceFactory::new(config)).await?;
    Ok(())
}
```

The null devices are useful for bring-up and protocol testing because they accept streams and discard payloads while still exercising pairing and session setup.

## Playback Model

`rairplay` does not decode or render media for you. Instead, you implement the playback traits:

- `playback::audio::AudioDevice` creates per-stream audio sinks and exposes volume control
- `playback::video::VideoDevice` creates per-stream video sinks
- `playback::Stream` receives decrypted packets through `on_data`. When the stream ends, the receiver calls `on_end` once with the reason.

That design keeps the crate transport- and protocol-focused. It is a good fit if you want to wire AirPlay into an existing media pipeline, custom player, transcoder, or embedded device.

## Pairing And Keys

Two pairing modes are exposed through `config::Pairing`:

- `Legacy`
- `HomeKit`

The default key storage implementation is `config::DefaultKeychain`. It is useful for development, but it is in-memory and ships with fixed default identity material, so it is not appropriate for production deployments. Real integrations should provide a custom `config::Keychain` implementation backed by persistent device keys and trusted-peer storage.

## Repository Layout

The library lives in `crates/rairplay/`. Paths below are relative to that directory.

- `src/config`: receiver configuration, pairing mode, PINs, keychain abstraction
- `src/playback`: audio/video device traits and a null backend
- `src/transport`: listener and protocol transport glue
- `src/rtsp`: RTSP request handling
- `src/pairing`: legacy and HomeKit pairing flows
- `src/streaming`: stream synchronization and packet processing
- `3rdparty/shairplay/` contains individual links to the C sources and headers in the workspace's `3rdparty/shairplay/src/lib/playfair/` directory.

## Known Limits

- No binary, CLI, or packaged receiver daemon is included
- Public API documentation is still sparse
- Production key management is left to the integrator
- Media decoding, muxing, playback, and persistence are out of scope

## References

- [Old AirPlay1 protocol specification](https://nto.github.io/AirPlay)
- [openairplay/airplay2-receiver](https://github.com/openairplay/airplay2-receiver)
- [Emanuele Cozzi's AirPlay 2 notes](https://emanuelecozzi.net/docs/airplay2)
- [AirPlay2 Protocol wiki notes](https://github.com/SteeBono/airplayreceiver/wiki/AirPlay2-Protocol)

## License

This project is licensed under the GNU General Public License v3.0. See [`LICENSE`](LICENSE).
