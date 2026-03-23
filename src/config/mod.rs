//! Receiver configuration types.

use bitflags::bitflags;
use derivative::Derivative;
/// Key storage and trust management used by pairing.
pub use keychain::{Keychain, default::DefaultKeychain};
/// Receiver MAC address type.
pub use macaddr::MacAddr6;
/// Pairing PIN types.
pub use pin::{PinCode, PinError};

mod keychain;
mod pin;

/// Top-level receiver configuration.
///
/// This binds together the receiver identity advertised to clients, the
/// pairing mode, supported AirPlay features, and the concrete audio/video
/// backends that will receive decrypted stream data.
#[derive(Debug, Derivative)]
#[derivative(Default)]
pub struct Config<ADev, VDev, KC> {
    /// MAC address advertised by the receiver.
    pub mac_addr: MacAddr6,
    /// Feature bits reported during capability discovery.
    pub features: Features,
    /// Manufacturer string exposed to clients.
    #[derivative(Default(value = "env!(\"CARGO_PKG_AUTHORS\").to_string()"))]
    pub manufacturer: String,
    /// Model string exposed to clients.
    #[derivative(Default(value = "env!(\"CARGO_PKG_NAME\").to_string()"))]
    pub model: String,
    /// Friendly display name shown by clients.
    #[derivative(Default(value = "env!(\"CARGO_PKG_NAME\").to_string()"))]
    pub name: String,
    /// Firmware version string exposed to clients.
    #[derivative(Default(value = "env!(\"CARGO_PKG_VERSION\").to_string()"))]
    pub fw_version: String,

    /// Optional PIN required by pairing flows that use one.
    pub pin: Option<PinCode>,
    /// Receiver identity and trusted peer storage.
    pub keychain: KC,
    /// Pairing protocol exposed by the receiver.
    pub pairing: Pairing,
    /// Audio backend configuration.
    pub audio: Audio<ADev>,
    /// Video backend configuration.
    pub video: Video<VDev>,
}

/// Pairing protocol used by the receiver.
#[derive(Debug, Default, Copy, Clone)]
pub enum Pairing {
    /// Legacy AirPlay pairing.
    #[default]
    Legacy,
    /// HomeKit-based pairing.
    HomeKit,
}

/// Audio-specific configuration.
///
/// The device creates per-session audio sinks, while `buf_size` controls how
/// much stream data can be buffered internally for a single audio session.
#[derive(Derivative)]
#[derivative(Debug, Default)]
pub struct Audio<Device> {
    /// Maximum buffered audio payload per stream, in bytes.
    #[derivative(Default(value = "1024 * 1024"))]
    pub buf_size: u32,
    /// Audio device factory used for new streams.
    pub device: Device,
}

/// Video-specific configuration.
///
/// The advertised dimensions and frame rate affect how the receiver describes
/// itself to clients. `buf_size` controls the internal per-stream buffer.
#[derive(Derivative)]
#[derivative(Debug, Default)]
pub struct Video<Device> {
    /// Advertised output width.
    #[derivative(Default(value = "1920"))]
    pub width: u32,
    /// Advertised output height.
    #[derivative(Default(value = "1080"))]
    pub height: u32,
    /// Advertised frame rate.
    #[derivative(Default(value = "30"))]
    pub fps: u32,
    /// Maximum buffered video payload per stream, in bytes.
    #[derivative(Default(value = "1024 * 1024"))]
    pub buf_size: u32,
    /// Video device factory used for new streams.
    pub device: Device,
}

bitflags! {
    /// AirPlay capability bits advertised by the receiver.
    ///
    /// Clients may interpret combinations of these flags, so they should stay
    /// aligned with the behavior actually implemented by the crate.
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

/// Default feature set implemented by the crate.
impl Default for Features {
    fn default() -> Self {
        Self::Video
            | Self::Photo
            | Self::VideoHTTPLiveStreaming
            | Self::Unknown6
            | Self::ScreenMirroring
            | Self::AirPlayAudio
            | Self::LegacyPairing

            // Seems like not mandatory
            | Self::AudioMetaCovers
            | Self::AudioMetaTxtDAAP
            | Self::AudioMetaProgress

            | Self::ReceiveAudioPCM
            | Self::ReceiveAudioALAC
            | Self::ReceiveAudioAAC_LC

            // A glitch whether /fp-setup is called, but the audio/video data is clear
            | Self::MFiSoft_FairPlay
            | Self::HomeKitPairing
            // | Self::AudioUnencrypted

            // Seems like needed for a GET /info call
            | Self::UnifiedAdvertisingInfo

            // Enable AirPlay2, using buffered audio (e.g. Apple Music)
            | Self::BufferedAudio
            | Self::NTPClock
            | Self::PTPClock
    }
}
