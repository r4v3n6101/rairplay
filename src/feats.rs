use bitflags::bitflags;

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

impl Features {
    #[tracing::instrument]
    pub fn validate(&self) {
        self.iter_names()
            .for_each(|(name, feat): (&str, Features)| match feat {
                Features::Video => {
                    tracing::error!(feature = name, "not implemented");
                }
                Features::Photo => {
                    tracing::error!(feature = name, "not implemented");
                }
                Features::VideoFairPlay => {
                    tracing::error!(feature = name, "not implemented");
                }
                Features::VideoVolumeCtrl => {
                    tracing::error!(feature = name, "not implemented");
                }
                Features::VideoHTTPLiveStreaming => {
                    tracing::error!(feature = name, "not implemented");
                }
                Features::Slideshow => {
                    tracing::error!(feature = name, "not implemented");
                }
                Features::Unknown6 => {
                    tracing::error!(feature = name, "not implemented");
                }
                Features::ScreenMirroring => {
                    tracing::error!(feature = name, "not implemented");
                }
                Features::ScreenRotate => {
                    tracing::error!(feature = name, "not implemented");
                }
                Features::AirPlayAudio => {
                    tracing::info!(feature = name, "working");
                }
                Features::Unknown10 => {
                    tracing::error!(feature = name, "not implemented");
                }
                Features::AudioRedundant => {
                    tracing::error!(feature = name, "not implemented");
                }
                Features::FPSAPv2p5_AES_GCM => {
                    tracing::error!(feature = name, "not implemented");
                }
                Features::MFiHardware => {
                    tracing::error!(feature = name, "not implemented");
                }
                Features::MFiSoft_FairPlay => {
                    tracing::warn!(feature = name, "working with limitations");
                }
                Features::AudioMetaCovers => {
                    tracing::info!(feature = name, "working");
                }
                Features::AudioMetaProgress => {
                    tracing::info!(feature = name, "working");
                }
                Features::AudioMetaTxtDAAP => {
                    tracing::info!(feature = name, "working");
                }
                Features::ReceiveAudioPCM => {
                    tracing::info!(feature = name, "working");
                }
                Features::ReceiveAudioALAC => {
                    tracing::info!(feature = name, "working");
                }
                Features::ReceiveAudioAAC_LC => {
                    tracing::info!(feature = name, "working");
                }
                Features::Unknown21 => {
                    tracing::error!(feature = name, "not implemented");
                }
                Features::AudioUnencrypted => {
                    tracing::info!(feature = name, "working");
                }
                Features::RSA_Auth => {
                    tracing::error!(feature = name, "not implemented");
                }
                Features::Unknown24 => {
                    tracing::error!(feature = name, "not implemented");
                }
                Features::iTunes4WEncryption => {
                    tracing::error!(feature = name, "not implemented");
                }
                Features::Audio_AES_Mfi => {
                    tracing::error!(feature = name, "not implemented");
                }
                Features::LegacyPairing => {
                    tracing::error!(feature = name, "not implemented");
                }
                Features::Unknown28 => {
                    tracing::error!(feature = name, "not implemented");
                }
                Features::plistMetaData => {
                    tracing::error!(feature = name, "not implemented");
                }
                Features::UnifiedAdvertisingInfo => {
                    tracing::info!(feature = name, "working");
                }
                Features::CarPlay => {
                    tracing::error!(feature = name, "not implemented");
                }
                Features::AirPlayVideoPlayQueue => {
                    tracing::error!(feature = name, "not implemented");
                }
                Features::AirPlayFromCloud => {
                    tracing::error!(feature = name, "not implemented");
                }
                Features::TLS_PSK => {
                    tracing::error!(feature = name, "not implemented");
                }
                Features::Unknown36 => {
                    tracing::error!(feature = name, "not implemented");
                }
                Features::CarPlayControl => {
                    tracing::error!(feature = name, "not implemented");
                }
                Features::ControlChannelEncrypt => {
                    tracing::error!(feature = name, "not implemented");
                }
                Features::Unknown39 => {
                    tracing::error!(feature = name, "not implemented");
                }
                Features::BufferedAudio => {
                    tracing::info!(feature = name, "working");
                }
                Features::PTPClock => {
                    tracing::warn!(feature = name, "working with limitations");
                }
                Features::ScreenMultiCodec => {
                    tracing::error!(feature = name, "not implemented");
                }
                Features::SystemPairing => {
                    tracing::error!(feature = name, "not implemented");
                }
                Features::APValeriaScreenSend => {
                    tracing::error!(feature = name, "not implemented");
                }
                Features::NTPClock => {
                    tracing::warn!(feature = name, "working with limitations");
                }
                Features::HomeKitPairing => {
                    tracing::error!(feature = name, "not implemented");
                }
                Features::PeerManagement => {
                    tracing::info!(feature = name, "working");
                }
                Features::TransientPairing => {
                    tracing::error!(feature = name, "not implemented");
                }
                Features::AirPlayVideoV2 => {
                    tracing::error!(feature = name, "not implemented");
                }
                Features::NowPlayingInfo => {
                    tracing::error!(feature = name, "not implemented");
                }
                Features::MfiPairSetup => {
                    tracing::error!(feature = name, "not implemented");
                }
                Features::PeersExtendedMessage => {
                    tracing::error!(feature = name, "not implemented");
                }
                Features::Unknown53 => {
                    tracing::error!(feature = name, "not implemented");
                }
                Features::SupportsAPSync => {
                    tracing::error!(feature = name, "not implemented");
                }
                Features::SupportsWoL1 => {
                    tracing::error!(feature = name, "not implemented");
                }
                Features::SupportsWoL2 => {
                    tracing::error!(feature = name, "not implemented");
                }
                Features::Unknown57 => {
                    tracing::error!(feature = name, "not implemented");
                }
                Features::HangdogRemote => {
                    tracing::error!(feature = name, "not implemented");
                }
                Features::AudioStreamConnectionSetup => {
                    tracing::error!(feature = name, "not implemented");
                }
                Features::AudioMediaDataControl => {
                    tracing::error!(feature = name, "not implemented");
                }
                Features::RFC2198Redundant => {
                    tracing::error!(feature = name, "not implemented");
                }
                Features::Unknown62 => {
                    tracing::error!(feature = name, "not implemented");
                }
                _ => {}
            });
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
    }
}
