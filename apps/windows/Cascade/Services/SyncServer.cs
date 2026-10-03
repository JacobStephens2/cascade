using System;
using System.Net.Http;
using System.Text;
using System.Threading.Tasks;

namespace Cascade.Services;

/// <summary>Base URL of cascade-sync-server. Empty disables the sync feature.</summary>
public static class SyncConfig
{
    public const string ApiBase = "https://sync.cascade.stephens.page";
    public static bool Available => !string.IsNullOrEmpty(ApiBase);
}

/// <summary>
/// Carries the core's <see cref="ServerRequestEffect"/>s to the sync server.
/// The core owns the request table; this sends exactly what it is given and
/// reports the status and body verbatim, with no per-request knowledge.
/// </summary>
public sealed class SyncServer
{
    private static readonly HttpClient DefaultHttp = new() { Timeout = TimeSpan.FromSeconds(15) };

    private readonly string _baseUrl;
    private readonly HttpClient _http;

    public SyncServer(string baseUrl = SyncConfig.ApiBase, HttpClient? http = null)
    {
        _baseUrl = baseUrl;
        _http = http ?? DefaultHttp;
    }

    /// <summary>
    /// Send one request and answer it with the <see cref="ServerResponseCommand"/>
    /// that settles it. Never throws: no sync server, a network error, a
    /// timeout or a cancellation all settle with status 0.
    /// </summary>
    public async Task<ServerResponseCommand> CarryAsync(ServerRequestEffect request)
    {
        if (string.IsNullOrEmpty(_baseUrl)) return NoResponse(request);
        try
        {
            using var req = new HttpRequestMessage(new HttpMethod(request.Method), _baseUrl + request.Path);
            if (request.BearerToken is not null)
                req.Headers.Authorization =
                    new System.Net.Http.Headers.AuthenticationHeaderValue("Bearer", request.BearerToken);
            if (request.Body is not null)
                req.Content = new StringContent(request.Body, Encoding.UTF8, "application/json");
            using var resp = await _http.SendAsync(req);
            return new ServerResponseCommand(request.Id, (ushort)resp.StatusCode,
                await resp.Content.ReadAsStringAsync());
        }
        catch (Exception)
        {
            return NoResponse(request);
        }
    }

    private static ServerResponseCommand NoResponse(ServerRequestEffect request) =>
        new(request.Id, 0, "");
}
