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

    public SettingsStore()
    {
        var dir = Path.Combine(
            Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData),
            "Cascade");
        Directory.CreateDirectory(dir);
        FilePath = Path.Combine(dir, "settings.json");
    }

    public string? ReadSafely()
    {
        try
        {
            return File.Exists(FilePath) ? File.ReadAllText(FilePath) : null;
        }
        catch
        {
            return null;
        }
    }

    public void WriteSafely(string json)
    {
        try
        {
            File.WriteAllText(FilePath, json);
        }
        catch
        {
            // Persistence is best-effort; never block the dispatch loop on disk IO.
        }
    }
}
