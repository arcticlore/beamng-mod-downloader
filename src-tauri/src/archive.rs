//! Проверка целостности zip-архивов и SHA-256-хэширование файлов.
//!
//! Структурная валидация читает EOCD (End of Central Directory) и центральный
//! каталог, сверяет их смещения длину и пробегает по всем записям (сигнатуры,
//! границы локальных заголовков и данных). Это отсекает усечённые и повреждённые
//! архивы без полной распаковки и без внешних крэйтов. ZIP64-маркеры
//! (0xFFFF/0xFFFFFFFF) не расшифровываются, но и не считаются ошибкой.

use anyhow::{anyhow, Context, Result};
use sha2::{Digest, Sha256};
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

const EOCD_SIG: [u8; 4] = [0x50, 0x4b, 0x05, 0x06];
const LFH_SIG: [u8; 4] = [0x50, 0x4b, 0x03, 0x04];
const CD_SIG: [u8; 4] = [0x50, 0x4b, 0x01, 0x02];
const EOCD_MIN: u64 = 22;
const MAX_COMMENT: u64 = 0xffff;

/// Результат структурной проверки архива.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ZipSummary {
    pub entries: usize,
    pub total_uncompressed: u64,
}

const HEX: &[u8; 16] = b"0123456789abcdef";

/// Нижний регистр hex-представления байтов (для SHA-256-сумм).
pub fn to_hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        s.push(HEX[(b >> 4) as usize] as char);
        s.push(HEX[(b & 0x0f) as usize] as char);
    }
    s
}

/// SHA-256 файла (hex).
pub fn sha256_file(path: &Path) -> Result<String> {
    let mut f = std::fs::File::open(path).with_context(|| {
        crate::i18n::tf(
            "не удалось открыть {0}",
            "failed to open {0}",
            &[&path.display().to_string()],
        )
    })?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = f.read(&mut buf).with_context(|| {
            crate::i18n::tf(
                "ошибка чтения {0}",
                "read error {0}",
                &[&path.display().to_string()],
            )
        })?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(to_hex(&hasher.finalize()))
}

fn rd_u16(b: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([b[at], b[at + 1]])
}

fn rd_u32(b: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([b[at], b[at + 1], b[at + 2], b[at + 3]])
}

/// Структурная проверка zip-архива: EOCD + центральный каталог.
pub fn validate_zip(path: &Path) -> Result<ZipSummary> {
    let mut f = std::fs::File::open(path).with_context(|| {
        crate::i18n::tf(
            "не удалось открыть {0}",
            "failed to open {0}",
            &[&path.display().to_string()],
        )
    })?;
    let file_len = f
        .metadata()
        .with_context(|| {
            crate::i18n::tf(
                "не удалось получить размер {0}",
                "failed to get the size of {0}",
                &[&path.display().to_string()],
            )
        })?
        .len();
    if file_len < EOCD_MIN {
        return Err(anyhow!(crate::i18n::t(
            "файл меньше минимального размера zip-архива",
            "file smaller than the minimum zip size"
        )));
    }

    // Комментарий после EOCD допустим до 64 КБ — ищем сигнатуру с конца файла.
    let scan = file_len.min(EOCD_MIN + MAX_COMMENT) as usize;
    let mut tail = vec![0u8; scan];
    f.seek(SeekFrom::End(-(scan as i64)))?;
    f.read_exact(&mut tail).with_context(|| {
        crate::i18n::tf(
            "не удалось прочитать хвост {0}",
            "failed to read the tail of {0}",
            &[&path.display().to_string()],
        )
    })?;
    let rel = tail
        .iter()
        .rposition(|b| *b == 0x50)
        .filter(|&i| i + 4 <= tail.len() && tail[i..i + 4] == EOCD_SIG)
        .ok_or_else(|| {
            anyhow!(crate::i18n::t(
                "не найден маркер конца архива (EOCD) — файл не zip",
                "no end-of-archive marker (EOCD) found — not a zip file"
            ))
        })?;
    let eocd_at = (file_len as i64 - scan as i64 + rel as i64) as u64;

    if eocd_at + EOCD_MIN > file_len {
        return Err(anyhow!(crate::i18n::t(
            "маркер конца архива выходит за пределы файла",
            "end-of-archive marker exceeds the file bounds"
        )));
    }
    if rd_u16(&tail, rel + 4) != 0 || rd_u16(&tail, rel + 6) != 0 {
        return Err(anyhow!(crate::i18n::t(
            "многодисковые архивы не поддерживаются",
            "multi-disk archives are not supported"
        )));
    }
    let comment_len = rd_u16(&tail, rel + 20) as u64;
    if eocd_at + EOCD_MIN + comment_len != file_len {
        return Err(anyhow!(crate::i18n::t(
            "хвост файла не соответствует комментарию EOCD",
            "file tail does not match the EOCD comment"
        )));
    }

    let entries = rd_u16(&tail, rel + 10) as usize;
    if entries == 0 {
        return Err(anyhow!(crate::i18n::t(
            "в архиве нет записей",
            "the archive has no entries"
        )));
    }
    let cd_size = rd_u32(&tail, rel + 12) as u64;
    let cd_offset = rd_u32(&tail, rel + 16) as u64;
    let cd_end = cd_offset.checked_add(cd_size).ok_or_else(|| {
        anyhow!(crate::i18n::t(
            "переполнение размера центрального каталога",
            "central directory size overflow"
        ))
    })?;
    if cd_end != eocd_at {
        return Err(anyhow!(crate::i18n::tf(
            "центральный каталог не стыкуется с концом архива (offset={0}, size={1}, eocd={2})",
            "central directory does not align with the archive end (offset={0}, size={1}, eocd={2})",
            &[&cd_offset.to_string(), &cd_size.to_string(), &eocd_at.to_string()],
        )));
    }
    if cd_size == 0 {
        return Err(anyhow!(crate::i18n::t(
            "центральный каталог пуст",
            "central directory is empty"
        )));
    }
    let cd_len = cd_size as usize;
    let mut cd = vec![0u8; cd_len];
    f.seek(SeekFrom::Start(cd_offset))?;
    f.read_exact(&mut cd).with_context(|| {
        crate::i18n::tf(
            "не удалось прочитать каталог {0}",
            "failed to read the directory of {0}",
            &[&path.display().to_string()],
        )
    })?;

    let mut pos = 0usize;
    let mut total_uncompressed: u64 = 0;
    for i in 0..entries {
        if pos + 46 > cd_len {
            return Err(anyhow!(crate::i18n::tf(
                "запись {0} обрывается в каталоге",
                "entry {0} is truncated in the directory",
                &[&(i + 1).to_string()]
            )));
        }
        if cd[pos..pos + 4] != CD_SIG {
            return Err(anyhow!(crate::i18n::tf(
                "битая сигнатура записи {0} в каталоге",
                "bad signature of entry {0} in the directory",
                &[&(i + 1).to_string()]
            )));
        }
        let name_len = rd_u16(&cd, pos + 28) as usize;
        let extra_len = rd_u16(&cd, pos + 30) as usize;
        let entry_comment_len = rd_u16(&cd, pos + 32) as usize;
        let c_size = rd_u32(&cd, pos + 20) as u64;
        let u_size = rd_u32(&cd, pos + 24) as u64;
        let local_off = rd_u32(&cd, pos + 42) as u64;
        pos += 46 + name_len + extra_len + entry_comment_len;
        if pos > cd_len {
            return Err(anyhow!(crate::i18n::tf(
                "запись {0} выходит за пределы каталога",
                "entry {0} exceeds the directory bounds",
                &[&(i + 1).to_string()]
            )));
        }

        // Локальный заголовок обязан лежать до центрального каталога.
        if local_off + 30 > cd_offset {
            return Err(anyhow!(crate::i18n::tf(
                "запись {0}: локальный заголовок за пределами каталога",
                "entry {0}: local header is outside the directory",
                &[&(i + 1).to_string()],
            )));
        }
        let mut lfh = [0u8; 30];
        f.seek(SeekFrom::Start(local_off))?;
        f.read_exact(&mut lfh).with_context(|| {
            crate::i18n::tf(
                "не удалось прочитать локальный заголовок записи {0}",
                "failed to read the local header of entry {0}",
                &[&(i + 1).to_string()],
            )
        })?;
        if lfh[0..4] != LFH_SIG {
            return Err(anyhow!(crate::i18n::tf(
                "запись {0}: неверная сигнатура локального заголовка",
                "entry {0}: invalid local header signature",
                &[&(i + 1).to_string()],
            )));
        }
        let l_name_len = u16::from_le_bytes([lfh[26], lfh[27]]) as u64;
        let l_extra_len = u16::from_le_bytes([lfh[28], lfh[29]]) as u64;
        // ZIP64 маркер размера — расшифровывать не умеем, но хуже не делаем.
        if u_size != 0xffff_ffff {
            total_uncompressed = total_uncompressed.checked_add(u_size).ok_or_else(|| {
                anyhow!(crate::i18n::t(
                    "переполнение суммарного размера распакованных данных",
                    "total uncompressed size overflow"
                ))
            })?;
            let data_end = local_off + 30 + l_name_len + l_extra_len + c_size;
            if data_end > cd_offset {
                return Err(anyhow!(crate::i18n::tf(
                    "запись {0}: данные выходят за каталог",
                    "entry {0}: data exceeds the directory",
                    &[&(i + 1).to_string()]
                )));
            }
        }
    }
    if pos != cd_len {
        return Err(anyhow!(crate::i18n::tf(
            "центральный каталог длиннее, чем описано записей (лишние {0} байт)",
            "central directory is longer than described by entries ({0} extra bytes)",
            &[&(cd_len - pos).to_string()],
        )));
    }

    Ok(ZipSummary {
        entries,
        total_uncompressed,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write_temp(bytes: &[u8], label: &str) -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!(
            "bmd-archive-test-{}-{}.zip",
            std::process::id(),
            label
        ));
        std::fs::File::create(&path)
            .unwrap()
            .write_all(bytes)
            .unwrap();
        path
    }

    fn cleanup(path: &std::path::Path) {
        let _ = std::fs::remove_file(path);
    }

    /// Строит корректный одногофайловый zip (метод "stored").
    fn build_zip(name: &str, data: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        // Local file header
        out.extend_from_slice(&LFH_SIG);
        out.extend_from_slice(&20_u16.to_le_bytes());
        out.extend_from_slice(&0_u16.to_le_bytes());
        out.extend_from_slice(&0_u16.to_le_bytes());
        out.extend_from_slice(&0_u16.to_le_bytes());
        out.extend_from_slice(&0_u16.to_le_bytes());
        out.extend_from_slice(&0_u32.to_le_bytes());
        out.extend_from_slice(&(data.len() as u32).to_le_bytes());
        out.extend_from_slice(&(data.len() as u32).to_le_bytes());
        out.extend_from_slice(&(name.len() as u16).to_le_bytes());
        out.extend_from_slice(&0_u16.to_le_bytes());
        out.extend_from_slice(name.as_bytes());
        out.extend_from_slice(data);
        let local_off = 0_u32;
        // Central directory
        let cd_start = out.len() as u32;
        out.extend_from_slice(&CD_SIG);
        out.extend_from_slice(&20_u16.to_le_bytes());
        out.extend_from_slice(&20_u16.to_le_bytes());
        out.extend_from_slice(&0_u16.to_le_bytes());
        out.extend_from_slice(&0_u16.to_le_bytes());
        out.extend_from_slice(&0_u16.to_le_bytes());
        out.extend_from_slice(&0_u16.to_le_bytes());
        out.extend_from_slice(&0_u32.to_le_bytes());
        out.extend_from_slice(&(data.len() as u32).to_le_bytes());
        out.extend_from_slice(&(data.len() as u32).to_le_bytes());
        out.extend_from_slice(&(name.len() as u16).to_le_bytes());
        out.extend_from_slice(&0_u16.to_le_bytes());
        out.extend_from_slice(&0_u16.to_le_bytes());
        out.extend_from_slice(&0_u16.to_le_bytes());
        out.extend_from_slice(&0_u16.to_le_bytes());
        out.extend_from_slice(&0_u32.to_le_bytes());
        out.extend_from_slice(&local_off.to_le_bytes());
        out.extend_from_slice(name.as_bytes());
        let cd_size = (out.len() - cd_start as usize) as u32;
        // EOCD
        out.extend_from_slice(&EOCD_SIG);
        out.extend_from_slice(&0_u16.to_le_bytes());
        out.extend_from_slice(&0_u16.to_le_bytes());
        out.extend_from_slice(&1_u16.to_le_bytes());
        out.extend_from_slice(&1_u16.to_le_bytes());
        out.extend_from_slice(&cd_size.to_le_bytes());
        out.extend_from_slice(&cd_start.to_le_bytes());
        out.extend_from_slice(&0_u16.to_le_bytes());
        out
    }

    #[test]
    fn hex_encodes_lowercase() {
        assert_eq!(to_hex(&[0x00, 0x12, 0xff, 0xab]), "0012ffab");
        assert_eq!(to_hex(&[]), "");
    }

    #[test]
    fn valid_zip_parses() {
        let raw = build_zip("mod/main.txt", b"hello beamng");
        let path = write_temp(&raw, "valid");
        let s = validate_zip(&path).expect("валидный zip должен пройти");
        assert_eq!(s.entries, 1);
        assert_eq!(s.total_uncompressed, "hello beamng".len() as u64);
        cleanup(&path);
    }

    #[test]
    fn truncated_zip_rejected() {
        let raw = build_zip("mod/main.txt", b"hello beamng");
        let path = write_temp(&raw[..raw.len() - 10], "trunc");
        assert!(validate_zip(&path).is_err());
        cleanup(&path);
    }

    #[test]
    fn tiny_non_zip_rejected() {
        let path = write_temp(b"not a zip at all", "tiny");
        assert!(validate_zip(&path).is_err());
        cleanup(&path);
    }

    #[test]
    fn bad_cd_size_rejected() {
        let mut raw = build_zip("mod/main.txt", b"hello beamng");
        let cd_size_lsb = raw.len() - 22 + 12;
        raw[cd_size_lsb] ^= 0x01;
        let path = write_temp(&raw, "badsize");
        assert!(validate_zip(&path).is_err());
        cleanup(&path);
    }

    #[test]
    fn sha256_matches_known_value() {
        // SHA-256 от "abc"
        let path = write_temp(b"abc", "sha");
        assert_eq!(
            sha256_file(&path).unwrap(),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        cleanup(&path);
    }
}
