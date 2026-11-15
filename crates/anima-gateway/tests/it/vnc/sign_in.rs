//! The gateway signs in to a VNC device with each method, and takes a refusal as one.

use aes::cipher::{BlockCipherDecrypt as _, KeyInit as _};
use anima_gateway::handshake::Credential;
use anima_gateway::vnc::{Account, Method, SignInError, offered, sign_in};
use md5::{Digest as _, Md5};
use num_bigint::BigUint;
use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _, DuplexStream, duplex};

const ACCOUNT: Account<'static> = Account {
    username: "ada",
    password: "secret12",
};
/// The 1024-bit prime of RFC 2409 (the second Oakley group), in hexadecimal.
const PRIME: &str = "FFFFFFFFFFFFFFFFC90FDAA22168C234C4C6628B80DC1CD129024E088A67CC74\
                     020BBEA63B139B22514A08798E3404DDEF9519B3CD3A431B302B0A6DF25F1437\
                     4FE1356D6D51C245E485B576625E7EC6F44C42E9A637ED6B0BFF5CB6F406B7ED\
                     EE386BFB5A899FA5AE9F24117C4B1FE649286651ECE65381FFFFFFFFFFFFFFFF";
/// The bytes of the prime and of the public numbers.
const KEY_BYTES: usize = 128;

fn prime() -> BigUint {
    BigUint::parse_bytes(PRIME.as_bytes(), 16).expect("a number")
}

/// `number` in `KEY_BYTES` bytes.
fn key_bytes(number: &BigUint) -> Vec<u8> {
    let bytes = number.to_bytes_be();
    [vec![0; KEY_BYTES - bytes.len()], bytes].concat()
}

/// What a device sends to open Apple's method: generator 2, the prime, its public number.
fn apple_opening(public: &BigUint) -> Vec<u8> {
    [vec![0, 2, 0, 128], key_bytes(&prime()), key_bytes(public)].concat()
}

/// Not random, so each run is the same.
fn counting(buffer: &mut [u8]) {
    for (at, byte) in buffer.iter_mut().enumerate() {
        *byte = u8::try_from(at % 251).expect("under 251") + 1;
    }
}

/// A device that offers `types`, and the gateway's end of its connection.
async fn device(types: &[u8]) -> (DuplexStream, DuplexStream) {
    let (gateway, mut device) = duplex(1 << 16);
    device.write_all(b"RFB 003.889\n").await.expect("banner");
    device
        .write_all(&[u8::try_from(types.len()).expect("few")])
        .await
        .expect("count");
    device.write_all(types).await.expect("types");
    (gateway, device)
}

/// The device's end after the sign-in method: the result, then `ServerInit` for 800 x 600.
async fn accept(device: &mut DuplexStream, result: u32) {
    device
        .write_all(&result.to_be_bytes())
        .await
        .expect("result");
    device.write_all(&[3, 32, 2, 88]).await.expect("size");
    device.write_all(&[0; 16]).await.expect("pixel format");
    device
        .write_all(&[0, 0, 0, 4, b'd', b'e', b's', b'k'])
        .await
        .expect("name");
}

/// What the gateway sent so far, after the 12 bytes of its version.
async fn sent(device: &mut DuplexStream, count: usize) -> Vec<u8> {
    let mut bytes = vec![0; 12 + count];
    device
        .read_exact(&mut bytes)
        .await
        .expect("the gateway's bytes");
    assert_eq!(&bytes[..12], b"RFB 003.008\n");
    bytes.split_off(12)
}

#[tokio::test]
async fn a_device_without_a_sign_in_opens_and_gets_the_gateways_pixel_format() {
    let (mut gateway, mut device) = device(&[1]).await;
    let method = offered(&mut gateway)
        .await
        .expect("offered")
        .method(false, false)
        .expect("a method");
    assert_eq!((method, method.needs()), (Method::None, [].as_slice()));
    accept(&mut device, 0).await;
    let size = sign_in(&mut gateway, method, ACCOUNT, &mut counting)
        .await
        .expect("signed in");
    assert_eq!(size, (800, 600));
    // The chosen type, ClientInit (shared), SetPixelFormat, SetEncodings.
    let bytes = sent(&mut device, 2 + 20 + 24).await;
    assert_eq!(&bytes[..2], [1, 1]);
    assert_eq!(
        &bytes[2..22],
        [
            0, 0, 0, 0, 32, 24, 0, 1, 0, 255, 0, 255, 0, 255, 16, 8, 0, 0, 0, 0
        ]
    );
    assert_eq!(&bytes[22..26], [2, 0, 0, 5]);
}

#[tokio::test]
async fn a_vnc_password_gives_the_answer_openssl_gives() {
    let (mut gateway, mut device) = device(&[2]).await;
    let method = offered(&mut gateway)
        .await
        .expect("offered")
        .method(false, true)
        .expect("a method");
    assert_eq!(method.needs(), [Credential::Password]);
    let challenge: [u8; 16] = std::array::from_fn(|at| u8::try_from(at).expect("under 16"));
    device.write_all(&challenge).await.expect("challenge");
    accept(&mut device, 0).await;
    sign_in(&mut gateway, method, ACCOUNT, &mut counting)
        .await
        .expect("signed in");
    // DES-ECB of the challenge under "secret12" with each key byte bit-reversed, from
    // `openssl enc -des-ecb -K cea6c64ea62e8c4c -nopad`.
    let expected = [
        0xad, 0xcd, 0x99, 0x7f, 0x8e, 0x16, 0xfe, 0xe5, 0x75, 0xe9, 0x73, 0xf9, 0x3c, 0x2b, 0x62,
        0xb4,
    ];
    assert_eq!(sent(&mut device, 17).await[1..], expected);
}

#[tokio::test]
async fn apples_method_carries_the_account_to_a_device_that_can_read_it() {
    let (mut gateway, mut device) = device(&[30, 33, 36, 35]).await;
    let method = offered(&mut gateway)
        .await
        .expect("offered")
        .method(true, true)
        .expect("a method");
    assert_eq!(
        (method, method.needs()),
        (
            Method::Apple,
            [Credential::Username, Credential::Password].as_slice()
        )
    );
    // The device's side of the key exchange.
    let device_private = BigUint::from(0x1234_5678_9abc_def0_u64);
    let device_public = BigUint::from(2u8).modpow(&device_private, &prime());
    device
        .write_all(&apple_opening(&device_public))
        .await
        .expect("the opening");
    accept(&mut device, 0).await;
    sign_in(&mut gateway, method, ACCOUNT, &mut counting)
        .await
        .expect("signed in");

    let bytes = sent(&mut device, 1 + 128 + KEY_BYTES).await;
    let (fields, gateway_public) = bytes[1..].split_at(128);
    let secret = BigUint::from_bytes_be(gateway_public).modpow(&device_private, &prime());
    let key: [u8; 16] = Md5::digest(key_bytes(&secret)).into();
    let cipher = aes::Aes128::new(&key.into());
    let mut fields: [u8; 128] = fields.try_into().expect("128 bytes");
    let (blocks, _) = fields.as_chunks_mut::<16>();
    for block in blocks {
        cipher.decrypt_block(block.into());
    }
    assert_eq!(&fields[..4], b"ada\0");
    assert_eq!(&fields[64..73], b"secret12\0");
}

#[tokio::test]
async fn the_method_follows_what_the_hub_has() {
    let (mut gateway, _device) = device(&[30, 2, 1]).await;
    let all = offered(&mut gateway).await.expect("offered");
    assert_eq!(all.method(true, true).expect("a method"), Method::Apple);
    assert_eq!(
        all.method(false, true).expect("a method"),
        Method::VncPassword
    );
    assert_eq!(
        all.method(false, false).expect("a method"),
        Method::VncPassword
    );
}

#[tokio::test]
async fn an_account_is_never_sent_by_a_weaker_method_than_the_hub_expects() {
    // The account's password does not go out as a VNC password.
    let (mut gateway, _device) = device(&[2, 1]).await;
    let no_apple = offered(&mut gateway).await.expect("offered");
    assert!(matches!(
        no_apple.method(true, true),
        Err(SignInError::NoMethod)
    ));
    // A device that asks for no sign-in, while the hub has a password for it, is refused.
    let (mut gateway, _device) = device(&[1]).await;
    let open = offered(&mut gateway).await.expect("offered");
    assert!(matches!(
        open.method(false, true),
        Err(SignInError::NoMethod)
    ));
    assert!(matches!(
        open.method(true, false),
        Err(SignInError::NoMethod)
    ));
    assert_eq!(open.method(false, false).expect("a method"), Method::None);
    // Methods the gateway does not have.
    let (mut gateway, _device) = device(&[19, 33]).await;
    let unknown = offered(&mut gateway).await.expect("offered");
    assert!(matches!(
        unknown.method(false, false),
        Err(SignInError::NoMethod)
    ));
}

#[tokio::test]
async fn a_refused_account_is_a_refusal_and_not_a_fault() {
    for reason in [vec![0, 0, 0, 3, b'n', b'o', b'!'], vec![]] {
        let (mut gateway, mut device) = device(&[2]).await;
        let method = offered(&mut gateway)
            .await
            .expect("offered")
            .method(false, true)
            .expect("a method");
        device.write_all(&[0; 16]).await.expect("challenge");
        device.write_all(&[0, 0, 0, 1]).await.expect("failed");
        device.write_all(&reason).await.expect("reason");
        device.shutdown().await.expect("closed");
        let refused = sign_in(&mut gateway, method, ACCOUNT, &mut counting).await;
        assert!(matches!(refused, Err(SignInError::Refused)), "{refused:?}");
    }
}

#[tokio::test]
async fn a_peer_that_is_not_a_vnc_server_of_version_3_8_is_refused() {
    for banner in [
        &b"RFB 003.003\n"[..],
        b"RFB 003.007\n",
        b"SSH-2.0-Open",
        b"RFB 003.8\n\n\n",
    ] {
        let (mut gateway, mut device) = duplex(64);
        device.write_all(banner).await.expect("banner");
        assert!(
            matches!(offered(&mut gateway).await, Err(SignInError::NotVnc)),
            "{banner:?}"
        );
    }
    let (mut gateway, mut device) = duplex(64);
    device
        .write_all(b"RFB 003.008\n\0\0\0\0\x04busy")
        .await
        .expect("refusal");
    assert!(matches!(
        offered(&mut gateway).await,
        Err(SignInError::Turned)
    ));
}

/// The gateway's answer to a device that opens Apple's method with `opening`.
async fn apple_answer_to(
    opening: &[u8],
    account: Account<'_>,
) -> Result<(usize, usize), SignInError> {
    let (mut gateway, mut device) = device(&[30]).await;
    let method = offered(&mut gateway)
        .await
        .expect("offered")
        .method(true, true)
        .expect("a method");
    device.write_all(opening).await.expect("the opening");
    sign_in(&mut gateway, method, account, &mut counting).await
}

#[tokio::test]
async fn key_exchange_numbers_that_would_show_the_password_are_refused() {
    let five = BigUint::from(5u8);
    let bad = |opening: Vec<u8>| async move {
        matches!(
            apple_answer_to(&opening, ACCOUNT).await,
            Err(SignInError::BadNumbers)
        )
    };
    // A key of one byte, or longer than the limit: nothing more is read.
    assert!(bad(vec![0, 2, 0, 1, 251, 5]).await);
    assert!(bad(vec![0, 2, 0xff, 0xff]).await);
    // A public number of 1: the secret would be 1 for anyone.
    assert!(bad(apple_opening(&BigUint::from(1u8))).await);
    // A small prime written in 128 bytes, and an even number in place of a prime.
    let small = [
        vec![0, 2, 0, 128],
        key_bytes(&BigUint::from(251u8)),
        key_bytes(&five),
    ]
    .concat();
    assert!(bad(small).await);
    let even = [
        vec![0, 2, 0, 128],
        key_bytes(&(prime() - 1u8)),
        key_bytes(&five),
    ]
    .concat();
    assert!(bad(even).await);
    // A generator of 1.
    let mut generator_one = apple_opening(&five);
    generator_one[1] = 1;
    assert!(bad(generator_one).await);
}

#[tokio::test]
async fn an_account_name_longer_than_the_method_carries_is_refused() {
    let long = "n".repeat(64);
    let account = Account {
        username: &long,
        password: "p",
    };
    let answer = apple_answer_to(&apple_opening(&BigUint::from(5u8)), account).await;
    assert!(matches!(answer, Err(SignInError::TooLong)));
}

#[tokio::test]
async fn a_screen_with_no_width_is_refused() {
    let (mut gateway, mut device) = self::device(&[1]).await;
    offered(&mut gateway).await.expect("offered");
    device
        .write_all(&[0, 0, 0, 0, 0, 0, 2, 88])
        .await
        .expect("result and size");
    device
        .write_all(&[0; 20])
        .await
        .expect("pixel format and name");
    assert!(matches!(
        sign_in(&mut gateway, Method::None, ACCOUNT, &mut counting).await,
        Err(SignInError::BadNumbers)
    ));
}
