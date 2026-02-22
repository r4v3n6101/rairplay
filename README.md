# rairplay

**rairplay** is a Rust library for receiving and processing **audio and video streams from Apple’s AirPlay2 protocol**.


## Features

- [x] RTSP server for handling AirPlay2 commands (e.g. changing volume, setting up streams)
- [x] Decrypting **audio streams** (both real-time and buffered is supported)  
- [x] Decrypting **video streams** 
- [x] Legacy pairing (using **X25519**, **ED25519** and **DH**)
- [x] FairPlay (v3) using [shairplay](https://github.com/juhovh/shairplay)
- [x] HomeKit pairing (video and RTSP work, though audio couldn't be decrypted)  
- [x] "DJ mode" for managing many devices at once
- [x] Example implementation using **GStreamer** library to pipe stream data into a file  
- [ ] **GStreamer** plugin creating streams according to **rairplay**'s API.


## Getting Started

```bash
# Clone it locally
git clone https://github.com/r4v3n6101/rairplay.git
cd rairplay

# Set up dev environment via `nix`
nix develop .

# Run or `cargo build` if you need
cargo run
```


## Used resources
- [Old AirPlay1 protocol specification](https://nto.github.io/AirPlay)
- [airplay2-receiver](https://github.com/openairplay/airplay2-receiver)
- [emmanuelecozzi blog](https://emanuelecozzi.net/docs/airplay2)
- [AirPlay2 Protocol](https://github.com/SteeBono/airplayreceiver/wiki/AirPlay2-Protocol)
- [AirPlay2 Analysis](https://www.programmersought.com/article/2084789418/)
- A lot of my nerves, patience and time spent with [my friend](https://www.wireshark.org)


## License

rairplay is distributed under the **GNU General Public License, version 3 (GPL-3.0)**.  
You may use, modify, and redistribute the software under the terms of this license.  
A full copy of the license text is available in the `LICENSE` file included with the project.
