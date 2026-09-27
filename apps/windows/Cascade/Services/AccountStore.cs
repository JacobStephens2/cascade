using System;
using System.IO;
using System.Text.Json;

namespace Cascade.Services;

public sealed record Account(string SessionToken, string Email);

/// <summary>
/// Persists the optional sync account (session token + email) as JSON under
/// %LOCALAPPDATA%\Cascade. The device id is owned by the core (it lives in the
/// listening blob, so rotating it and zeroing the slot are one write); this
/// store only reads the id it used to persist, so an existing install keeps
/// its server slot. (This app ships unpackaged, so we use plain files rather
/// than ApplicationData; Credential Locker / DPAPI is the on-device hardening
/// follow-up.)
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

    public Account? ReadAccount()
    {
        try
        {
            return File.Exists(_accountPath)
                ? JsonSerializer.Deserialize<Account>(File.ReadAllText(_accountPath))
                : null;
        }
        catch
        {
            return null;
        }
    }

    public void WriteAccount(Account account)
    {
        try { File.WriteAllText(_accountPath, JsonSerializer.Serialize(account)); }
        catch { /* best-effort */ }
    }

    public void ClearAccount()
    {
        try { if (File.Exists(_accountPath)) File.Delete(_accountPath); }
        catch { /* best-effort */ }
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
