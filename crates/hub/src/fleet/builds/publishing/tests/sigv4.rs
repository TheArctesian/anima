//! The signer against the worked examples of the AWS documentation (D-24): "Examples of how to
//! derive a signing key for Signature Version 4", and the Amazon S3 examples "Signature
//! Calculations for the Authorization Header: Transferring Payload in a Single Chunk" (GET
//! Object, PUT Object) and "Authenticating Requests: Using Query Parameters" (GET Object).

use chrono::{DateTime, Utc};

use crate::fleet::builds::publishing::sigv4::{
    Key, Request, encode, hex, presign, sha256_hex, signed_headers, signing_key,
};

const EMPTY_SHA256: &str = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

fn example_key() -> Key<'static> {
    Key {
        id: "AKIAIOSFODNN7EXAMPLE",
        secret: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY",
        region: "us-east-1",
        service: "s3",
    }
}

fn may_24_2013() -> DateTime<Utc> {
    "2013-05-24T00:00:00Z".parse().unwrap()
}

fn request(method: &'static str, path: &'static str, headers: &[(&str, &str)]) -> Request<'static> {
    Request {
        method,
        host: "examplebucket.s3.amazonaws.com",
        path,
        query: Vec::new(),
        headers: headers
            .iter()
            .map(|(name, value)| ((*name).to_owned(), (*value).to_owned()))
            .collect(),
    }
}

fn authorization(headers: &[(String, String)]) -> &str {
    &headers
        .iter()
        .find(|(name, _)| name == "authorization")
        .unwrap()
        .1
}

#[test]
fn d_24_the_signing_key_is_the_aws_example() {
    let key = signing_key(
        "wJalrXUtnFEMI/K7MDENG+bPxRfiCYEXAMPLEKEY",
        "20120215",
        "us-east-1",
        "iam",
    );
    assert_eq!(
        hex(&*key),
        "f4780e2d9f65fa895f9c67b32ce1baf0b0d8a43505a000a1a9e090d414db404d"
    );
}

#[test]
fn d_24_a_presigned_get_is_the_aws_example() {
    let query = presign(
        &example_key(),
        request("GET", "/test.txt", &[]),
        may_24_2013(),
        86_400,
    );
    assert_eq!(
        query,
        "X-Amz-Algorithm=AWS4-HMAC-SHA256\
         &X-Amz-Credential=AKIAIOSFODNN7EXAMPLE%2F20130524%2Fus-east-1%2Fs3%2Faws4_request\
         &X-Amz-Date=20130524T000000Z&X-Amz-Expires=86400&X-Amz-SignedHeaders=host\
         &X-Amz-Signature=aeeed9bbccd4d02ee5c0109b86d86835f995330da4c265957d157751f604d404"
    );
}

#[test]
fn d_24_a_signed_get_with_a_range_is_the_aws_example() {
    let headers = signed_headers(
        &example_key(),
        request("GET", "/test.txt", &[("Range", "bytes=0-9")]),
        EMPTY_SHA256,
        may_24_2013(),
    );
    assert_eq!(
        authorization(&headers),
        "AWS4-HMAC-SHA256 \
         Credential=AKIAIOSFODNN7EXAMPLE/20130524/us-east-1/s3/aws4_request,\
         SignedHeaders=host;range;x-amz-content-sha256;x-amz-date,\
         Signature=f0e8bdb87c964420e857bd35b5d6ed310bd44f0170aba48dd91039c6036bdb41"
    );
}

#[test]
fn d_24_a_signed_put_with_an_encoded_path_is_the_aws_example() {
    let body = b"Welcome to Amazon S3.";
    let payload = sha256_hex(body);
    assert_eq!(
        payload,
        "44ce7dd67c959e0d3524ffac1771dfbba87d2b6b4b4e99e42034a8b803f8b072"
    );
    let headers = signed_headers(
        &example_key(),
        request(
            "PUT",
            "/test$file.text",
            &[
                ("Date", "Fri, 24 May 2013 00:00:00 GMT"),
                ("x-amz-storage-class", "REDUCED_REDUNDANCY"),
            ],
        ),
        &payload,
        may_24_2013(),
    );
    assert_eq!(
        authorization(&headers),
        "AWS4-HMAC-SHA256 \
         Credential=AKIAIOSFODNN7EXAMPLE/20130524/us-east-1/s3/aws4_request,\
         SignedHeaders=date;host;x-amz-content-sha256;x-amz-date;x-amz-storage-class,\
         Signature=98ad721746da40c64f1a55b78f14c238d841ea1380cd77a1b5971af0ece108bd"
    );
}

#[test]
fn d_24_encoding_keeps_only_unreserved_bytes_and_path_slashes() {
    assert_eq!(encode("a/b c$~._-", true), "a/b%20c%24~._-");
    assert_eq!(encode("a/b", false), "a%2Fb");
    assert_eq!(encode("é", false), "%C3%A9");
}
