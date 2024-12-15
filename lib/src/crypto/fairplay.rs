//! The main idea of what's going on here is taken from [shairplay](https://github.com/juhovh/shairplay/).
//! Decryption of aes key with the third message of `/fp-setup` is taken from there too.
//!
//! Thank you, my dear!

use thiserror::Error;

use crate::ffi;

const MESSAGES: [&[u8]; 4] = [
    &[
        70, 80, 76, 89, 3, 1, 2, 0, 0, 0, 0, 130, 2, 0, 15, 159, 63, 158, 10, 37, 33, 219, 223, 49,
        42, 178, 191, 178, 158, 141, 35, 43, 99, 118, 168, 200, 24, 112, 29, 34, 174, 147, 216, 39,
        55, 254, 175, 157, 180, 253, 244, 28, 45, 186, 157, 31, 73, 202, 170, 191, 101, 145, 172,
        31, 123, 198, 247, 224, 102, 61, 33, 175, 224, 21, 101, 149, 62, 171, 129, 244, 24, 206,
        237, 9, 90, 219, 124, 61, 14, 37, 73, 9, 167, 152, 49, 212, 156, 57, 130, 151, 52, 52, 250,
        203, 66, 198, 58, 28, 217, 17, 166, 254, 148, 26, 138, 109, 74, 116, 59, 70, 195, 167, 100,
        158, 68, 199, 137, 85, 228, 157, 129, 85, 0, 149, 73, 196, 226, 247, 163, 246, 213, 186,
    ],
    &[
        70, 80, 76, 89, 3, 1, 2, 0, 0, 0, 0, 130, 2, 1, 207, 50, 162, 87, 20, 178, 82, 79, 138,
        160, 173, 122, 241, 100, 227, 123, 207, 68, 36, 226, 0, 4, 126, 252, 10, 214, 122, 252,
        217, 93, 237, 28, 39, 48, 187, 89, 27, 150, 46, 214, 58, 156, 77, 237, 136, 186, 143, 199,
        141, 230, 77, 145, 204, 253, 92, 123, 86, 218, 136, 227, 31, 92, 206, 175, 199, 67, 25,
        149, 160, 22, 101, 165, 78, 25, 57, 210, 91, 148, 219, 100, 185, 228, 93, 141, 6, 62, 30,
        106, 240, 126, 150, 86, 22, 43, 14, 250, 64, 66, 117, 234, 90, 68, 217, 89, 28, 114, 86,
        185, 251, 230, 81, 56, 152, 184, 2, 39, 114, 25, 136, 87, 22, 80, 148, 42, 217, 70, 104,
        138,
    ],
    &[
        70, 80, 76, 89, 3, 1, 2, 0, 0, 0, 0, 130, 2, 2, 193, 105, 163, 82, 238, 237, 53, 177, 140,
        221, 156, 88, 214, 79, 22, 193, 81, 154, 137, 235, 83, 23, 189, 13, 67, 54, 205, 104, 246,
        56, 255, 157, 1, 106, 91, 82, 183, 250, 146, 22, 178, 182, 84, 130, 199, 132, 68, 17, 129,
        33, 162, 199, 254, 216, 61, 183, 17, 158, 145, 130, 170, 215, 209, 140, 112, 99, 226, 164,
        87, 85, 89, 16, 175, 158, 14, 252, 118, 52, 125, 22, 64, 67, 128, 127, 88, 30, 228, 251,
        228, 44, 169, 222, 220, 27, 94, 178, 163, 170, 61, 46, 205, 89, 231, 238, 231, 11, 54, 41,
        242, 42, 253, 22, 29, 135, 115, 83, 221, 185, 154, 220, 142, 7, 0, 110, 86, 248, 80, 206,
    ],
    &[
        70, 80, 76, 89, 3, 1, 2, 0, 0, 0, 0, 130, 2, 3, 144, 1, 225, 114, 126, 15, 87, 249, 245,
        136, 13, 177, 4, 166, 37, 122, 35, 245, 207, 255, 26, 187, 225, 233, 48, 69, 37, 26, 251,
        151, 235, 159, 192, 1, 30, 190, 15, 58, 129, 223, 91, 105, 29, 118, 172, 178, 247, 165,
        199, 8, 227, 211, 40, 245, 107, 179, 157, 189, 229, 242, 156, 138, 23, 244, 129, 72, 126,
        58, 232, 99, 198, 120, 50, 84, 34, 230, 247, 142, 22, 109, 24, 170, 127, 214, 54, 37, 139,
        206, 40, 114, 111, 102, 31, 115, 136, 147, 206, 68, 49, 30, 75, 230, 192, 83, 81, 147, 229,
        239, 114, 232, 104, 98, 51, 114, 156, 34, 125, 130, 12, 153, 148, 69, 216, 146, 70, 200,
        195, 89,
    ],
];
const FP_HEADER: &[u8] = &[70, 80, 76, 89, 3, 1, 4, 0, 0, 0, 0, 20];
const DECRYPTED_AES_KEY_LEN: usize = 16;

#[derive(Debug, Error)]
pub enum DecodingError {
    #[error("insufficient data")]
    InsufficientData,
    #[error("invalid version: {0}")]
    InvalidVersion(u8),
    #[error("invalid msg type: {0}")]
    InvalidMsgType(u8),
    #[error("invalid mode: {0}")]
    InvalidMode(u8),
    #[error("invalid seq: {0}")]
    InvalidSeq(u8),
}

pub fn decode_buf(buf: impl AsRef<[u8]>) -> Result<Vec<u8>, DecodingError> {
    let buf = buf.as_ref();

    match buf.get(4) {
        Some(3) => {}
        Some(version) => return Err(DecodingError::InvalidVersion(*version)),
        None => return Err(DecodingError::InsufficientData),
    }

    match buf.get(5) {
        Some(1) => match buf.get(6) {
            Some(1) => match buf.get(14) {
                Some(mode @ 0..=4) => Ok(MESSAGES[*mode as usize].to_vec()),
                Some(mode) => Err(DecodingError::InvalidMode(*mode)),
                None => Err(DecodingError::InsufficientData),
            },
            Some(3) => {
                let mut output = vec![0; FP_HEADER.len() + 20];
                output[..FP_HEADER.len()].copy_from_slice(FP_HEADER);
                match buf.get(buf.len() - 20..) {
                    Some(suffix) => {
                        output[FP_HEADER.len()..].copy_from_slice(suffix);
                        Ok(output)
                    }
                    None => Err(DecodingError::InsufficientData),
                }
            }
            Some(seq) => Err(DecodingError::InvalidSeq(*seq)),
            None => Err(DecodingError::InsufficientData),
        },
        Some(msg_type) => Err(DecodingError::InvalidMsgType(*msg_type)),
        None => Err(DecodingError::InsufficientData),
    }
}

pub fn decrypt_key(
    message: impl AsRef<[u8]>,
    encrypted_aes_key: impl AsRef<[u8]>,
) -> [u8; DECRYPTED_AES_KEY_LEN] {
    let message = message.as_ref();
    let encrypted_aes_key = encrypted_aes_key.as_ref();
    let mut aes = [0u8; DECRYPTED_AES_KEY_LEN];

    unsafe {
        ffi::playfair_decrypt(
            message.as_ptr(),
            encrypted_aes_key.as_ptr(),
            aes.as_mut_ptr(),
        );
    }

    aes
}

#[cfg(test)]
mod tests {
    //! The test and its data are taken from [aireplay2-receiver](https://github.com/openairplay/airplay2-receiver).

    use base64::Engine;

    use super::decrypt_key;

    const AES_KEY_BASE64: &[&str] = &[
        "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",
    ];

    const MESSAGE3_HEX: &[&str]= &[
        "46504c590301030000000098008f1a9ca548fdd57560a52926ff399f2eb154d0a7a0fffc997f58e27e00499eb9f310110d019e550e328047aea54308ab71b647041406878af96e06cf74127ae35941dceb58931b5543b39903f9f76a376248ee52e3656b561e1c1a0106ec6608df0ab4f2df528e65db6d622d3892d5b49c6c025606a574f19ebea7d93500bdd69db23333f22edcb3ccf7a6acde7389f2facabfa61b0b50",
        "46504c590301030000000098018f1a9c144a77fb15383f69cf6ba6ae3504582d489a121c644dac40bfb382388d758b294841cbe51bb4feea983f9157a1fb2e57765d1bfc7262053ca6f75c90c82794a43b8d844637aa018c28619a43da6727c7faf81b911a92f317d6ac7a3e1a7b923fe693cffb37317159be8904556862d81ed4f794957dc330b7e681e5a0067f596a0f3f936dd761f5afa2d69ab77938328bb1fcd92e",
        "46504c590301030000000098028f1a9c049ae04d1691802802c75b3ced9204acbeb5482b582f4faf3c008d7dd3675a37967e3bee3079bec95b8bfeea69aaec8233c7ab3b7df283e8f9a50b8ecdcc53e3ee2e5ee1d78421378fdf8cfa1e1c04995d3c6f14b47e9487f3458cc6e4727fe1e3ad2b1db60afdb590c14da5011404ab0972c15ab14ad6a71ebd8cab10098bc1b1822b14dd3e496fe13bfcca8fd399eee52581d4",
        "46504c590301030000000098038f1a9c63b126a2325158ff9ba6e5d9996b33cf4fda80c7bb3492ee15790fa57b5cdf0129fc3d29a2f6fb66ef4494aca148ddff2b726df69de41caf7292a3a960673699430df2eb5742ab52e61d5ef576ffbfb960686c14976dce6c66b07b491b94a41b445152a4449221fc704645fe1d5af60f446c3dbf1030580eb2c0002e387053fc9783d04dfba861da3f1444664737bcc2d82f4a5c",
        "46504c590301030000000098018f1a9c29495ae9d6e3648a357409f2209a1c158c1f39ce8cc85e01071ac7cd9faab49c49c90d5d2eb9bab2848ac6712eaea5df12de1cda35bfa1b9a6d465f231e3588b0555d5bee46ba40f903988e6ac945d7718564de23eca6aa527edde0600568431349243c9569bf047c813675d0c19caca0ca68bd6a7e64e33616b204f2e67459f3494c462549f56d12391126a09b5b5a41486c9b0",
        "46504c590301030000000098028f1a9cdf8b2485fcee11ff33f0ee5e2a5a4323641d631e4935f625312c5cf233455e075e5e469d7bc056c7ea127c0dcd14b9acbbf9d36b82f729df8b9436ddc06a617d1d1cb2fb1ed1d6b8a2f3e75a3e946cb96db3fc87397a7c456c60d2db150692e1a2f5aa22d032800cd71f45e0b40013f6d063f29d2ee244ab1a5b51123eee9bce5a6009add24a851ab7c16c8510e9afffef52649a",
        "46504c590301030000000098018f1a9c6f35086eb11e63e57957b8668008e91a684f3c7fd21d7dabfbc11e463bba37fc25c0e9e003149894e4a2c31a42caa8bbc0ba0543fc72d195268402e441e0b2953118deaa0fc7361942d145e0359065fcf2f81e325295090b4c548b370c112f5f112e3854b36fdd2924ddf50e9d808b6b8d0dd6e424524de34f038db6559e5f3faeb8ffbf9e4806bfbab8b8015c576194429c7aec",
        "46504c590301030000000098018f1a9c67ba6fab74a6e41137ec13540d661f85fbc50f8413dfdddec9af2e68c3136257165a822d3b28f24f7c51acd1b2edd38e7f6abdab966924f4104cfd138a52e4c860a7c5b72762dd386b77a6476acd6f3128c2e44b75900bffb67afeb9ba9401b83a56a1e7b2ad032bd854a054e38738bb8a163002f6670032b55a501c3027c21fc8d42b5b7951f1ea6456a7873a22fe2e184e381b",
        "46504c590301030000000098018f1a9c18881cb5d47f6bd3171037556ad2e7e3d7c70d5cb1a3dca450f4358668417bf45429dc6c57ece1061b716715ae856736b4904f729827a68650e59f07cf43cc2c707d374a66fa4d1de635a4cd8d4ca87ea8fb672b4b493b4c5700a7df865d46a2ed3b0714d97b20088b6c605caa47bc8d45ec4a6f90eb1f731a0be9744a02777ecded8131e667db9bab7dc84c2a8879855638f64f",
        "46504c590301030000000098008f1a9ca66352fcad37cf98cff8be95eb67fc07d78ea7cdec69285f30db7728254374b0b4875d37837eca8b9fc55663ddb55ccb2cb9800f5537d78f73a542d51017effd0470f32171cbf819680c7b2c695f087d20cb26c1e551b4449569ff09ffa89ff46e2db5cbe7d81df14ab91ada7cac520106a3569cd43a7985bbcc28c637066e6e4b942379f744eed1e661203e0e06a40edf4ffb45",
        "46504c590301030000000098018f1a9c9dd7e68de23b32f3f169be4de7f2de6919ce0ae597e910c31bf03467137d7389026ca094af18bef5374380560136a511e67847b7afe2d6dedd3723832c5c6ff3dd90d7a06d06709cb1861c1129940a6bd37c2299eab023e64dc56b758a372d913d70aafe044a6525add22422eca069f68ec0bf53dc780541c65844b9843a24ec567aa3947831c6d0ea91989d39fb97e40f058a48",
        "46504c590301030000000098028f1a9c384ed75e69fadd495b3751512e839c2b812e93fb59b18b2cf9ed02453e636c54035b4b75ac2852d284ebcdd74d7fdfdd94063bdf7a8fe3f3371c2057c5e2cd0ca288412bec984256356fdaa76c81fdbc7d48da7e234f4cd4a7008d48f274529e5aae6df5c36c93231b8cb3bcc9d0f7e1b1135b732c9c9a7b58b542193c831ee82a1cbaad029be1bafd052e71341891e3c9aaddea",
        "46504c590301030000000098038f1a9c739127c444b13c7982c64ef5b5152d76d297a22816092cbb9f73c38284696e52c79222b83ed116035a5b052f9c0fe12f3666825305cd5392e95e5b45b06557b829dee8b4bbdd2b10092ddcf0bf010c03ad0cc6d2406b2e2253e106358fe0c8edcb8f040506bd3ba17e1be18efa3576d9ccdb759bf27114b031393d3f79d6961f2b43b6a9261d052b2c00bb79b8dac81a90bdb8b1",
        "46504c590301030000000098008f1a9c8eea57769ca08100c4c4a42854fbd389b319597801bbee7954f64e2cccfe781733e65f73fa1a7a0cf0c1224160ff5c25e49969f3a81efe2d0f068407e242dc23ef4022a600f5db27b080300602735fb7cfad35c13f6f06d74aa432b359220483973ca8e4eb1fd00c1f4eca462a3ee83a2461ebb695eefd1c590e641055d65ebe4b2e31facb41543f209940a851a91a823799cd04",
        "46504c590301030000000098018f1a9cfa235805d532ed9e3e0372715ebc0cb9764efe4d6f34cc1fa3f596e1469a59c9f5550d94a2b6e0740ce59d977f7dd48e29da74708370dc98362d2bd8e697521261f0c35d681265ceb23919faa5384c50b323a03e8a84ec289304972740e252449cdb06b5a0b727aa7eaa0a81c2510005cb2526c878829207b7d43d61947a5e54e6efbe4ac6b5fab3814a745e023eb48bb10ac77f",
        "46504c590301030000000098028f1a9c6c0f5700e381dc018d364eba80117b639fb31317c0030575d53b854d4af4b279983058141fc67e82acd74ace7fc976dd8084609a9bb2ddb86e731c609dca2afba038b971be4282b7bf62a96a102560240a413d333dded20b14b723e7e8820bbc90ec630531da1c77ca12583587e43c5abc80faf0c379aee4d57c1e317eba6b4f4beaf85460e4bea20ae4c8e4c25cc7cac61ca3dd",
        "46504c590301030000000098028f1a9c56f41aacc969a8b8deb34d81ec5d68649ab6678280ebed1e0240419761b9dee3b261d970065b68ace990fc44fab231783284351f0cc390e3297e575455cc093bd432e107bee3687807258710395bdc3f8db86103add6cb690a581f345b26607bc94710d62e0c3c66b31be2437b107fb54bf7b646276a7f6f456338a163a6d2eee6503e9943646363a3d6d91f1fff7001f240773b",
        "46504c590301030000000098008f1a9c170c310e806087794119777cca4f91e26e6f13e88c1ddce46d371a5ab06d2c0acf8b7a2e2d26dde43a753c2af8ad5560ca56a9742be93ed5aee23c0c5c6a936e08ad6417e35b6798defcbf3fd86fb369f758ff17983d0d2732cda58ce7ed5cded4fb5c4decb3ce4c7aa112022ff8bc95af6f040848d514ae953f522635c533d7c02b823817f545039c0456249e70e2a95a48fd07",
        "46504c590301030000000098018f1a9c0a1cb485933f9a1ece342077bfdb8224e1303b9ece9d83f8e289e03952be49eaf38ad9382983272f6e3bc261263d63e127a831482fe9ff1c1b35670a30c78f2ecfab1b8e9bd0da8c8eac626595858d8e9557a293fdf8b3ed0cd177656f2bf359847a8ac3f062dd682bbbbb864b9e4d572cb0eeed5321eb4c70d67824bc91f7797fa5722d951e85d8a1c7b8d7e94dfa85b0ec9cbf",
        "46504c590301030000000098028f1a9c9322788842e7463d130a4326ee591b3b56240f39380f482853211950ea4a3c6e6284e19149cf66be49814ee3e9bc79dccda4f96b6c6e620a4aff35a0f6dc6d76b6ca3923da667b91409f7251010a8ab1de310d37242028ce711903bd777ecd2c7fe6dfb992623e842db6a61a2d63b432558fb68244e8a629c4a4e281623da7d8028c47f85565b23349e3af865ea192204d94deea",
    ];

    const EXPECTED_KEYS: &[&str] = &[
        "0496a612172f41e0fd71912acc33fc54",
        "1512816fbdbe4856570931c3ec7d0e3d",
        "bc3b888f894276c7dd43c3739a08947c",
        "63082ff46b87d13d8211e048a696d914",
        "2832d44e4dd9f7d3a806562ccd733aba",
        "afb47b92476582fa19d5f2bacbb289b9",
        "eb096a94a2bd2b2480d470c4a562d9bf",
        "b7d8a066bcaed0448601472797a0feb0",
        "97935000cfb79f384d996104362c2045",
        "70e5d54a36a00f4878eba13604a5f3e5",
        "f98a3079b81de696cecca09f18f7ed23",
        "7777a0fc7c9b93d230c4f8795c3f3523",
        "1b21477ed94e634da07769b696fc3e51",
        "671a855130491be5a6c81b2a28a0c466",
        "d6364d68cf353b9f74bb9488c7313b73",
        "6031fc351232683d60b01cd2108e227e",
        "7244f743459404794995317c848e91d9",
        "dae8ac01942b80da0712e2873400acc4",
        "546d85c2b476236e42f2b4906a438c95",
        "7245a3cfeb08e958d4985a19255410ea",
    ];

    #[test]
    fn test_fairplay3_decrypt() {
        for ((message, aeskey), expected) in MESSAGE3_HEX
            .iter()
            .zip(AES_KEY_BASE64)
            .zip(EXPECTED_KEYS.iter())
        {
            let message = hex::decode(message).expect("invalid message hex");
            let aeskey = base64::prelude::BASE64_STANDARD
                .decode(aeskey)
                .expect("invalid base64 for aes key");
            assert_eq!(164, message.len());
            assert_eq!(72, aeskey.len());
            assert_eq!(expected, &hex::encode(decrypt_key(message, aeskey)));
        }
    }
}
