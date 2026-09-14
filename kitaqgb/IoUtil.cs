using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Text;
using System.Threading;

// Shared file I/O for compiler artifacts. Robust writes may choose an alternate
// filename after a failure, so callers must use the returned path in reports.
static class IoUtil
{
    public static readonly Encoding Utf8NoBom = new UTF8Encoding(false);
    public static readonly Encoding Utf8BomAware = new UTF8Encoding(false, false);

    const int DefaultRetryCount = 6;
    const int DefaultRetryDelayMs = 70;

    // Read text using UTF-8 with the framework's BOM detection and propagate read failures.
    public static string ReadAllTextUtf8(string path)
    {
        return File.ReadAllText(path, Encoding.UTF8);
    }

    // Read UTF-8 source lines without their newline separators for source diagnostics.
    public static string[] ReadAllLinesUtf8(string path)
    {
        return File.ReadAllLines(path, Encoding.UTF8);
    }

    // Encode text as UTF-8 without a BOM; null text becomes an empty file.
    // Return the actual destination chosen by the byte-writing helper.
    public static string WriteAllTextUtf8Robust(string path, string text, bool allowAlternatePath = true)
    {
        var bytes = Utf8NoBom.GetBytes(text ?? "");
        return WriteAllBytesRobust(path, bytes, allowAlternatePath);
    }

    // Join lines using the host newline convention, without adding a final newline,
    // then write them through the robust UTF-8 path.
    public static string WriteAllLinesUtf8Robust(string path, IEnumerable<string> lines, bool allowAlternatePath = true)
    {
        string text = string.Join(Environment.NewLine, lines ?? Enumerable.Empty<string>());
        return WriteAllTextUtf8Robust(path, text, allowAlternatePath);
    }

    // Create/truncate the requested artifact and retry transient I/O/access errors.
    // If enabled, a failed primary write falls back to a timestamped sibling path.
    // Writes are direct, not an atomic temporary-file replacement transaction.
    public static string WriteAllBytesRobust(string path, byte[] bytes, bool allowAlternatePath = true)
    {
        // Parent-directory creation happens before the fallback try block; a failure here does not try an alternate path.
        EnsureParentDirectory(path);
        try
        {
            RetryIo(() =>
            {
                using (var fs = new FileStream(path, FileMode.Create, FileAccess.Write, FileShare.Read))
                {
                    fs.Write(bytes, 0, bytes.Length);
                }
            });
            return path;
        }
        // The outer fallback catches any primary-write exception, not only locking errors; the warning is generic.
        catch
        {
            if (!allowAlternatePath) throw;
            string alt = BuildAlternatePath(path);
            EnsureParentDirectory(alt);
            RetryIo(() =>
            {
                using (var fs = new FileStream(alt, FileMode.Create, FileAccess.Write, FileShare.Read))
                {
                    fs.Write(bytes, 0, bytes.Length);
                }
            });
            Console.Error.WriteLine("warning KQ0000: output is locked, wrote alternate file: " + alt);
            return alt;
        }
    }

    // Ensure the destination directory exists, then retry the requested file copy.
    public static void CopyFileRobust(string src, string dst, bool overwrite = true)
    {
        EnsureParentDirectory(dst);
        RetryIo(() => File.Copy(src, dst, overwrite));
    }

    // Delete an existing file with retries; check again inside each attempt so an
    // already-removed file does not fail the operation.
    public static void DeleteFileIfExistsRobust(string path)
    {
        if (!File.Exists(path)) return;
        RetryIo(() =>
        {
            if (File.Exists(path)) File.Delete(path);
        });
    }

    // Retry only I/O and access-denied exceptions with linearly increasing delays.
    // Other exceptions escape immediately; exhausting attempts rethrows the last failure.
    static void RetryIo(Action action)
    {
        Exception last = null;
        for (int i = 0; i < DefaultRetryCount; i++)
        {
            try
            {
                action();
                return;
            }
            catch (IOException ex)
            {
                last = ex;
            }
            catch (UnauthorizedAccessException ex)
            {
                last = ex;
            }
            // Delay increases on each failure, including after the last attempt before its exception is rethrown.
            Thread.Sleep(DefaultRetryDelayMs * (i + 1));
        }
        if (last != null) throw last;
    }

    // Insert a UTC millisecond timestamp before the extension while keeping the
    // same directory. The timestamp is a fallback name, not an exclusive reservation.
    static string BuildAlternatePath(string path)
    {
        string dir = Path.GetDirectoryName(path);
        if (string.IsNullOrEmpty(dir)) dir = ".";
        string file = Path.GetFileNameWithoutExtension(path);
        string ext = Path.GetExtension(path);
        string suffix = ".lockretry_" + DateTime.UtcNow.ToString("yyyyMMdd_HHmmssfff");
        return Path.Combine(dir, file + suffix + ext);
    }

    // Create the parent directory when the path has one; a bare filename uses the current directory.
    static void EnsureParentDirectory(string path)
    {
        string dir = Path.GetDirectoryName(path);
        if (!string.IsNullOrEmpty(dir))
        {
            Directory.CreateDirectory(dir);
        }
    }
}




