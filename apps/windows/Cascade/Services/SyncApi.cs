using System;
using System.Net.Http;
using System.Net.Http.Json;
using System.Threading.Tasks;

namespace Cascade.Services;

/// <summary>Base URL of cascade-sync-server. Empty disables the sync feature.</summary>
public static class SyncConfig
{
    public const string ApiBase = "https://sync.cascade.stephens.page";
    public static bool Available => !string.IsNullOrEmpty(ApiBase);
}

public sealed class SyncHttpException : Exception
{
    public int Status { get; }
    public SyncHttpException(int status, string message) : base(message) => Status = status;
}

public sealed record VerifyResponse(string SessionToken, string Email);
public sealed record ListeningResponse(long ServerTotalMs, long SyncedThroughMs);

/// <summary>Typed client for the sync endpoints (System.Net.Http + System.Text.Json).</summary>
public sealed class SyncApi
{
    private static readonly HttpClient Http = new() { Timeout = TimeSpan.FromSeconds(15) };

    public Task RequestLinkAsync(string email) =>
        // The "windows" hint makes the emailed link carry &app=windows, so the
        // web /auth page hands the token to this app via cascade:// rather than
        // consuming it in the browser.
        PostAsync("/auth/request", new { email, platform = "windows" }, null);

    public async Task<VerifyResponse> VerifyAsync(string token)
    {
        using var resp = await SendAsync(HttpMethod.Post, "/auth/verify", new { token }, null);
        return (await resp.Content.ReadFromJsonAsync<VerifyResponse>())!;
    }

    public Task LogoutAsync(string sessionToken) =>
        PostAsync("/auth/logout", null, sessionToken);

    public async Task<ListeningResponse> PutListeningAsync(string sessionToken, string deviceId, long deviceTotalMs)
    {
        using var resp = await SendAsync(HttpMethod.Put, "/listening",
            new { deviceId, deviceTotalMs }, sessionToken);
        return (await resp.Content.ReadFromJsonAsync<ListeningResponse>())!;
    }

    public Task DeleteListeningAsync(string sessionToken) =>
        SendVoidAsync(HttpMethod.Delete, "/listening", sessionToken);

    public Task DeleteAccountAsync(string sessionToken) =>
        SendVoidAsync(HttpMethod.Delete, "/account", sessionToken);

    private async Task PostAsync(string path, object? body, string? token)
    {
        using var _ = await SendAsync(HttpMethod.Post, path, body, token);
    }

    private async Task SendVoidAsync(HttpMethod method, string path, string? token)
    {
        using var _ = await SendAsync(method, path, null, token);
    }

    private static async Task<HttpResponseMessage> SendAsync(
        HttpMethod method, string path, object? body, string? token)
    {
        using var req = new HttpRequestMessage(method, $"{SyncConfig.ApiBase}{path}");
        if (token is not null)
            req.Headers.Authorization = new System.Net.Http.Headers.AuthenticationHeaderValue("Bearer", token);
        if (body is not null)
            req.Content = JsonContent.Create(body);
        var resp = await Http.SendAsync(req);
        if (!resp.IsSuccessStatusCode)
        {
            var text = await resp.Content.ReadAsStringAsync();
            resp.Dispose();
            throw new SyncHttpException((int)resp.StatusCode,
                string.IsNullOrEmpty(text) ? $"request failed ({(int)resp.StatusCode})" : text);
        }
        return resp;
    }
}
