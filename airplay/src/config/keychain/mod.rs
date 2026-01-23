pub mod default;

pub trait Keychain: Send + Sync + 'static {
    fn id(&self) -> &[u8];

    fn pubkey(&self) -> &[u8];

    fn sign(&self, data: &[u8]) -> Vec<u8>;

    fn trust(&self, id: &[u8], key: &[u8]) -> bool;

    fn verify(&self, id: &[u8], message: &[u8], signature: &[u8]) -> bool;
}
