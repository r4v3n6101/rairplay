use bitflags::*;

pub use macaddr::MacAddr6;

#[derive(Debug, Clone)]
pub struct Config {
    pub mac_addr: MacAddr6,
    pub features: Features,
    pub manufacturer: String,
    pub model: String,
    pub name: String,
    pub fw_version: String,
    pub initial_volume: Option<f32>,
}

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
        const Unknown6 = 1 << 6;
        const ScreenMirroring = 1 << 7;
        const ScreenRotate = 1 << 8;
        const AirPlayAudio = 1 << 9;
        const Unknown10 = 1 << 10;
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
        const Unknown21 = 1 << 21;
        const AudioUnencrypted = 1 << 22;
        const RSA_Auth = 1 << 23;
        const Unknown24 = 1 << 24;
        const iTunes4WEncryption = 1 << 25;
        const Audio_AES_Mfi = 1 << 26;
        const LegacyPairing = 1 << 27;
        const Unknown28 = 1 << 28;
        const plistMetaData = 1 << 29;
        const UnifiedAdvertisingInfo = 1 << 30;
        const CarPlay = 1 << 32;
        const AirPlayVideoPlayQueue = 1 << 33;
        const AirPlayFromCloud = 1 << 34;
        const TLS_PSK = 1 << 35;
        const Unknown36 = 1 << 36;
        const CarPlayControl = 1 << 37;
        const ControlChannelEncrypt = 1 << 38;
        const Unknown39 = 1 << 39;
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
        const Unknown53 = 1 << 53;
        const SupportsAPSync = 1 << 54;
        const SupportsWoL1 = 1 << 55;
        const SupportsWoL2 = 1 << 56;
        const Unknown57 = 1 << 57;
        const HangdogRemote = 1 << 58;
        const AudioStreamConnectionSetup = 1 << 59;
        const AudioMediaDataControl = 1 << 60;
        const RFC2198Redundant = 1 << 61;
        const Unknown62 = 1 << 62;
    }
}

/// Default features that supported by the current version of the crate.
/// Modify it if you make any changes into the code.
impl Default for Features {
    fn default() -> Self {
        Self::Video
            | Self::Photo
            | Self::VideoHTTPLiveStreaming
            | Self::Unknown6
            | Self::ScreenMirroring
            | Self::AirPlayAudio

            // TODO : | Self::AudioRedundant

            // Seems like not mandatory
            | Self::AudioMetaCovers
            | Self::AudioMetaTxtDAAP
            | Self::AudioMetaProgress

            | Self::ReceiveAudioPCM
            | Self::ReceiveAudioALAC
            | Self::ReceiveAudioAAC_LC

            // A glitch whether /fp-setup is called, but the audio/video data is clear
            | Self::MFiSoft_FairPlay
            | Self::AudioUnencrypted

            // Seems like needed for a GET /info call
            | Self::UnifiedAdvertisingInfo

            // Enable AirPlay2, using buffered audio (e.g. Apple Music)
            | Self::BufferedAudio
            | Self::NTPClock
            | Self::PTPClock
    }
}
