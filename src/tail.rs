//! tail -f 追従。タイマーでファイルサイズを監視し、増えた分だけ読み込む。
//! ネットワークドライブでも確実に動くようポーリング方式にしている。

use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

use crate::doc::{self, Encoding};

pub struct Tail {
    /// ここまで読み込み済み（ファイル先頭からのバイト位置）
    pub offset: u64,
    pub enc: Encoding,
}

pub enum Event {
    None,
    /// UTF-8 に変換済みの追記分
    Appended(Vec<u8>),
    /// ファイルが縮んだ（ローテーション・切り詰め）。全体を読み直す
    Truncated,
}

const MAX_CHUNK: u64 = 64 << 20;

pub fn poll(path: &Path, t: &mut Tail) -> Event {
    let Ok(meta) = std::fs::metadata(path) else { return Event::None };
    let size = meta.len();
    if size < t.offset {
        return Event::Truncated;
    }
    if size == t.offset {
        return Event::None;
    }
    let Ok(mut f) = std::fs::File::open(path) else { return Event::None };
    if f.seek(SeekFrom::Start(t.offset)).is_err() {
        return Event::None;
    }
    let want = (size - t.offset).min(MAX_CHUNK);
    let mut buf = vec![0u8; want as usize];
    let Ok(n) = f.read(&mut buf) else { return Event::None };
    buf.truncate(n);
    // 書きかけの行は次回に回す
    let utf16 = matches!(t.enc, Encoding::Utf16Le | Encoding::Utf16Be);
    let cut = if utf16 {
        let nl: [u8; 2] = if t.enc == Encoding::Utf16Le { [b'\n', 0] } else { [0, b'\n'] };
        buf.chunks_exact(2).rposition(|c| c == nl).map(|i| i * 2 + 2)
    } else {
        buf.iter().rposition(|&b| b == b'\n').map(|i| i + 1)
    };
    let Some(cut) = cut else {
        // 改行が無いまま大きくなった場合はそのまま取り込む
        if buf.len() as u64 >= MAX_CHUNK {
            t.offset += buf.len() as u64;
            return Event::Appended(doc::decode_chunk(&buf, t.enc));
        }
        return Event::None;
    };
    buf.truncate(cut);
    t.offset += cut as u64;
    Event::Appended(doc::decode_chunk(&buf, t.enc))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn reads_only_complete_lines() {
        let p = std::env::temp_dir().join(format!("sakura2_tail_test_{}.log", std::process::id()));
        std::fs::write(&p, b"a\n").unwrap();
        let mut t = Tail { offset: 2, enc: Encoding::Utf8 };
        assert!(matches!(poll(&p, &mut t), Event::None));
        let mut f = std::fs::OpenOptions::new().append(true).open(&p).unwrap();
        f.write_all(b"b\nc").unwrap();
        match poll(&p, &mut t) {
            Event::Appended(v) => assert_eq!(v, b"b\n"),
            _ => panic!(),
        }
        f.write_all(b"\n").unwrap();
        match poll(&p, &mut t) {
            Event::Appended(v) => assert_eq!(v, b"c\n"),
            _ => panic!(),
        }
        drop(f);
        std::fs::write(&p, b"x\n").unwrap();
        assert!(matches!(poll(&p, &mut t), Event::Truncated));
        let _ = std::fs::remove_file(&p);
    }
}
