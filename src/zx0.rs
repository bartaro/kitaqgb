// Copyright (c) 2026 DAISUKE OBA. MIT License; see LICENSE.
//! Native port of KITAQ's independently authored ZX0 v2 forward codec.
//! The ZX0 format was designed by Einar Saukas. No upstream implementation is included.
use std::{
    fmt::Write as _,
    fs,
    path::{Component, Path, PathBuf},
};

#[derive(Default)]
struct BitOutput {
    data: Vec<u8>,
    control: usize,
    mask: u8,
    borrowed: Option<usize>,
}
impl BitOutput {
    fn bit(&mut self, bit: u8) {
        if let Some(i) = self.borrowed.take() {
            self.data[i] |= bit;
            return;
        }
        if self.mask == 0 {
            self.control = self.data.len();
            self.data.push(0);
            self.mask = 128;
        }
        if bit != 0 {
            self.data[self.control] |= self.mask;
        }
        self.mask >>= 1;
    }
    fn gamma(&mut self, value: usize, invert: bool) {
        let mut top = 1;
        while top <= value / 2 {
            top <<= 1;
        }
        top >>= 1;
        while top != 0 {
            self.bit(0);
            self.bit(u8::from(value & top != 0) ^ u8::from(invert));
            top >>= 1;
        }
        self.bit(1);
    }
    fn offset(&mut self, value: u8) {
        self.data.push(value);
        self.borrowed = Some(self.data.len() - 1);
    }
}
fn gamma_bits(mut n: usize) -> usize {
    let mut count = 1;
    n >>= 1;
    while n != 0 {
        count += 2;
        n >>= 1;
    }
    count
}

pub fn compress(input: &[u8]) -> Result<Vec<u8>, String> {
    if input.is_empty() || input.len() > 65535 {
        return Err("ZX0 input must contain 1..65535 bytes.".into());
    }
    let mut writer = BitOutput::default();
    let mut heads = vec![usize::MAX; 65536];
    let mut previous = vec![usize::MAX; input.len()];
    let (mut at, mut indexed, mut literal_start, mut last_offset) = (1, 0, 0, 1);
    let mut first = true;
    while at < input.len() {
        while indexed < at {
            if indexed + 1 < input.len() {
                let key = (usize::from(input[indexed]) << 8) | usize::from(input[indexed + 1]);
                previous[indexed] = heads[key];
                heads[key] = indexed;
            }
            indexed += 1;
        }
        let (mut best_len, mut best_offset, mut best_saving) = (0, 0, 0);
        if at + 1 < input.len() {
            let mut candidate = heads[(usize::from(input[at]) << 8) | usize::from(input[at + 1])];
            let mut attempts = 0;
            while candidate != usize::MAX && at - candidate <= 32640 && attempts < 256 {
                attempts += 1;
                let distance = at - candidate;
                let mut length = 2;
                while at + length < input.len()
                    && input[at + length] == input[at + length - distance]
                {
                    length += 1;
                }
                let reuse = literal_start < at && distance == last_offset;
                let cost = if reuse {
                    1 + gamma_bits(length)
                } else {
                    8 + gamma_bits((distance - 1) / 128 + 1) + gamma_bits(length - 1)
                };
                let saving = (length * 8).saturating_sub(cost);
                if saving > best_saving || (saving == best_saving && length > best_len) {
                    best_len = length;
                    best_offset = distance;
                    best_saving = saving;
                }
                if at + length == input.len() {
                    break;
                }
                candidate = previous[candidate];
            }
        }
        if best_len < 2 || best_saving == 0 {
            at += 1;
            continue;
        }
        let after_literal = literal_start < at;
        if after_literal {
            if !first {
                writer.bit(0);
            }
            writer.gamma(at - literal_start, false);
            writer.data.extend_from_slice(&input[literal_start..at]);
            first = false;
        }
        if after_literal && best_offset == last_offset {
            writer.bit(0);
            writer.gamma(best_len, false);
        } else {
            writer.bit(1);
            writer.gamma((best_offset - 1) / 128 + 1, true);
            writer.offset(((127 - (best_offset - 1) % 128) << 1) as u8);
            writer.gamma(best_len - 1, false);
            last_offset = best_offset;
        }
        at += best_len;
        literal_start = at;
    }
    if literal_start < input.len() {
        if !first {
            writer.bit(0);
        }
        writer.gamma(input.len() - literal_start, false);
        writer.data.extend_from_slice(&input[literal_start..]);
    }
    writer.bit(1);
    writer.gamma(256, true);
    Ok(writer.data)
}
struct Cursor<'a> {
    data: &'a [u8],
    at: usize,
    bits: u8,
    remaining: u8,
    borrowed: Option<u8>,
}
impl Cursor<'_> {
    fn byte(&mut self) -> Result<u8, String> {
        let b = self
            .data
            .get(self.at)
            .copied()
            .ok_or("Truncated ZX0 stream.")?;
        self.at += 1;
        Ok(b)
    }
    fn bit(&mut self) -> Result<u8, String> {
        if let Some(b) = self.borrowed.take() {
            return Ok(b);
        }
        if self.remaining == 0 {
            self.bits = self.byte()?;
            self.remaining = 8;
        }
        let bit = self.bits >> 7;
        self.bits <<= 1;
        self.remaining -= 1;
        Ok(bit)
    }
    fn number(&mut self, invert: bool) -> Result<usize, String> {
        let mut n = 1;
        while self.bit()? == 0 {
            if n >= 32768 {
                return Err("ZX0 integer exceeds 16 bits.".into());
            }
            n = n * 2 + usize::from(self.bit()? ^ u8::from(invert));
        }
        Ok(n)
    }
}
pub fn decompress(packed: &[u8], capacity: usize) -> Result<Vec<u8>, String> {
    if capacity > 65535 {
        return Err("Invalid decoder arguments.".into());
    }
    let mut input = Cursor {
        data: packed,
        at: 0,
        bits: 0,
        remaining: 0,
        borrowed: None,
    };
    let mut output = Vec::new();
    let (mut phase, mut distance) = (0, 1);
    loop {
        let count = if phase == 2 {
            let upper = input.number(true)?;
            if upper == 256 {
                if input.at != packed.len() {
                    return Err("Trailing bytes after ZX0 end marker.".into());
                }
                return Ok(output);
            }
            if upper > 255 {
                return Err("Invalid ZX0 offset.".into());
            }
            let low = input.byte()?;
            distance = upper * 128 - usize::from(low >> 1);
            input.borrowed = Some(low & 1);
            input.number(false)? + 1
        } else {
            input.number(false)?
        };
        if count > capacity - output.len() {
            return Err("ZX0 output exceeds capacity.".into());
        }
        if phase == 0 {
            for _ in 0..count {
                output.push(input.byte()?);
            }
            phase = if input.bit()? == 0 { 1 } else { 2 };
        } else {
            if distance > output.len() {
                return Err("ZX0 match precedes output.".into());
            }
            for _ in 0..count {
                output.push(output[output.len() - distance]);
            }
            phase = if input.bit()? == 0 { 0 } else { 2 };
        }
    }
}
pub fn rle(data: &[u8]) -> Vec<u8> {
    let mut result = Vec::new();
    let mut at = 0;
    while at < data.len() {
        let mut n = 1;
        while n < 255 && at + n < data.len() && data[at + n] == data[at] {
            n += 1;
        }
        result.extend_from_slice(&[n as u8, data[at]]);
        at += n;
    }
    result.push(0);
    result
}
pub fn automatic(input: &[u8]) -> Result<(Vec<u8>, &'static str), String> {
    if input.len() > 65535 {
        return Err("Asset must contain 0..65535 bytes.".into());
    }
    let (mut payload, mut codec, mut chosen) = (input.to_vec(), 0, "raw");
    let encoded = rle(input);
    if encoded.len() < payload.len() {
        payload = encoded;
        codec = 1;
        chosen = "rle";
    }
    if !input.is_empty() {
        let encoded = compress(input)?;
        if encoded.len() < payload.len() {
            payload = encoded;
            codec = 2;
            chosen = "zx0";
        }
    }
    if payload.len() > 65535 - 9 {
        return Err(
            "Packed asset including the KQA1 header exceeds 65535 bytes; split the asset.".into(),
        );
    }
    let mut result = b"KQA1".to_vec();
    result.push(codec);
    result.extend_from_slice(&(input.len() as u16).to_le_bytes());
    result.extend_from_slice(&(payload.len() as u16).to_le_bytes());
    result.extend(payload);
    Ok((result, chosen))
}
pub fn header(data: &[u8], raw_size: usize, name: &str) -> Result<String, String> {
    let mut chars = name.bytes();
    if !chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == b'_')
        || !chars.all(|c| c.is_ascii_alphanumeric() || c == b'_')
    {
        return Err("Invalid C identifier.".into());
    }
    let mut text = format!(
        "// Generated asset data. Original asset rights remain with its author.\n#pragma once\n__prg_rom u8 {name}[] = {{\n"
    );
    if data.is_empty() {
        text.push('0');
    }
    for (i, byte) in data.iter().enumerate() {
        write!(text, "{byte},").unwrap();
        if i % 24 == 23 {
            text.push('\n');
        }
    }
    write!(
        text,
        "\n}};\n#define {name}_SIZE {}\n#define {name}_RAW_SIZE {raw_size}\n",
        data.len()
    )
    .unwrap();
    Ok(text)
}
fn absolute(path: &Path) -> Result<PathBuf, String> {
    let full = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|e| e.to_string())?
            .join(path)
    };
    let mut normal = PathBuf::new();
    for part in full.components() {
        match part {
            Component::CurDir => {}
            Component::ParentDir => {
                normal.pop();
            }
            _ => normal.push(part.as_os_str()),
        }
    }
    Ok(normal)
}
pub fn cli(tool: &str, args: &[String]) -> Result<(), String> {
    let usage = format!(
        "Usage: {tool} input output [--format=zx0|raw|rle|auto] [--header=identifier] [--decompress]"
    );
    if args == ["--help"] || args == ["-h"] {
        println!("{usage}");
        return Ok(());
    }
    if args == ["--version"] {
        println!("{tool} {} (Rust)", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }
    let mut paths = Vec::new();
    let mut format = "zx0";
    let mut identifier = None;
    let mut decode = false;
    for arg in args {
        if arg == "--decompress" {
            decode = true;
        } else if let Some(value) = arg.strip_prefix("--format=") {
            format = value;
        } else if let Some(value) = arg.strip_prefix("--header=") {
            identifier = Some(value);
        } else if arg.starts_with("--") {
            return Err(format!("Unknown option: {arg}"));
        } else {
            paths.push(Path::new(arg));
        }
    }
    if paths.len() != 2 {
        return Err(usage);
    }
    let source = absolute(paths[0])?;
    let target = absolute(paths[1])?;
    if source
        .to_string_lossy()
        .eq_ignore_ascii_case(&target.to_string_lossy())
        || (source.canonicalize().ok().is_some()
            && source.canonicalize().ok() == target.canonicalize().ok())
    {
        return Err("Input and output must differ.".into());
    }
    let input = fs::read(&source).map_err(|e| e.to_string())?;
    let (result, chosen) = if decode {
        if format != "zx0" || identifier.is_some() {
            return Err("--decompress accepts a ZX0 v2 stream and binary output only.".into());
        }
        (decompress(&input, 65535)?, format)
    } else {
        match format {
            "auto" => automatic(&input)?,
            "zx0" => (compress(&input)?, format),
            "raw" | "rle" => {
                if input.len() > 65535 {
                    return Err("Asset exceeds 65535 bytes.".into());
                }
                (
                    if format == "raw" {
                        input.clone()
                    } else {
                        rle(&input)
                    },
                    format,
                )
            }
            _ => return Err(format!("Unknown format: {format}")),
        }
    };
    if !decode && result.len() > 65535 {
        return Err("Packed data exceeds the 16-bit target size; split the asset.".into());
    }
    let bytes = if let Some(name) = identifier {
        header(&result, input.len(), name)?.into_bytes()
    } else {
        result.clone()
    };
    fs::write(target, bytes).map_err(|e| e.to_string())?;
    println!(
        "{chosen}: {} -> {} bytes{}",
        input.len(),
        result.len(),
        if format == "auto" {
            " (includes 9-byte KQA1 header)"
        } else {
            ""
        }
    );
    Ok(())
}
