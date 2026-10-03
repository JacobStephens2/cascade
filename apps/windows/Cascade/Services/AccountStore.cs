using System;
using System.IO;
using System.Text.Json;

namespace Cascade.Services;

/// <summary>
/// Stores the account blob the core hands over in <c>PersistAccount</c>, as
/// <c>account.json</c> under %LOCALAPPDATA%\Cascade. The core owns the blob's
/// shape; this store keeps it verbatim. The device id is owned by the core too
/// (it lives in the listening blob, so rotating it and zeroing the slot are one
/// write); this store only reads the id it used to persist, so an existing
/// install keeps its server slot. (This app ships unpackaged, so we use plain
/// files rather than ApplicationData; Credential Locker / DPAPI is the
/// on-device hardening follow-up.)
/// </summary>
public sealed class AccountStore
{
    private readonly string _accountPath;
    private readonly string _devicePath;

    public AccountStore()
    {
        var dir = Path.Combine(
            Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData),
            "Cascade");
        Directory.CreateDirectory(dir);
        _accountPath = Path.Combine(dir, "account.json");
        _devicePath = Path.Combine(dir, "device.txt");
    }

    /// <summary>
    /// The stored account blob for <c>Restore</c>'s <c>accountJson</c>, or ""
    /// if there is none. Builds before the core held the account stored a
    /// PascalCase <c>{ SessionToken, Email }</c> record; that is converted to
    /// the version-less <c>{ sessionToken, email }</c> shape the core accepts,
    /// so an existing sign-in survives the upgrade. The file is rewritten on
    /// the core's next <c>PersistAccount</c>.
    /// </summary>
    public string ReadAccountJson()
    {
        try
        {
            if (!File.Exists(_accountPath)) return "";
            var json = File.ReadAllText(_accountPath);
            return FromLegacyRecord(json) ?? json;
        }
        catch
        {
            return "";
        }
    }

    /// <summary>Store the core's account blob verbatim; "" deletes it.</summary>
    public void WriteAccountJson(string json)
    {
        try
        {
            if (json.Length > 0) File.WriteAllText(_accountPath, json);
            else if (File.Exists(_accountPath)) File.Delete(_accountPath);
        }
        catch { /* best-effort */ }
    }

    /// The version-less blob for a PascalCase record from an older build, or
    /// null if <paramref name="json"/> is not one (property names match
    /// case-sensitively, so the core's camelCase blob passes through).
    private static string? FromLegacyRecord(string json)
    {
        using var doc = JsonDocument.Parse(json);
        var root = doc.RootElement;
        if (root.ValueKind != JsonValueKind.Object ||
            !root.TryGetProperty("SessionToken", out var token) ||
            !root.TryGetProperty("Email", out var email))
            return null;
        return JsonSerializer.Serialize(new
        {
            sessionToken = token.GetString(),
            email = email.GetString(),
        });
    }

    /// <summary>
    /// The device id older builds persisted here, if any — handed to the core
    /// once as its fallback id so the existing server slot carries over. Never
    /// written; the core owns the id from here on.
    /// </summary>
    public string? ReadLegacyDeviceId()
    {
        try
        {
            if (!File.Exists(_devicePath)) return null;
            var existing = File.ReadAllText(_devicePath).Trim();
            return string.IsNullOrEmpty(existing) ? null : existing;
        }
        catch
        {
            return null;
        }
    }
}
