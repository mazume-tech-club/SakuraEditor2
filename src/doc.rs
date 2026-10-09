//! ファイル読み書きと文字コード判定。エディタ内部は常に UTF-8。

use windows_sys::Win32::Globalization::{MultiByteToWideChar, WideCharToMultiByte};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Encoding {
    Utf8,
    Utf8Bom,
    Sjis,
    Utf16Le,
    Utf16Be,
}

impl Encoding {
    pub fn from_label(s: &str) -> Option<Encoding> {
        [Encoding::Utf8, Encoding::Utf8Bom, Encoding::Sjis, Encoding::Utf16Le, Encoding::Utf16Be]
            .into_iter()
            .find(|e| e.label() == s)
    }

    pub fn label(self) -> &'static str {
        match self {
            Encoding::Utf8 => "UTF-8",
            Encoding::Utf8Bom => "UTF-8 BOM",
            Encoding::Sjis => "SJIS",
            Encoding::Utf16Le => "UTF-16LE",
            Encoding::Utf16Be => "UTF-16BE",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Eol {
    Crlf,
    Lf,
    Cr,
}

impl Eol {
    pub fn from_label(s: &str) -> Option<Eol> {
        [Eol::Crlf, Eol::Lf, Eol::Cr].into_iter().find(|e| e.label() == s)
    }

    pub fn label(self) -> &'static str {
        match self {
            Eol::Crlf => "CRLF",
            Eol::Lf => "LF",
            Eol::Cr => "CR",
        }
    }
}

const CP932: u32 = 932;

/// バイト列から文字コードを判定し、UTF-8 に変換する
pub fn decode(raw: Vec<u8>) -> (Vec<u8>, Encoding) {
    if raw.starts_with(&[0xEF, 0xBB, 0xBF]) {
        return (raw[3..].to_vec(), Encoding::Utf8Bom);
    }
    if raw.starts_with(&[0xFF, 0xFE]) {
        let u: Vec<u16> = raw[2..].chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
        return (String::from_utf16_lossy(&u).into_bytes(), Encoding::Utf16Le);
    }
    if raw.starts_with(&[0xFE, 0xFF]) {
        let u: Vec<u16> = raw[2..].chunks_exact(2).map(|c| u16::from_be_bytes([c[0], c[1]])).collect();
        return (String::from_utf16_lossy(&u).into_bytes(), Encoding::Utf16Be);
    }
    if std::str::from_utf8(&raw).is_ok() {
        return (raw, Encoding::Utf8);
    }
    // 末尾が途中で切れた UTF-8 (tail 中のログなど) も UTF-8 とみなす
    if let Err(e) = std::str::from_utf8(&raw) {
        if e.error_len().is_none() && raw.len() - e.valid_up_to() < 4 {
            return (raw, Encoding::Utf8);
        }
    }
    (sjis_to_utf8(&raw), Encoding::Sjis)
}

/// 追記分のデコード（tail 用）。文字コードは既知とする。
pub fn decode_chunk(raw: &[u8], enc: Encoding) -> Vec<u8> {
    match enc {
        Encoding::Utf8 | Encoding::Utf8Bom => raw.to_vec(),
        Encoding::Sjis => sjis_to_utf8(raw),
        Encoding::Utf16Le => {
            let u: Vec<u16> = raw.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
            String::from_utf16_lossy(&u).into_bytes()
        }
        Encoding::Utf16Be => {
            let u: Vec<u16> = raw.chunks_exact(2).map(|c| u16::from_be_bytes([c[0], c[1]])).collect();
            String::from_utf16_lossy(&u).into_bytes()
        }
    }
}

pub fn encode(utf8: &[u8], enc: Encoding) -> Vec<u8> {
    match enc {
        Encoding::Utf8 => utf8.to_vec(),
        Encoding::Utf8Bom => {
            let mut v = vec![0xEF, 0xBB, 0xBF];
            v.extend_from_slice(utf8);
            v
        }
        Encoding::Sjis => utf8_to_sjis(utf8),
        Encoding::Utf16Le => {
            let mut v = vec![0xFF, 0xFE];
            for c in String::from_utf8_lossy(utf8).encode_utf16() {
                v.extend_from_slice(&c.to_le_bytes());
            }
            v
        }
        Encoding::Utf16Be => {
            let mut v = vec![0xFE, 0xFF];
            for c in String::from_utf8_lossy(utf8).encode_utf16() {
                v.extend_from_slice(&c.to_be_bytes());
            }
            v
        }
    }
}

pub fn detect_eol(text: &[u8]) -> Eol {
    let probe = &text[..text.len().min(1 << 20)];
    match probe.iter().position(|&b| b == b'\n' || b == b'\r') {
        Some(i) if probe[i] == b'\r' => {
            if probe.get(i + 1) == Some(&b'\n') { Eol::Crlf } else { Eol::Cr }
        }
        Some(_) => Eol::Lf,
        None => Eol::Crlf,
    }
}

fn sjis_to_utf8(raw: &[u8]) -> Vec<u8> {
    if raw.is_empty() {
        return Vec::new();
    }
    unsafe {
        let n = MultiByteToWideChar(CP932, 0, raw.as_ptr(), raw.len() as i32, std::ptr::null_mut(), 0);
        let mut wbuf = vec![0u16; n as usize];
        MultiByteToWideChar(CP932, 0, raw.as_ptr(), raw.len() as i32, wbuf.as_mut_ptr(), n);
        String::from_utf16_lossy(&wbuf).into_bytes()
    }
}

fn utf8_to_sjis(utf8: &[u8]) -> Vec<u8> {
    if utf8.is_empty() {
        return Vec::new();
    }
    let wide: Vec<u16> = String::from_utf8_lossy(utf8).encode_utf16().collect();
    unsafe {
        let n = WideCharToMultiByte(
            CP932, 0, wide.as_ptr(), wide.len() as i32, std::ptr::null_mut(), 0,
            std::ptr::null(), std::ptr::null_mut(),
        );
        let mut out = vec![0u8; n as usize];
        WideCharToMultiByte(
            CP932, 0, wide.as_ptr(), wide.len() as i32, out.as_mut_ptr(), n,
            std::ptr::null(), std::ptr::null_mut(),
        );
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_utf8_and_bom() {
        assert_eq!(decode("あいう".as_bytes().to_vec()).1, Encoding::Utf8);
        let mut b = vec![0xEF, 0xBB, 0xBF];
        b.extend_from_slice(b"abc");
        let (t, e) = decode(b);
        assert_eq!(e, Encoding::Utf8Bom);
        assert_eq!(t, b"abc");
    }

    #[test]
    fn sjis_round_trip() {
        let sjis = utf8_to_sjis("日本語ログ".as_bytes());
        let (t, e) = decode(sjis.clone());
        assert_eq!(e, Encoding::Sjis);
        assert_eq!(String::from_utf8(t.clone()).unwrap(), "日本語ログ");
        assert_eq!(encode(&t, Encoding::Sjis), sjis);
    }

    #[test]
    fn eol_detection() {
        assert_eq!(detect_eol(b"a\r\nb"), Eol::Crlf);
        assert_eq!(detect_eol(b"a\nb"), Eol::Lf);
        assert_eq!(detect_eol(b"abc"), Eol::Crlf);
    }
}
