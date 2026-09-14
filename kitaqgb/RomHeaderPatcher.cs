using System;
using System.IO;
using System.Text;
using System.Text.RegularExpressions;

// Nullable GB header overrides preserve the difference between an unspecified field and an explicit zero/off value.
public sealed class RomHeaderOptions
{
    public bool? HeaderLogoEnabled = null; // 0x0104..0x0133
    public string Title = null; // 0x0134..0x0143 (16 bytes)
    public byte? CgbFlag = null; // 0x0143
    public byte? CartType = null; // 0x0147
    public byte? RomSizeCode = null; // 0x0148
    public int? RomSizeBytes = null; // derived from RomSizeCode
    public byte? RamSizeCode = null; // 0x0149
    public byte? SgbFlag = null; // 0x0146
    public byte? DestinationCode = null; // 0x014A
    public byte? Version = null; // 0x014C

    public bool HasAny => HeaderLogoEnabled.HasValue || Title != null || CgbFlag.HasValue || CartType.HasValue || RomSizeCode.HasValue || RamSizeCode.HasValue || SgbFlag.HasValue || DestinationCode.HasValue || Version.HasValue;

    // Merge only fields supplied by the other options object, either overwriting or filling unset fields.
    // ROM size code and byte count are merged independently.
    public void MergeFrom(RomHeaderOptions other, bool overwrite)
    {
        if (other == null) return;
        if (other.HeaderLogoEnabled.HasValue && (overwrite || !this.HeaderLogoEnabled.HasValue)) this.HeaderLogoEnabled = other.HeaderLogoEnabled;
        if (other.Title != null && (overwrite || this.Title == null)) this.Title = other.Title;
        if (other.CgbFlag.HasValue && (overwrite || !this.CgbFlag.HasValue)) this.CgbFlag = other.CgbFlag;
        if (other.SgbFlag.HasValue && (overwrite || !this.SgbFlag.HasValue)) this.SgbFlag = other.SgbFlag;
        if (other.CartType.HasValue && (overwrite || !this.CartType.HasValue)) this.CartType = other.CartType;
        if (other.RomSizeCode.HasValue && (overwrite || !this.RomSizeCode.HasValue)) this.RomSizeCode = other.RomSizeCode;
        if (other.RomSizeBytes.HasValue && (overwrite || !this.RomSizeBytes.HasValue)) this.RomSizeBytes = other.RomSizeBytes;
        if (other.RamSizeCode.HasValue && (overwrite || !this.RamSizeCode.HasValue)) this.RamSizeCode = other.RamSizeCode;
        if (other.DestinationCode.HasValue && (overwrite || !this.DestinationCode.HasValue)) this.DestinationCode = other.DestinationCode;
        if (other.Version.HasValue && (overwrite || !this.Version.HasValue)) this.Version = other.Version;
    }

    // Resolve a supported key alias and report whether that option has a value, including explicit false/zero values.
    public bool IsFieldSetByKey(string key)
    {
        string k = NormalizeKey(key);
        switch (k)
        {
            case "header_logo":
                return HeaderLogoEnabled.HasValue;
            case "title":
                return Title != null;
            case "cgb":
                return CgbFlag.HasValue;
            case "cart":
                return CartType.HasValue;
            case "romsize":
                return RomSizeCode.HasValue || RomSizeBytes.HasValue;
            case "ramsize":
                return RamSizeCode.HasValue;
            case "sgb":
                return SgbFlag.HasValue;
            case "dest":
                return DestinationCode.HasValue;
            case "version":
                return Version.HasValue;
        }
        return false;
    }

    // Dispatch a normalized key to its value parser; unknown keys and invalid values return an explanatory message.
    public bool TrySetByKey(string key, string value, out string err)
    {
        err = null;
        string k = NormalizeKey(key);
        string v = value ?? "";

        if (k == "title")
        {
            Title = v;
            return true;
        }
        if (k == "header_logo") return TrySetHeaderLogo(v, out err);
        if (k == "cgb") return TrySetCgb(v, out err);
        if (k == "cart") return TrySetCart(v, out err);
        if (k == "romsize") return TrySetRomSize(v, out err);
        if (k == "ramsize") return TrySetRamSize(v, out err);
        if (k == "sgb") return TrySetSgb(v, out err);
        if (k == "dest") return TrySetDest(v, out err);
        if (k == "version")
        {
            if (!byte.TryParse(v.Trim(), out byte bv))
            {
                err = "warning: version must be 0..255";
                return false;
            }
            Version = bv;
            return true;
        }

        err = "warning: unknown rom header key: " + key;
        return false;
    }

    // Normalize case, hyphens and the optional rom_ prefix, then resolve the supported header-field aliases.
    static string NormalizeKey(string key)
    {
        string k = (key ?? "").Trim();
        k = k.Replace('-', '_');
        k = k.ToLowerInvariant();
        if (k.StartsWith("rom_")) k = k.Substring(4);
        if (k == "romtitle" || k == "title") return "title";
        if (k == "rom_title" || k == "romtitle" || k == "game_title") return "title";
        if (k == "header_logo" || k == "headerlogo" || k == "logo" || k == "validation_logo") return "header_logo";
        if (k == "cgb") return "cgb";
        if (k == "cart" || k == "cartridge") return "cart";
        if (k == "romsize" || k == "rom_size") return "romsize";
        if (k == "ramsize" || k == "ram_size") return "ramsize";
        if (k == "sgb") return "sgb";
        if (k == "dest" || k == "destination") return "dest";
        if (k == "version" || k == "rom_version") return "version";
        // fall back to raw
        return k;
    }

    // Read a small JSON-like template using regular expressions rather than a complete JSON parser.
    // Unmatched text is not schema-validated, and comment stripping below does not distinguish quoted strings.
    public static RomHeaderOptions LoadFromJson(string path)
    {
        if (string.IsNullOrEmpty(path)) throw new ArgumentException("path is empty");
        if (!File.Exists(path)) throw new FileNotFoundException("file not found", path);
        // NOTE: This project targets .NET Framework 4.8, so we avoid System.Text.Json.
        // We only need a tiny subset: a top-level object with primitive values.
        // Supported value kinds: string, number, true, false.
        string json = IoUtil.ReadAllTextUtf8(path);

        // Strip C/C++-style comments before extracting primitive key/value pairs.
        json = Regex.Replace(json, @"//.*?$", "", RegexOptions.Multiline);
        json = Regex.Replace(json, @"/\*.*?\*/", "", RegexOptions.Singleline);

        // Quick sanity check: must contain an object.
        int lbrace = json.IndexOf('{');
        int rbrace = json.LastIndexOf('}');
        if (lbrace < 0 || rbrace < 0 || rbrace <= lbrace)
            throw new Exception("root must be an object");

        string body = json.Substring(lbrace + 1, rbrace - lbrace - 1);
        var opt = new RomHeaderOptions();

        // Match "key": value (value = "..." | number | true | false)
        // This is intentionally simple; escapes are minimally handled for \" and \\.
        var rx = new Regex(
            "\"(?<k>[^\"\\\\]*(?:\\\\.[^\"\\\\]*)*)\"\\s*:\\s*(?<v>\"[^\"]*(?:\\\\.[^\"]*)*\"|-?\\d+|true|false)",
            RegexOptions.IgnoreCase);

        foreach (Match m in rx.Matches(body))
        {
            string key = UnescapeJsonString(m.Groups["k"].Value);
            string rawVal = m.Groups["v"].Value.Trim();
            string val;
            if (rawVal.Equals("true", StringComparison.OrdinalIgnoreCase)) val = "on";
            else if (rawVal.Equals("false", StringComparison.OrdinalIgnoreCase)) val = "off";
            else if (rawVal.Length >= 2 && rawVal[0] == '"' && rawVal[rawVal.Length - 1] == '"')
                val = UnescapeJsonString(rawVal.Substring(1, rawVal.Length - 2));
            else
                val = rawVal; // number

            if (!opt.TrySetByKey(key, val, out string err))
                throw new Exception($"invalid '{key}': {err}");
        }

        return opt;
    }

    // Decode the supported quote, backslash and whitespace escapes; unrecognized escapes retain only the escaped character.
    static string UnescapeJsonString(string s)
    {
        if (s == null) return null;
        // Minimal unescape: \\ \" \n \r \t. Others are passed through.
        var sb = new StringBuilder(s.Length);
        for (int i = 0; i < s.Length; i++)
        {
            char c = s[i];
            if (c == '\\' && i + 1 < s.Length)
            {
                char n = s[++i];
                switch (n)
                {
                    case '\\': sb.Append('\\'); break;
                    case '"': sb.Append('"'); break;
                    case 'n': sb.Append('\n'); break;
                    case 'r': sb.Append('\r'); break;
                    case 't': sb.Append('\t'); break;
                    default: sb.Append(n); break;
                }
            }
            else sb.Append(c);
        }
        return sb.ToString();
    }

    // Map accepted monochrome, compatible and color-only spellings to their GB header flag values.
    public bool TrySetCgb(string s, out string err)
    {
        err = null;
        s = (s ?? "").Trim().ToLowerInvariant();
        // Accept legacy pragma value.
        if (s == "cgb_support") s = "compatible";
        if (s == "dmg" || s == "gb" || s == "dmg_only")
        {
            CgbFlag = 0x00;
            return true;
        }
        if (s == "cgb" || s == "support" || s == "compatible")
        {
            CgbFlag = 0x80;
            return true;
        }
        if (s == "cgb_only" || s == "only")
        {
            CgbFlag = 0xC0;
            return true;
        }
        err = "error: --cgb must be dmg|cgb|cgb_only";
        return false;
    }

    // Parse whether the GB boot-header validation bytes should be emitted or replaced with 0xFF.
    public bool TrySetHeaderLogo(string s, out string err)
    {
        err = null;
        s = (s ?? "").Trim().ToLowerInvariant();
        if (s == "on" || s == "1" || s == "true" || s == "yes" || s == "default")
        {
            HeaderLogoEnabled = true;
            return true;
        }
        if (s == "off" || s == "0" || s == "false" || s == "no" || s == "none" || s == "omit")
        {
            HeaderLogoEnabled = false;
            return true;
        }
        err = "error: --header-logo must be on|off";
        return false;
    }

    // Accept the supported boolean/legacy spellings and encode enabled SGB support as 0x03.
    public bool TrySetSgb(string s, out string err)
    {
        err = null;
        s = (s ?? "").Trim().ToLowerInvariant();
        // Accept legacy pragma value.
        if (s == "sgb_support") s = "on";
        if (s == "on" || s == "1" || s == "true")
        {
            SgbFlag = 0x03;
            return true;
        }
        if (s == "off" || s == "0" || s == "false")
        {
            SgbFlag = 0x00;
            return true;
        }
        err = "error: --sgb must be on|off";
        return false;
    }

    // Map Japanese and non-Japanese destination aliases to the corresponding single-byte flag.
    public bool TrySetDest(string s, out string err)
    {
        err = null;
        s = (s ?? "").Trim().ToLowerInvariant();
        // Accept legacy pragma values.
        if (s == "dest_jp") s = "jp";
        if (s == "dest_nonjp") s = "nonjp";
        if (s == "jp" || s == "japan" || s == "0")
        {
            DestinationCode = 0x00;
            return true;
        }
        if (s == "nonjp" || s == "non-jp" || s == "world" || s == "1")
        {
            DestinationCode = 0x01;
            return true;
        }
        err = "error: --dest must be jp|nonjp";
        return false;
    }

    // Select only the listed ROM-only/MBC variants; this helper does not infer RAM size or cartridge compatibility.
    public bool TrySetCart(string s, out string err)
    {
        err = null;
        s = (s ?? "").Trim().ToLowerInvariant();
        // Minimal set (expand later as needed)
        switch (s)
        {
            case "romonly":
            case "rom_only":
            case "rom":
                CartType = 0x00; return true;
            case "mbc1":
                CartType = 0x01; return true;
            case "mbc1_ram":
                CartType = 0x02; return true;
            case "mbc1_ram_batt":
            case "mbc1_ram_battery":
                CartType = 0x03; return true;
            case "mbc3":
                CartType = 0x11; return true;
            case "mbc3_ram":
                CartType = 0x12; return true;
            case "mbc3_ram_batt":
            case "mbc3_ram_battery":
                CartType = 0x13; return true;
            case "mbc5":
                CartType = 0x19; return true;
            case "mbc5_ram":
                CartType = 0x1A; return true;
            case "mbc5_ram_batt":
            case "mbc5_ram_battery":
                CartType = 0x1B; return true;
        }

        err = "error: --cart unsupported value: " + s + " (try romonly|mbc1|mbc3|mbc5 ...)";
        return false;
    }

    // Record both the header code and byte count for a supported power-of-two ROM capacity.
    public bool TrySetRomSize(string s, out string err)
    {
        err = null;
        s = NormalizeSizeToken(s);
        // Header codes per Pan Docs:
        // 0=32K,1=64K,2=128K,3=256K,4=512K,5=1M,6=2M,7=4M,8=8M
        if (s == "32k") { RomSizeCode = 0x00; RomSizeBytes = 32 * 1024; return true; }
        if (s == "64k") { RomSizeCode = 0x01; RomSizeBytes = 64 * 1024; return true; }
        if (s == "128k") { RomSizeCode = 0x02; RomSizeBytes = 128 * 1024; return true; }
        if (s == "256k") { RomSizeCode = 0x03; RomSizeBytes = 256 * 1024; return true; }
        if (s == "512k") { RomSizeCode = 0x04; RomSizeBytes = 512 * 1024; return true; }
        if (s == "1m") { RomSizeCode = 0x05; RomSizeBytes = 1024 * 1024; return true; }
        if (s == "2m") { RomSizeCode = 0x06; RomSizeBytes = 2 * 1024 * 1024; return true; }
        if (s == "4m") { RomSizeCode = 0x07; RomSizeBytes = 4 * 1024 * 1024; return true; }
        if (s == "8m") { RomSizeCode = 0x08; RomSizeBytes = 8 * 1024 * 1024; return true; }

        err = "error: --romsize must be 32k|64k|128k|256k|512k|1m|2m|4m|8m";
        return false;
    }

    // Encode the supported external-RAM capacity, including the nonsequential 64/128 KiB header codes.
    public bool TrySetRamSize(string s, out string err)
    {
        err = null;
        s = NormalizeSizeToken(s);
        // Header codes:
        // 0=none, 1=2K, 2=8K, 3=32K, 4=128K, 5=64K
        if (s == "none" || s == "0") { RamSizeCode = 0x00; return true; }
        if (s == "2k") { RamSizeCode = 0x01; return true; }
        if (s == "8k") { RamSizeCode = 0x02; return true; }
        if (s == "32k") { RamSizeCode = 0x03; return true; }
        if (s == "64k") { RamSizeCode = 0x05; return true; }
        if (s == "128k") { RamSizeCode = 0x04; return true; }

        err = "error: --ramsize must be none|2k|8k|32k|64k|128k";
        return false;
    }

    // Normalize case and remove spaces, bytes and b spellings before comparing the supported size tokens.
    static string NormalizeSizeToken(string s)
    {
        s = (s ?? "").Trim().ToLowerInvariant();
        s = s.Replace(" ", "");
        s = s.Replace("bytes", "");
        s = s.Replace("b", "");
        if (s.EndsWith("kb")) s = s.Substring(0, s.Length - 2) + "k";
        if (s.EndsWith("mb")) s = s.Substring(0, s.Length - 2) + "m";
        return s;
    }
}

// Apply GB header fields and checksum updates after assembly; ROM content outside the header is preserved except padding.
public static class RomHeaderPatcher
{
    // Header offsets
    const int OFF_LOGO = 0x0104;
    const int OFF_TITLE = 0x0134;
    const int LEN_LOGO = 48;
    const int LEN_TITLE = 16;
    const int OFF_CGB = 0x0143;
    const int OFF_SGB = 0x0146;
    const int OFF_CART = 0x0147;
    const int OFF_ROMSIZE = 0x0148;
    const int OFF_RAMSIZE = 0x0149;
    const int OFF_DEST = 0x014A;
    const int OFF_VERSION = 0x014C;
    const int OFF_HDRCHK = 0x014D;
    const int OFF_GLOBCHK = 0x014E;

    // GB-compatible boot header validation bytes. They are emitted by default
    // for hardware-compatible ROMs; use --no-header-logo or
    // #pragma header_logo off for development outputs that should omit them.
    internal static readonly byte[] RequiredHeaderLogoBytes = new byte[] {
        0xCE, 0xED, 0x66, 0x66, 0xCC, 0x0D, 0x00, 0x0B, 0x03, 0x73, 0x00, 0x83, 0x00, 0x0C, 0x00, 0x0D,
        0x00, 0x08, 0x11, 0x1F, 0x88, 0x89, 0x00, 0x0E, 0xDC, 0xCC, 0x6E, 0xE6, 0xDD, 0xDD, 0xD9, 0x99,
        0xBB, 0xBB, 0x67, 0x63, 0x6E, 0x0E, 0xEC, 0xCC, 0xDD, 0xDC, 0x99, 0x9F, 0xBB, 0xB9, 0x33, 0x3E
    };

    // Decode standard and legacy GB ROM-size codes; unknown codes provide no expected byte count.
    static int? RomSizeBytesFromCode(byte code)
    {
        switch (code)
        {
            case 0x00: return 32 * 1024;
            case 0x01: return 64 * 1024;
            case 0x02: return 128 * 1024;
            case 0x03: return 256 * 1024;
            case 0x04: return 512 * 1024;
            case 0x05: return 1024 * 1024;
            case 0x06: return 2 * 1024 * 1024;
            case 0x07: return 4 * 1024 * 1024;
            case 0x08: return 8 * 1024 * 1024;
            // Pan Docs legacy extension codes.
            case 0x52: return 1152 * 1024;
            case 0x53: return 1280 * 1024;
            case 0x54: return 1536 * 1024;
            default: return null;
        }
    }

    // Read an existing GB-format ROM, validate requested capacity and ROM-only limits, then patch selected fields.
    // Only after those checks pass are both checksums recomputed and the file overwritten.
    public static void PatchFile(string romPath, RomHeaderOptions opt)
    {
        if (opt == null || !opt.HasAny) return;
        if (string.IsNullOrEmpty(romPath) || !File.Exists(romPath))
        {
            Program.Error("error: ROM header patch: output ROM not found: {0}", romPath);
            return;
        }

        byte[] rom = File.ReadAllBytes(romPath);
        if (rom.Length < 0x150)
        {
            Program.Error("error: ROM header patch: ROM too small ({0} bytes)", rom.Length);
            return;
        }

        int originalLen = rom.Length;
        // Prefer the explicit byte count; otherwise derive capacity from the supplied code, not from the existing header bytes.
        int? expectedRomSize = opt.RomSizeBytes;
        if (!expectedRomSize.HasValue && opt.RomSizeCode.HasValue)
        {
            expectedRomSize = RomSizeBytesFromCode(opt.RomSizeCode.Value);
        }

        // --- Consistency checks and optional padding ---
        if (expectedRomSize.HasValue)
        {
            int expected = expectedRomSize.Value;
            if (rom.Length > expected)
            {
                Program.Error("error: ROM size {0} bytes exceeds header/--romsize expectation ({1} bytes)", rom.Length, expected);
                return;
            }
            if (rom.Length < expected)
            {
                if (opt.RomSizeBytes.HasValue)
                    Program.Warning("warning: ROM size {0} bytes smaller than --romsize ({1} bytes); padding with 0xFF", rom.Length, expected);
                else
                    Program.Warning("warning: ROM size {0} bytes smaller than header ROM size code ({1} bytes); padding with 0xFF", rom.Length, expected);
                Array.Resize(ref rom, expected);
                for (int i = originalLen; i < expected; i++) rom[i] = 0xFF;
            }
        }

        // ROMONLY guard
        if (opt.CartType.HasValue && opt.CartType.Value == 0x00)
        {
            int limit = 32 * 1024;
            if (rom.Length > limit)
            {
                Program.Error("error: --cart=romonly cannot be used with ROM size >32K (actual {0} bytes)", rom.Length);
                return;
            }
            if (expectedRomSize.HasValue && expectedRomSize.Value > limit)
            {
                Program.Error("error: --cart=romonly incompatible with --romsize >32K");
                return;
            }
        }

        // --- Patch header fields ---
        if (opt.HeaderLogoEnabled.HasValue)
        {
            WriteHeaderLogo(rom, opt.HeaderLogoEnabled.Value);
        }
        if (opt.Title != null)
        {
            WriteTitle(rom, opt.Title);
        }
        if (opt.CgbFlag.HasValue) rom[OFF_CGB] = opt.CgbFlag.Value;
        if (opt.SgbFlag.HasValue) rom[OFF_SGB] = opt.SgbFlag.Value;
        if (opt.CartType.HasValue) rom[OFF_CART] = opt.CartType.Value;
        if (opt.RomSizeCode.HasValue) rom[OFF_ROMSIZE] = opt.RomSizeCode.Value;
        if (opt.RamSizeCode.HasValue) rom[OFF_RAMSIZE] = opt.RamSizeCode.Value;
        if (opt.DestinationCode.HasValue) rom[OFF_DEST] = opt.DestinationCode.Value;
        if (opt.Version.HasValue) rom[OFF_VERSION] = opt.Version.Value;

        // --- Recompute checksums ---
        rom[OFF_HDRCHK] = ComputeHeaderChecksum(rom);
        ushort g = ComputeGlobalChecksum(rom);
        rom[OFF_GLOBCHK] = (byte)((g >> 8) & 0xFF);
        rom[OFF_GLOBCHK + 1] = (byte)(g & 0xFF);

        File.WriteAllBytes(romPath, rom);
    }

    // Write or clear only the 48-byte validation field; checksum updates are the caller's responsibility.
    internal static void WriteHeaderLogo(byte[] rom, bool enabled)
    {
        if (rom == null || rom.Length < OFF_LOGO + LEN_LOGO) return;
        if (enabled)
        {
            Array.Copy(RequiredHeaderLogoBytes, 0, rom, OFF_LOGO, RequiredHeaderLogoBytes.Length);
            return;
        }

        for (int i = 0; i < LEN_LOGO; i++) rom[OFF_LOGO + i] = 0xFF;
    }

    // Replace the full 16-byte legacy title field with printable ASCII and zero padding.
    // This includes offset 0x0143; a separately supplied CGB flag is written after the title.
    static void WriteTitle(byte[] rom, string title)
    {
        // Use ASCII; non-ASCII -> '?'
        byte[] bytes = Encoding.ASCII.GetBytes(title ?? "");
        for (int i = 0; i < LEN_TITLE; i++)
        {
            byte b = 0x00;
            if (i < bytes.Length)
            {
                b = bytes[i];
                if (b < 0x20 || b > 0x7E) b = (byte)'?';
            }
            rom[OFF_TITLE + i] = b;
        }
    }

    // Compute the wrapping eight-bit subtractive checksum over the title and header fields through 0x014C.
    static byte ComputeHeaderChecksum(byte[] rom)
    {
        // x = 0; for i=0x0134..0x014C: x = x - rom[i] - 1
        int x = 0;
        for (int i = 0x0134; i <= 0x014C; i++)
        {
            x = (x - rom[i] - 1) & 0xFF;
        }
        return (byte)x;
    }

    // Sum the complete padded image modulo 65536, excluding the two stored global-checksum bytes.
    static ushort ComputeGlobalChecksum(byte[] rom)
    {
        int sum = 0;
        for (int i = 0; i < rom.Length; i++)
        {
            // Treat checksum bytes as 0
            if (i == 0x014E || i == 0x014F) continue;
            sum += rom[i];
        }
        return (ushort)(sum & 0xFFFF);
    }
}




