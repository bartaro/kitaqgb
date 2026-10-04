use crate::json::Value;

pub const LOGO: [u8; 48] = [
    0xce, 0xed, 0x66, 0x66, 0xcc, 0x0d, 0, 0x0b, 3, 0x73, 0, 0x83, 0, 0x0c, 0, 0x0d, 0, 8, 0x11,
    0x1f, 0x88, 0x89, 0, 0x0e, 0xdc, 0xcc, 0x6e, 0xe6, 0xdd, 0xdd, 0xd9, 0x99, 0xbb, 0xbb, 0x67,
    0x63, 0x6e, 0x0e, 0xec, 0xcc, 0xdd, 0xdc, 0x99, 0x9f, 0xbb, 0xb9, 0x33, 0x3e,
];
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Options {
    pub logo: Option<bool>,
    pub title: Option<String>,
    pub cgb: Option<u8>,
    pub cart: Option<u8>,
    pub romsize: Option<u8>,
    pub romsize_bytes: Option<usize>,
    pub ramsize: Option<u8>,
    pub sgb: Option<u8>,
    pub destination: Option<u8>,
    pub version: Option<u8>,
}
impl Options {
    pub fn has_any(&self) -> bool {
        self.logo.is_some()
            || self.title.is_some()
            || self.cgb.is_some()
            || self.cart.is_some()
            || self.romsize.is_some()
            || self.ramsize.is_some()
            || self.sgb.is_some()
            || self.destination.is_some()
            || self.version.is_some()
    }
    pub fn merge_from(&mut self, b: &Self, overwrite: bool) {
        macro_rules! merge{($($field:ident),*)=>{$(if b.$field.is_some()&&(overwrite||self.$field.is_none()){self.$field=b.$field.clone();})*}}
        merge!(
            logo,
            title,
            cgb,
            cart,
            romsize,
            romsize_bytes,
            ramsize,
            sgb,
            destination,
            version
        );
    }
    pub fn is_set(&self, key: &str) -> bool {
        match normalize_key(key).as_str() {
            "header_logo" => self.logo.is_some(),
            "title" => self.title.is_some(),
            "cgb" => self.cgb.is_some(),
            "cart" => self.cart.is_some(),
            "romsize" => self.romsize.is_some() || self.romsize_bytes.is_some(),
            "ramsize" => self.ramsize.is_some(),
            "sgb" => self.sgb.is_some(),
            "dest" => self.destination.is_some(),
            "version" => self.version.is_some(),
            _ => false,
        }
    }
    pub fn set(&mut self, key: &str, value: &str) -> Result<(), String> {
        let k = normalize_key(key);
        let s = value.trim().to_ascii_lowercase();
        match k.as_str() {
            "title" => self.title = Some(value.into()),
            "header_logo" => {
                self.logo = Some(match s.as_str() {
                    "on" | "1" | "true" | "yes" | "default" => true,
                    "off" | "0" | "false" | "no" | "none" | "omit" => false,
                    _ => return Err("error: --header-logo must be on|off".into()),
                })
            }
            "cgb" => {
                self.cgb = Some(match s.as_str() {
                    "dmg" | "gb" | "dmg_only" => 0,
                    "cgb" | "support" | "compatible" | "cgb_support" => 0x80,
                    "cgb_only" | "only" => 0xc0,
                    _ => return Err("error: --cgb must be dmg|cgb|cgb_only".into()),
                })
            }
            "cart" => {
                self.cart = Some(match s.as_str() {
                    "romonly" | "rom_only" | "rom" => 0,
                    "mbc1" => 1,
                    "mbc1_ram" => 2,
                    "mbc1_ram_batt" | "mbc1_ram_battery" => 3,
                    "mbc3" => 0x11,
                    "mbc3_ram" => 0x12,
                    "mbc3_ram_batt" | "mbc3_ram_battery" => 0x13,
                    "mbc5" => 0x19,
                    "mbc5_ram" => 0x1a,
                    "mbc5_ram_batt" | "mbc5_ram_battery" => 0x1b,
                    _ => {
                        return Err(format!(
                            "error: --cart unsupported value: {s} (try romonly|mbc1|mbc3|mbc5 ...)"
                        ));
                    }
                })
            }
            "romsize" => {
                let s = normalize_size(&s);
                let code = match s.as_str() {
                    "32k" => 0,
                    "64k" => 1,
                    "128k" => 2,
                    "256k" => 3,
                    "512k" => 4,
                    "1m" => 5,
                    "2m" => 6,
                    "4m" => 7,
                    "8m" => 8,
                    _ => {
                        return Err(
                            "error: --romsize must be 32k|64k|128k|256k|512k|1m|2m|4m|8m".into(),
                        );
                    }
                };
                self.romsize = Some(code);
                self.romsize_bytes = rom_size_from_code(code);
            }
            "ramsize" => {
                self.ramsize = Some(match normalize_size(&s).as_str() {
                    "none" | "0" => 0,
                    "2k" => 1,
                    "8k" => 2,
                    "32k" => 3,
                    "128k" => 4,
                    "64k" => 5,
                    _ => return Err("error: --ramsize must be none|2k|8k|32k|64k|128k".into()),
                })
            }
            "sgb" => {
                self.sgb = Some(match s.as_str() {
                    "on" | "1" | "true" | "sgb_support" => 3,
                    "off" | "0" | "false" => 0,
                    _ => return Err("error: --sgb must be on|off".into()),
                })
            }
            "dest" => {
                self.destination = Some(match s.as_str() {
                    "jp" | "japan" | "0" | "dest_jp" => 0,
                    "nonjp" | "non-jp" | "world" | "1" | "dest_nonjp" => 1,
                    _ => return Err("error: --dest must be jp|nonjp".into()),
                })
            }
            "version" => {
                self.version = Some(
                    value
                        .trim()
                        .parse::<u8>()
                        .map_err(|_| "warning: version must be 0..255")?,
                )
            }
            _ => return Err(format!("warning: unknown rom header key: {key}")),
        }
        Ok(())
    }
    pub fn from_json(text: &str) -> Result<Self, String> {
        let Value::Object(values) = Value::parse(text)? else {
            return Err("root must be an object".into());
        };
        let mut opt = Self::default();
        for (k, v) in values {
            let s = match v {
                Value::String(s) => s,
                Value::Number(n) => n.to_string(),
                Value::Bool(true) => "on".into(),
                Value::Bool(false) => "off".into(),
                _ => {
                    return Err(format!(
                        "invalid '{k}': expected string, integer or boolean"
                    ));
                }
            };
            opt.set(&k, &s).map_err(|e| format!("invalid '{k}': {e}"))?;
        }
        Ok(opt)
    }
}
pub fn normalize_key(key: &str) -> String {
    let k = key.trim().replace('-', "_").to_ascii_lowercase();
    let k = k.strip_prefix("rom_").unwrap_or(&k);
    match k {
        "romtitle" | "title" | "rom_title" | "game_title" => "title",
        "header_logo" | "headerlogo" | "logo" | "validation_logo" => "header_logo",
        "cartridge" => "cart",
        "rom_size" => "romsize",
        "ram_size" => "ramsize",
        "destination" => "dest",
        "rom_version" => "version",
        _ => k,
    }
    .into()
}
fn normalize_size(s: &str) -> String {
    s.trim()
        .to_ascii_lowercase()
        .replace(' ', "")
        .replace("bytes", "")
        .replace('b', "")
}
pub fn rom_size_from_code(code: u8) -> Option<usize> {
    match code {
        0..=8 => Some(0x8000usize << code),
        0x52 => Some(1152 * 1024),
        0x53 => Some(1280 * 1024),
        0x54 => Some(1536 * 1024),
        _ => None,
    }
}
pub fn rom_size_code(bytes: usize) -> u8 {
    (0..=8).find(|n| bytes <= (0x8000usize << n)).unwrap_or(8)
}
pub fn write_logo(rom: &mut [u8], enabled: bool) {
    if let Some(dst) = rom.get_mut(0x104..0x134) {
        if enabled {
            dst.copy_from_slice(&LOGO)
        } else {
            dst.fill(0xff)
        }
    }
}
pub fn header_checksum(rom: &[u8]) -> Result<u8, String> {
    Ok(rom
        .get(0x134..=0x14c)
        .ok_or("ROM too small")?
        .iter()
        .fold(0u8, |s, b| s.wrapping_sub(*b).wrapping_sub(1)))
}
pub fn global_checksum(rom: &[u8]) -> u16 {
    rom.iter()
        .enumerate()
        .filter(|(i, _)| *i != 0x14e && *i != 0x14f)
        .fold(0u16, |s, (_, b)| s.wrapping_add(*b as u16))
}
pub fn fix_checksums(rom: &mut [u8]) -> Result<(), String> {
    if rom.len() < 0x150 {
        return Err("ROM too small".into());
    }
    rom[0x14d] = header_checksum(rom)?;
    let g = global_checksum(rom);
    rom[0x14e] = (g >> 8) as u8;
    rom[0x14f] = g as u8;
    Ok(())
}
/// Patch a copy so every validation failure leaves the caller's ROM unchanged.
pub fn patch(rom: &[u8], opt: &Options) -> Result<(Vec<u8>, Vec<String>), String> {
    if !opt.has_any() {
        return Ok((rom.to_vec(), Vec::new()));
    }
    if rom.len() < 0x150 {
        return Err(format!(
            "error: ROM header patch: ROM too small ({} bytes)",
            rom.len()
        ));
    }
    let mut out = rom.to_vec();
    let mut warnings = Vec::new();
    let expected = opt
        .romsize_bytes
        .or_else(|| opt.romsize.and_then(rom_size_from_code));
    if let Some(expected) = expected {
        if out.len() > expected {
            return Err(format!(
                "error: ROM size {} bytes exceeds header/--romsize expectation ({expected} bytes)",
                out.len()
            ));
        }
        if out.len() < expected {
            warnings.push(format!(
                "warning: ROM size {} bytes smaller than {} ({expected} bytes); padding with 0xFF",
                out.len(),
                if opt.romsize_bytes.is_some() {
                    "--romsize"
                } else {
                    "header ROM size code"
                }
            ));
            out.resize(expected, 0xff);
        }
    }
    if opt.cart == Some(0) && out.len() > 32768 {
        return Err(format!(
            "error: --cart=romonly cannot be used with ROM size >32K (actual {} bytes)",
            out.len()
        ));
    }
    if let Some(logo) = opt.logo {
        write_logo(&mut out, logo);
    }
    if let Some(title) = &opt.title {
        let mut ascii = title.encode_utf16().map(|c| {
            if (0x20..=0x7e).contains(&c) {
                c as u8
            } else {
                b'?'
            }
        });
        for b in &mut out[0x134..0x144] {
            *b = ascii.next().unwrap_or(0);
        }
    }
    for (offset, v) in [
        (0x143, opt.cgb),
        (0x146, opt.sgb),
        (0x147, opt.cart),
        (0x148, opt.romsize),
        (0x149, opt.ramsize),
        (0x14a, opt.destination),
        (0x14c, opt.version),
    ] {
        if let Some(v) = v {
            out[offset] = v;
        }
    }
    fix_checksums(&mut out)?;
    Ok((out, warnings))
}
