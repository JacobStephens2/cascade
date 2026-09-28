using System;
using System.IO;

namespace Cascade.Services;

/// <summary>
/// Stores the cascade-core settings blob (JSON produced by
/// <c>persistSettings</c>) as <c>%LOCALAPPDATA%\Cascade\settings.json</c>.
///
/// The JSON shape is opaque to C# — the Rust core owns the schema. We just
/// round-trip the bytes. Same wire format as the web (localStorage), Android
/// (DataStore), and macOS (Application Support) shells.
/// </summary>
public sealed class SettingsStore
{
    public string FilePath { get; }

    /// Lifetime listening ledger — a separate file from settings, so the two
    /// evolve and fail independently (mirrors cascade.listening.v1 on web).
    public string ListeningFilePath { get; }

    public SettingsStore()
    {
        var dir = Path.Combine(
            Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData),
            "Cascade");
        Directory.CreateDirectory(dir);
        FilePath = Path.Combine(dir, "settings.json");
        ListeningFilePath = Path.Combine(dir, "listening.json");
    }

    public string? ReadSafely() => ReadFile(FilePath);

    public void WriteSafely(string json) => WriteFile(FilePath, json);

    public string? ReadListeningSafely() => ReadFile(ListeningFilePath);

    public void WriteListeningSafely(string json) => WriteFile(ListeningFilePath, json);

    private static string? ReadFile(string path)
    {
        try
        {
            return File.Exists(path) ? File.ReadAllText(path) : null;
        }
        catch
        {
            return null;
        }
    }

    private static void WriteFile(string path, string json)
    {
        // Write a sibling temp file and swap it in, so a crash mid-write leaves
        // the previous blob intact rather than a torn one. Matters most for the
        // listening blob: the one write that rotates the device id and zeroes
        // the slot must land whole or not at all (same as the Apple shell's
        // `.atomic` write).
        var tmp = path + ".tmp";
        try
        {
            File.WriteAllText(tmp, json);
            File.Move(tmp, path, overwrite: true);
        }
        catch (Exception e)
        {
            // Persistence is best-effort; never block the dispatch loop on disk
            // IO. But don't fail silently.
            System.Diagnostics.Debug.WriteLine($"[Cascade] write to {path} failed: {e}");
        }
    }
}
