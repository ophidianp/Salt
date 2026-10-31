//! Plays the client side of "Sign in with Minecraft" against a running
//! `verify_listen`, all the way through encryption, and prints the (decrypted)
//! kick message. Without a real Mojang join it must be the "couldn't confirm"
//! message, which proves the handshake and AES stream work.

use aes::cipher::{AsyncStreamCipher, KeyIvInit};
use rsa::pkcs8::DecodePublicKey;
use rsa::{Pkcs1v15Encrypt, RsaPublicKey};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

fn varint(out: &mut Vec<u8>, v: i32) {
    let mut v = v as u32;
    loop {
        if v & !0x7F == 0 {
            out.push(v as u8);
            return;
        }
        out.push((v & 0x7F | 0x80) as u8);
        v >>= 7;
    }
}

fn read_varint(b: &[u8], pos: &mut usize) -> i32 {
    let mut v = 0;
    for i in 0..5 {
        let x = b[*pos];
        *pos += 1;
        v |= ((x & 0x7F) as i32) << (7 * i);
        if x & 0x80 == 0 {
            break;
        }
    }
    v
}

async fn send(s: &mut TcpStream, id: i32, body: &[u8]) {
    let mut inner = Vec::new();
    varint(&mut inner, id);
    inner.extend_from_slice(body);
    let mut out = Vec::new();
    varint(&mut out, inner.len() as i32);
    out.extend_from_slice(&inner);
    s.write_all(&out).await.unwrap();
}

async fn recv_raw(s: &mut TcpStream) -> Vec<u8> {
    let mut buf = vec![0; 8192];
    let n = s.read(&mut buf).await.unwrap();
    buf.truncate(n);
    buf
}

#[tokio::main]
async fn main() {
    let mut s = TcpStream::connect("127.0.0.1:25599").await.unwrap();
    let mut hs = Vec::new();
    varint(&mut hs, 777);
    varint(&mut hs, 9);
    hs.extend_from_slice(b"localhost");
    hs.extend_from_slice(&25599u16.to_be_bytes());
    varint(&mut hs, 2);
    send(&mut s, 0x00, &hs).await;
    let mut ls = Vec::new();
    varint(&mut ls, 4);
    ls.extend_from_slice(b"Test");
    ls.extend_from_slice(&[0; 16]);
    send(&mut s, 0x00, &ls).await;

    // Encryption request.
    let p = recv_raw(&mut s).await;
    let mut pos = 0;
    let _len = read_varint(&p, &mut pos);
    assert_eq!(read_varint(&p, &mut pos), 0x01, "expected encryption request");
    let sid_len = read_varint(&p, &mut pos) as usize;
    pos += sid_len;
    let key_len = read_varint(&p, &mut pos) as usize;
    let der = p[pos..pos + key_len].to_vec();
    pos += key_len;
    let tok_len = read_varint(&p, &mut pos) as usize;
    let token = p[pos..pos + tok_len].to_vec();
    pos += tok_len;
    println!("encryption request ok: {}-byte key, {}-byte token, should_authenticate={}", key_len, tok_len, p[pos]);

    let key = RsaPublicKey::from_public_key_der(&der).expect("server key must be X.509 DER");
    let secret: [u8; 16] = rand::random();
    let mut rng = rand::thread_rng();
    let mut resp = Vec::new();
    let es = key.encrypt(&mut rng, Pkcs1v15Encrypt, &secret).unwrap();
    varint(&mut resp, es.len() as i32);
    resp.extend_from_slice(&es);
    let et = key.encrypt(&mut rng, Pkcs1v15Encrypt, &token).unwrap();
    varint(&mut resp, et.len() as i32);
    resp.extend_from_slice(&et);
    send(&mut s, 0x01, &resp).await;

    // Encrypted login disconnect.
    let mut enc = recv_raw(&mut s).await;
    cfb8::Decryptor::<aes::Aes128>::new_from_slices(&secret, &secret).unwrap().decrypt(&mut enc);
    let mut pos = 0;
    let _len = read_varint(&enc, &mut pos);
    let id = read_varint(&enc, &mut pos);
    let n = read_varint(&enc, &mut pos) as usize;
    println!("decrypted packet 0x{id:02x}: {}", String::from_utf8_lossy(&enc[pos..pos + n]));
}
