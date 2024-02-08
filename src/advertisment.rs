use std::fmt::{self, Display};

use bitflags::bitflags;
use mac_address::{get_mac_address, MacAddress};

const INVALID_PINS: [[u8; 8]; 12] = [
    [0, 0, 0, 0, 0, 0, 0, 0],
    [1, 1, 1, 1, 1, 1, 1, 1],
    [2, 2, 2, 2, 2, 2, 2, 2],
    [3, 3, 3, 3, 3, 3, 3, 3],
    [4, 4, 4, 4, 4, 4, 4, 4],
    [5, 5, 5, 5, 5, 5, 5, 5],
    [6, 6, 6, 6, 6, 6, 6, 6],
    [7, 7, 7, 7, 7, 7, 7, 7],
    [8, 8, 8, 8, 8, 8, 8, 8],
    [9, 9, 9, 9, 9, 9, 9, 9],
    [1, 2, 3, 4, 5, 6, 7, 8],
    [8, 7, 6, 5, 4, 3, 2, 1],
];

bitflags! {
    #[repr(transparent)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct Features: u64 {
        const Video = 1 << 0;
        const Photo = 1 << 1;
        const VideoFairPlay = 1 << 2;
        const VideoVolumeCtrl = 1 << 3;
        const VideoHTTPLiveStreaming = 1 << 4;
        const Slideshow = 1 << 5;
        const _ = 1 << 6;
        const ScreenMirroring = 1 << 7;
        const ScreenRotate = 1 << 8;
        const AirPlayAudio = 1 << 9;
        const _ = 1 << 10;
        const AudioRedundant = 1 << 11;
        const FPSAPv2p5_AES_GCM = 1 << 12;
        const MFiHardware = 1 << 13;
        const MFiSoft_FairPlay = 1 << 14;
        const AudioMetaCovers = 1 << 15;
        const AudioMetaProgress = 1 << 16;
        const AudioMetaTxtDAAP = 1 << 17;
        const ReceiveAudioPCM = 1 << 18;
        const ReceiveAudioALAC = 1 << 19;
        const ReceiveAudioAAC_LC = 1 << 20;
        const _ = 1 << 21;
        const AudioUnencrypted = 1 << 22;
        const RSA_Auth = 1 << 23;
        const _ = 1 << 24;
        const iTunes4WEncryption = 1 << 25;
        const Audio_AES_Mfi = 1 << 26;
        const LegacyPairing = 1 << 27;
        const _ = 1 << 28;
        const plistMetaData = 1 << 29;
        const UnifiedAdvertisingInfo = 1 << 30;
        const CarPlay = 1 << 32;
        const AirPlayVideoPlayQueue = 1 << 33;
        const AirPlayFromCloud = 1 << 34;
        const TLS_PSK = 1 << 35;
        const _ = 1 << 36;
        const CarPlayControl = 1 << 37;
        const ControlChannelEncrypt = 1 << 38;
        const _ = 1 << 39;
        const BufferedAudio = 1 << 40;
        const PTPClock = 1 << 41;
        const ScreenMultiCodec = 1 << 42;
        const SystemPairing = 1 << 43;
        const APValeriaScreenSend = 1 << 44;
        const NTPClock = 1 << 45;
        const HomeKitPairing = 1 << 46;
        const PeerManagement = 1 << 47;
        const TransientPairing = 1 << 48;
        const AirPlayVideoV2 = 1 << 49;
        const NowPlayingInfo = 1 << 50;
        const MfiPairSetup = 1 << 51;
        const PeersExtendedMessage = 1 << 52;
        const _ = 1 << 53;
        const SupportsAPSync = 1 << 54;
        const SupportsWoL1 = 1 << 55;
        const SupportsWoL2 = 1 << 56;
        const _ = 1 << 57;
        const HangdogRemote = 1 << 58;
        const AudioStreamConnectionSetup = 1 << 59;
        const AudioMediaDataControl = 1 << 60;
        const RFC2198Redundant = 1 << 61;
        const _ = 1 << 62;
    }
}

impl Default for Features {
    fn default() -> Self {
        Self::AirPlayAudio
            | Self::MFiSoft_FairPlay
            | Self::ReceiveAudioPCM
            | Self::ReceiveAudioALAC
            | Self::ReceiveAudioAAC_LC
            | Self::AudioUnencrypted
            | Self::UnifiedAdvertisingInfo
            | Self::BufferedAudio
            | Self::PTPClock
            | Self::PeerManagement
    }
}

#[derive(Debug, Copy, Clone)]
pub struct Pin {
    digits: [u8; 8],
}

impl Pin {
    pub fn new(digits: [u8; 8]) -> Option<Self> {
        if INVALID_PINS.contains(&digits) {
            None
        } else {
            Some(Self { digits })
        }
    }

    pub fn octets(self) -> [u8; 8] {
        self.digits
    }
}

impl Display for Pin {
    fn fmt(&self, fmtr: &mut fmt::Formatter<'_>) -> fmt::Result {
        let [a, b, c, d, e, f, g, h] = self.digits;
        write!(fmtr, "{a}{b}-{c}{d}{e}-{f}{g}{h}")
    }
}

#[derive(Debug, Clone)]
pub struct AdvData {
    pub mac_addr: MacAddress,
    pub features: Features,
    pub manufacturer: String,
    pub model: String,
    pub name: String,
    pub fw_version: String,
    pub pin: Option<Pin>,
}

impl Default for AdvData {
    fn default() -> Self {
        let mac_addr = match get_mac_address() {
            Ok(Some(addr)) => addr,
            _ => [0x9F, 0xD7, 0xAF, 0x1F, 0xD3, 0xCD].into(),
        };
        Self {
            mac_addr,
            features: Features::default(),
            manufacturer: env!("CARGO_PKG_AUTHORS").to_string(),
            model: env!("CARGO_PKG_NAME").to_string(),
            name: env!("CARGO_PKG_NAME").to_string(),
            fw_version: env!("CARGO_PKG_VERSION").to_string(),
            pin: None,
        }
    }
}
