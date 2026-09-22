// Copyright (c) 2026 DAISUKE OBA. MIT License; see the repository LICENSE.
// Independently authored ZX0 v2 forward-format encoder and bounded decoder.
// ZX0 format designed by Einar Saukas. No upstream implementation is included.
using System;
using System.Collections.Generic;
using System.IO;
using System.Text;

public static class KitaqZx0
{
    // Control bits share bytes across literal/match data. A new-offset byte
    // lends its low bit to the first control bit of the following length.
    sealed class BitOutput
    {
        internal readonly List<byte> Data = new List<byte>();
        int controlIndex, mask, borrowed = -1;
        internal void Bit(int bit)
        {
            if (borrowed >= 0) { Data[borrowed] |= (byte)bit; borrowed = -1; return; }
            if (mask == 0) { controlIndex = Data.Count; Data.Add(0); mask = 128; }
            if (bit != 0) Data[controlIndex] |= (byte)mask;
            mask >>= 1;
        }
        internal void Gamma(int value, bool invert)
        {
            int top = 1;
            while (top <= value / 2) top <<= 1;
            for (top >>= 1; top != 0; top >>= 1) {
                Bit(0); Bit(((value & top) != 0 ? 1 : 0) ^ (invert ? 1 : 0));
            }
            Bit(1);
        }
        internal void OffsetByte(int value) { Data.Add((byte)value); borrowed = Data.Count - 1; }
    }

    static int GammaBits(int n)
    {
        int count = 1;
        while ((n >>= 1) != 0) count += 2;
        return count;
    }

    // A bounded hash-chain search makes this a fast, non-optimal encoder.
    // Every candidate refers only to already-produced bytes; overlapping
    // matches are compared against the original input to permit runs.
    public static byte[] Compress(byte[] input)
    {
        if (input == null || input.Length == 0 || input.Length > 65535)
            throw new ArgumentException("ZX0 input must contain 1..65535 bytes.");
        var writer = new BitOutput();
        int[] heads = new int[65536], previous = new int[input.Length];
        for (int i = 0; i < heads.Length; i++) heads[i] = -1;
        int at = 1, indexed = 0, literalStart = 0, lastOffset = 1;
        bool first = true;
        while (at < input.Length) {
            while (indexed < at) {
                if (indexed + 1 < input.Length) {
                    int key = (input[indexed] << 8) | input[indexed + 1];
                    previous[indexed] = heads[key]; heads[key] = indexed;
                }
                indexed++;
            }
            int bestLength = 0, bestOffset = 0, bestSaving = 0;
            if (at + 1 < input.Length) {
                int candidate = heads[(input[at] << 8) | input[at + 1]], attempts = 0;
                while (candidate >= 0 && at - candidate <= 32640 && attempts++ < 256) {
                    int distance = at - candidate, length = 2;
                    while (at + length < input.Length && input[at + length] == input[at + length - distance]) length++;
                    bool reuse = literalStart < at && distance == lastOffset;
                    int cost = reuse ? 1 + GammaBits(length) : 8 + GammaBits((distance - 1) / 128 + 1) + GammaBits(length - 1);
                    int saving = length * 8 - cost;
                    if (saving > bestSaving || (saving == bestSaving && length > bestLength)) {
                        bestLength = length; bestOffset = distance; bestSaving = saving;
                    }
                    if (at + length == input.Length) break;
                    candidate = previous[candidate];
                }
            }
            if (bestLength < 2 || bestSaving <= 0) { at++; continue; }
            bool afterLiteral = literalStart < at;
            if (afterLiteral) {
                if (!first) writer.Bit(0);
                writer.Gamma(at - literalStart, false);
                for (int i = literalStart; i < at; i++) writer.Data.Add(input[i]);
                first = false;
            }
            if (afterLiteral && bestOffset == lastOffset) {
                writer.Bit(0); writer.Gamma(bestLength, false);
            } else {
                writer.Bit(1); writer.Gamma((bestOffset - 1) / 128 + 1, true);
                writer.OffsetByte((127 - (bestOffset - 1) % 128) << 1);
                writer.Gamma(bestLength - 1, false); lastOffset = bestOffset;
            }
            at += bestLength; literalStart = at;
        }
        if (literalStart < input.Length) {
            if (!first) writer.Bit(0);
            writer.Gamma(input.Length - literalStart, false);
            for (int i = literalStart; i < input.Length; i++) writer.Data.Add(input[i]);
        }
        writer.Bit(1); writer.Gamma(256, true);
        return writer.Data.ToArray();
    }

    sealed class InputCursor
    {
        internal byte[] Data;
        internal int At;
        int bits, remaining, borrowed = -1;
        internal byte Byte()
        {
            if (At == Data.Length) throw new InvalidDataException("Truncated ZX0 stream.");
            return Data[At++];
        }
        internal int Bit()
        {
            if (borrowed >= 0) { int value = borrowed; borrowed = -1; return value; }
            if (remaining == 0) { bits = Byte(); remaining = 8; }
            int bit = (bits >> 7) & 1; bits <<= 1; remaining--; return bit;
        }
        internal void Lend(int lowBit) { borrowed = lowBit; }
        internal int Number(bool invert)
        {
            int n = 1;
            while (Bit() == 0) {
                if (n >= 32768) throw new InvalidDataException("ZX0 integer exceeds 16 bits.");
                n = n * 2 + (Bit() ^ (invert ? 1 : 0));
            }
            return n;
        }
    }

    // Decode into a caller-selected maximum size. Prefix dictionaries,
    // backwards streams and the classic v1 offset coding are not accepted.
    public static byte[] Decompress(byte[] packed, int capacity)
    {
        if (packed == null || capacity < 0 || capacity > 65535) throw new ArgumentException("Invalid decoder arguments.");
        var input = new InputCursor { Data = packed };
        var output = new List<byte>();
        int phase = 0, distance = 1;
        for (;;) {
            int count;
            if (phase == 2) {
                int upper = input.Number(true);
                if (upper == 256) {
                    if (input.At != packed.Length) throw new InvalidDataException("Trailing bytes after ZX0 end marker.");
                    return output.ToArray();
                }
                if (upper > 255) throw new InvalidDataException("Invalid ZX0 offset.");
                int low = input.Byte(); distance = upper * 128 - (low >> 1);
                input.Lend(low & 1); count = input.Number(false) + 1;
            } else count = input.Number(false);
            if (count > capacity - output.Count) throw new InvalidDataException("ZX0 output exceeds capacity.");
            if (phase == 0) {
                for (int n = 0; n < count; n++) output.Add(input.Byte());
                phase = input.Bit() == 0 ? 1 : 2;
            } else {
                if (distance > output.Count) throw new InvalidDataException("ZX0 match precedes output.");
                for (int n = 0; n < count; n++) output.Add(output[output.Count - distance]);
                phase = input.Bit() == 0 ? 0 : 2;
            }
        }
    }

    // Count/value RLE uses the library convention: 1..255 repeats, then a
    // single zero terminator. This is an alternative codec, not a ZX0 stream.
    public static byte[] Rle(byte[] data)
    {
        var result = new List<byte>();
        for (int i = 0; i < data.Length;) {
            int n = 1; while (n < 255 && i + n < data.Length && data[i + n] == data[i]) n++;
            result.Add((byte)n); result.Add(data[i]); i += n;
        }
        result.Add(0); return result.ToArray();
    }

    // KQA1 wraps the winning payload with an explicit codec and both sizes.
    // Compare complete payload sizes; ties favor raw, then RLE, then ZX0.
    public static byte[] Automatic(byte[] input, out string chosen)
    {
        if (input == null || input.Length > 65535) throw new ArgumentException("Asset must contain 0..65535 bytes.");
        byte[] payload = input; int codec = 0; chosen = "raw";
        byte[] rle = Rle(input);
        if (rle.Length < payload.Length) { payload = rle; codec = 1; chosen = "rle"; }
        if (input.Length != 0) {
            byte[] zx0 = Compress(input);
            if (zx0.Length < payload.Length) { payload = zx0; codec = 2; chosen = "zx0"; }
        }
        if (payload.Length > 65535 - 9)
            throw new ArgumentException("Packed asset including the KQA1 header exceeds 65535 bytes; split the asset.");
        byte[] result = new byte[payload.Length + 9];
        result[0] = 75; result[1] = 81; result[2] = 65; result[3] = 49; result[4] = (byte)codec;
        result[5] = (byte)input.Length; result[6] = (byte)(input.Length >> 8);
        result[7] = (byte)payload.Length; result[8] = (byte)(payload.Length >> 8);
        Array.Copy(payload, 0, result, 9, payload.Length); return result;
    }

    public static int Main(string[] args)
    {
        try {
            var paths = new List<string>(); string format = "zx0", header = null; bool decode = false;
            foreach (string arg in args) {
                if (arg == "--decompress") decode = true;
                else if (arg.StartsWith("--format=")) format = arg.Substring(9);
                else if (arg.StartsWith("--header=")) header = arg.Substring(9);
                else if (arg.StartsWith("--")) throw new ArgumentException("Unknown option: " + arg);
                else paths.Add(arg);
            }
            string toolName = Path.GetFileNameWithoutExtension(System.Reflection.Assembly.GetExecutingAssembly().Location);
            if (paths.Count != 2) throw new ArgumentException("Usage: " + toolName + " input output [--format=zx0|raw|rle|auto] [--header=identifier] [--decompress]");
            byte[] input = File.ReadAllBytes(paths[0]); byte[] result; string codec = format;
            if (decode) {
                if (format != "zx0" || header != null) throw new ArgumentException("--decompress accepts a ZX0 v2 stream and binary output only.");
                result = Decompress(input, 65535);
            } else if (format == "auto") result = Automatic(input, out codec);
            else if (format == "zx0") result = Compress(input);
            else if (format == "rle") { if (input.Length > 65535) throw new ArgumentException("Asset exceeds 65535 bytes."); result = Rle(input); }
            else if (format == "raw") { if (input.Length > 65535) throw new ArgumentException("Asset exceeds 65535 bytes."); result = input; }
            else throw new ArgumentException("Unknown format: " + format);
            if (!decode && result.Length > 65535) throw new ArgumentException("Packed data exceeds the 16-bit target size; split the asset.");
            if (Path.GetFullPath(paths[0]).Equals(Path.GetFullPath(paths[1]), StringComparison.OrdinalIgnoreCase)) throw new ArgumentException("Input and output must differ.");
            if (header == null) File.WriteAllBytes(paths[1], result);
            else {
                if (!System.Text.RegularExpressions.Regex.IsMatch(header, "^[A-Za-z_][A-Za-z0-9_]*$")) throw new ArgumentException("Invalid C identifier.");
                var text = new StringBuilder("// Generated asset data. Original asset rights remain with its author.\n#pragma once\n");
                text.Append("__prg_rom u8 ").Append(header).Append("[] = {\n");
                // C has no portable zero-length array. Keep a storage byte but
                // report the logical payload length as zero in the size macro.
                if (result.Length == 0) text.Append("0");
                for (int i = 0; i < result.Length; i++) { text.Append(result[i]).Append(','); if (i % 24 == 23) text.Append('\n'); }
                text.Append("\n};\n#define ").Append(header).Append("_SIZE ").Append(result.Length).Append("\n#define ").Append(header).Append("_RAW_SIZE ").Append(input.Length).Append('\n');
                File.WriteAllText(paths[1], text.ToString(), new UTF8Encoding(false));
            }
            Console.WriteLine("{0}: {1} -> {2} bytes{3}", codec, input.Length, result.Length, format == "auto" ? " (includes 9-byte KQA1 header)" : "");
            return 0;
        } catch (Exception e) { Console.Error.WriteLine(e.Message); return 1; }
    }
}
