use std::{
    fs::{self, File, OpenOptions},
    io::{BufReader, Read, Seek, SeekFrom, Write},
};

const PATCH_PATTERN: &str = "\\xe9....\\x85\\xf6\\x0f\\x84....\\x48\\x8b\\x59\\x10";
const PATCH_BYTE: u8 = 0x85;
const PATCH_OFFSET: usize = 8;

const DEFAULT_RESOLVE_PATH: &str =
    "C:\\Program Files\\Blackmagic Design\\DaVinci Resolve\\Resolve.exe";

fn pattern_to_bytes(pattern: &str) -> Vec<Option<u8>> {
    let mut bytes = Vec::new();
    let mut chars = pattern.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            '\\' if chars.peek() == Some(&'x') => {
                chars.next(); 
                let hex1 = chars.next().unwrap();
                let hex2 = chars.next().unwrap();
                let byte_str = format!("{}{}", hex1, hex2);
                bytes.push(Some(u8::from_str_radix(&byte_str, 16).unwrap()));
            }
            '.' => {
                bytes.push(None);
            }
            _ => continue, // ignore garbage
        }
    }
    bytes
}

fn matches_pattern(buffer: &[u8], offset: usize, pattern: &[Option<u8>]) -> bool {
    if offset + pattern.len() > buffer.len() {
        return false;
    }

    pattern
        .iter()
        .enumerate()
        .all(|(i, &pattern_byte)| pattern_byte.map_or(true, |b| buffer[offset + i] == b))
}

fn main() {
    println!("[INFO] Davinci Resolve Studio patcher by @sipacid");

    let pattern = pattern_to_bytes(PATCH_PATTERN);
    let file_path = String::from(DEFAULT_RESOLVE_PATH);

    if !fs::metadata(&file_path).is_ok() {
        eprintln!(
            "[ERROR] Resolve.exe not found at default path: {}",
            file_path
        );
        return;
    }

    let read_file = File::open(&file_path).unwrap();
    let mut reader = BufReader::new(read_file);
    let mut buffer = Vec::new();
    reader.read_to_end(&mut buffer).unwrap();

    let patch_offset = buffer
        .iter()
        .enumerate()
        .position(|(i, _)| matches_pattern(&buffer, i, &pattern))
        .map(|i| i + PATCH_OFFSET);

    let Some(patch_offset) = patch_offset else {
        eprintln!("[ERROR] Patch pattern not found in Resolve.exe");
        return;
    };

    println!(
        "[INFO] Found pattern at offset: 0x{:X}",
        patch_offset - PATCH_OFFSET
    );

    if patch_offset >= buffer.len() {
        eprintln!("[ERROR] Cannot patch: offset is beyond file size");
        return;
    }

    let mut write_file = match OpenOptions::new().write(true).open(&file_path) {
        Ok(file) => file,
        Err(e) => {
            eprintln!(
                "[ERROR] Cannot open file for writing. Are you running as administrator? ({})",
                e
            );
            return;
        }
    };

    buffer[patch_offset] = PATCH_BYTE;

    write_file.seek(SeekFrom::Start(0)).unwrap();
    write_file.set_len(buffer.len() as u64).unwrap();
    if let Err(e) = write_file.write_all(&buffer) {
        eprintln!("[ERROR] Failed to write patch: {}", e);
        return;
    }

    println!(
        "[INFO] Successfully patched the file at offset: 0x{:X}",
        patch_offset
    );
}
