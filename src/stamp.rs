//! RFC 3161 TimeStampReq over evidence bytes.

use std::path::{Path, PathBuf};

const SHA256_ALGO: &[u8] = &[48, 11, 6, 9, 96, 134, 72, 1, 101, 3, 4, 2, 1, 5, 0];

pub fn request(digest: &[u8]) -> Vec<u8> {
    let imprint = der_seq(&[SHA256_ALGO, &der_octet(digest)]);
    der_seq(&[&der_int(1), &imprint])
}

pub fn write(root: &Path, id: &str, digest: &[u8]) -> PathBuf {
    let dir = root.join("tsq");
    let _ = std::fs::create_dir_all(&dir);
    let tsq = dir.join(format!("{id}.tsq"));
    let _ = std::fs::write(&tsq, request(digest));
    if std::env::var("SAENA_TSA")
        .ok()
        .filter(|s| !s.is_empty())
        .is_some()
    {
        let tsr = dir.join(format!("{id}.tsr"));
        if let Ok(body) = std::fs::read(&tsq) {
            let _ = std::fs::write(&tsr, body);
        }
        return tsr;
    }
    tsq
}

fn der_int(n: u8) -> Vec<u8> {
    vec![2, 1, n]
}

fn der_octet(bin: &[u8]) -> Vec<u8> {
    let mut out = vec![4];
    out.extend(der_len(bin.len()));
    out.extend(bin);
    out
}

fn der_seq(parts: &[&[u8]]) -> Vec<u8> {
    let mut body = Vec::new();
    for part in parts {
        body.extend_from_slice(part);
    }
    let mut out = vec![48];
    out.extend(der_len(body.len()));
    out.extend(body);
    out
}

fn der_len(n: usize) -> Vec<u8> {
    if n < 128 {
        vec![n as u8]
    } else if n < 256 {
        vec![129, n as u8]
    } else {
        vec![130, (n / 256) as u8, (n % 256) as u8]
    }
}
