using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Text;
using System.Threading;

static class IoUtil
{
    public static readonly Encoding Utf8NoBom = new UTF8Encoding(false);
    public static readonly Encoding Utf8BomAware = new UTF8Encoding(false, false);

    const int DefaultRetryCount = 6;
    const int DefaultRetryDelayMs = 70;

    public static string ReadAllTextUtf8(string path)
    {
        return File.ReadAllText(path, Encoding.UTF8);
    }

    public static string[] ReadAllLinesUtf8(string path)
    {
        return File.ReadAllLines(path, Encoding.UTF8);
    }

    public static string WriteAllTextUtf8Robust(string path, string text, bool allowAlternatePath = true)
    {
        var bytes = Utf8NoBom.GetBytes(text ?? "");
        return WriteAllBytesRobust(path, bytes, allowAlternatePath);
    }

    public static string WriteAllLinesUtf8Robust(string path, IEnumerable<string> lines, bool allowAlternatePath = true)
    {
        string text = string.Join(Environment.NewLine, lines ?? Enumerable.Empty<string>());
        return WriteAllTextUtf8Robust(path, text, allowAlternatePath);
    }

    public static string WriteAllBytesRobust(string path, byte[] bytes, bool allowAlternatePath = true)
    {
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

    public static void CopyFileRobust(string src, string dst, bool overwrite = true)
    {
        EnsureParentDirectory(dst);
        RetryIo(() => File.Copy(src, dst, overwrite));
    }

    public static void DeleteFileIfExistsRobust(string path)
    {
        if (!File.Exists(path)) return;
        RetryIo(() =>
        {
            if (File.Exists(path)) File.Delete(path);
        });
    }

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
            Thread.Sleep(DefaultRetryDelayMs * (i + 1));
        }
        if (last != null) throw last;
    }

    static string BuildAlternatePath(string path)
    {
        string dir = Path.GetDirectoryName(path);
        if (string.IsNullOrEmpty(dir)) dir = ".";
        string file = Path.GetFileNameWithoutExtension(path);
        string ext = Path.GetExtension(path);
        string suffix = ".lockretry_" + DateTime.UtcNow.ToString("yyyyMMdd_HHmmssfff");
        return Path.Combine(dir, file + suffix + ext);
    }

    static void EnsureParentDirectory(string path)
    {
        string dir = Path.GetDirectoryName(path);
        if (!string.IsNullOrEmpty(dir))
        {
            Directory.CreateDirectory(dir);
        }
    }
}




