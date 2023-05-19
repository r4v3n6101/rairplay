use std::fmt::Debug;

use futures::Sink;

pub type AudioPacket = Vec<u8>;

pub trait AudioCipher: Debug {
    fn decrypt(&mut self, input: &[u8], output: &mut [u8]);
}

impl<A: AudioCipher + ?Sized> AudioCipher for Box<A> {
    fn decrypt(&mut self, input: &[u8], output: &mut [u8]) {
        A::decrypt(self, input, output);
    }
}

pub trait AudioSink: Sink<AudioPacket> + Debug {
    fn initialize(sample_rate: u32, sample_size: u16, channels: u8) -> Self;
    // TODO : maybe remove mut?
    fn set_volume(&mut self, value: f32);
    fn get_volume(&self) -> f32;
}
