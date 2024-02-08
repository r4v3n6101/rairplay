use axum::response::{IntoResponse, Response};
use bytes::Bytes;
use hyper::StatusCode;

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

pub async fn handler(body: Bytes) -> Response {
    // Version
    match body.get(4) {
        Some(3) => {}
        _ => return (StatusCode::BAD_REQUEST, "Unknown version").into_response(),
    }

    // Type
    match body.get(5) {
        Some(1) => {
            // Seq
            match body.get(6) {
                Some(1) => match body.get(14) {
                    Some(mode @ 0..=4) => MESSAGES[*mode as usize].into_response(),
                    _ => (StatusCode::BAD_REQUEST, "Unknown mode for M1").into_response(),
                },
                Some(3) => {
                    let mut output = [0; FP_HEADER.len() + 20];
                    output[..FP_HEADER.len()].copy_from_slice(FP_HEADER);
                    match body.get(body.len() - 20..) {
                        Some(suffix) => {
                            output[FP_HEADER.len()..].copy_from_slice(suffix);
                            output.into_response()
                        }
                        _ => (StatusCode::BAD_REQUEST, "Insufficient request").into_response(),
                    }
                }
                _ => (StatusCode::BAD_REQUEST, "Unknown seq").into_response(),
            }
        }
        _ => (StatusCode::BAD_REQUEST, "Unknown message type").into_response(),
    }
}
