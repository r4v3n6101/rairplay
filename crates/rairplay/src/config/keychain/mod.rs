pub mod default;

/// Stores receiver identity material and trusted peer keys.
///
/// Implement this trait when the default in-memory keychain is not sufficient,
/// for example when keys must persist across restarts.
pub trait Keychain: Send + Sync + 'static {
    /// Returns the receiver identity used by pairing.
    fn id(&self) -> &[u8];

    /// Returns the receiver public key.
    fn pubkey(&self) -> &[u8];

    /// Signs a message with the receiver private key.
    fn sign(&self, data: &[u8]) -> Vec<u8>;

    /// Records a trusted peer key.
    fn trust(&self, id: &[u8], key: &[u8]) -> bool;

    /// Verifies a signature from a trusted peer.
    fn verify(&self, id: &[u8], message: &[u8], signature: &[u8]) -> bool;
}
