using System;
using System.IO;
using System.Text.Json;

namespace Cascade.Services;

public sealed record Account(string SessionToken, string Email);

/// <summary>
/// Persists the optional sync account (session token + email) and a stable
/// per-device id as JSON files under %LOCALAPPDATA%\Cascade. The device id is
/// rotated on "delete data" so a stale offline write can't resurrect a deleted
/// G-Counter slot. (This app ships unpackaged, so we use plain files rather
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

    public string DeviceId()
    {
        try
        {
            if (File.Exists(_devicePath))
            {
                var existing = File.ReadAllText(_devicePath).Trim();
                if (!string.IsNullOrEmpty(existing)) return existing;
            }
        }
        catch { /* fall through to create */ }
        return RotateDeviceId();
    }

    public string RotateDeviceId()
    {
        var id = Guid.NewGuid().ToString();
        try { File.WriteAllText(_devicePath, id); }
        catch { /* best-effort */ }
        return id;
    }
}
