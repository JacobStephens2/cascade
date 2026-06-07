using System;
using Microsoft.Windows.AppLifecycle;

namespace Cascade.Services;

/// <summary>
/// Registers the <c>cascade://</c> URI scheme so a magic-link handoff
/// (<c>cascade://auth?token=…</c>) launches or focuses this app.
///
/// We use WinAppSDK's <see cref="ActivationRegistrationManager"/> rather than a
/// hand-written <c>HKCU\Software\Classes</c> command: the manual key launches
/// the exe with the URI as a raw argument, which WinAppSDK surfaces as a plain
/// <c>Launch</c> activation (no parsed <c>Uri</c>). Registering through the
/// manager instead makes activations arrive as
/// <see cref="Microsoft.Windows.AppLifecycle.ExtendedActivationKind.Protocol"/>
/// with the <c>Uri</c> populated — and it works for unpackaged apps, writing a
/// per-user registration (no admin, WDAC-safe).
/// </summary>
public static class ProtocolRegistration
{
    public const string Scheme = "cascade";

    /// <summary>
    /// Ensure <c>cascade://</c> resolves to this app. Idempotent and best-effort
    /// — any failure is swallowed so it never blocks startup (deep-link sign-in
    /// is a convenience, not a requirement).
    /// </summary>
    public static void EnsureRegistered()
    {
        try
        {
            ActivationRegistrationManager.RegisterForProtocolActivation(
                Scheme,
                logo: string.Empty,
                displayName: "Cascade",
                exePath: string.Empty); // empty => the current executable
        }
        catch
        {
            // Best-effort; deep links just won't work until a successful run.
        }
    }
}
