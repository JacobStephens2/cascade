using System;
using System.Runtime.InteropServices;
using System.Threading;
using System.Threading.Tasks;
using Cascade.Services;
using Microsoft.UI.Dispatching;
using Microsoft.UI.Xaml;
using Microsoft.Windows.AppLifecycle;
using Windows.ApplicationModel.Activation;

namespace Cascade;

/// <summary>
/// Hand-written entry point (the XAML-generated Main is disabled via
/// <c>DISABLE_XAML_GENERATED_MAIN</c>) so we can be single-instance and handle
/// <c>cascade://</c> protocol activation. A magic-link handoff
/// (<c>cascade://auth?token=…</c>) should sign into the <i>already-running</i>
/// app rather than spawning a second window, so a second launch redirects its
/// activation to the first instance and exits.
/// </summary>
public static class Program
{
    [STAThread]
    private static void Main()
    {
        WinRT.ComWrappersSupport.InitializeComWrappers();

        // Make cascade:// resolve to this exe (idempotent, per-user, no admin).
        ProtocolRegistration.EnsureRegistered();

        if (DecideRedirection())
        {
            // We handed our activation to the primary instance — nothing to run.
            return;
        }

        Application.Start(_ =>
        {
            var context = new DispatcherQueueSynchronizationContext(
                DispatcherQueue.GetForCurrentThread());
            SynchronizationContext.SetSynchronizationContext(context);
            // App registers itself as Application.Current in its constructor.
            new App();
        });
    }

    /// <summary>
    /// Register this process as the single-instance key owner. If another
    /// instance already owns it, forward our activation args to it and report
    /// that we redirected (so Main exits without starting a second app).
    /// </summary>
    private static bool DecideRedirection()
    {
        var activationArgs = AppInstance.GetCurrent().GetActivatedEventArgs();
        var keyInstance = AppInstance.FindOrRegisterForKey("cascade-main");

        if (keyInstance.IsCurrent)
        {
            keyInstance.Activated += OnActivated;
            return false;
        }

        RedirectActivationTo(activationArgs, keyInstance);
        return true;
    }

    /// <summary>Primary instance received a redirected activation from a second launch.</summary>
    private static void OnActivated(object? sender, AppActivationArguments args)
    {
        var uri = ExtractProtocolUri(args);
        if (uri is not null)
        {
            App.HandleProtocolActivation(uri);
        }
    }

    /// <summary>Pull the activating URI out of a protocol activation, or null.</summary>
    public static Uri? ExtractProtocolUri(AppActivationArguments args)
    {
        if (args.Kind == ExtendedActivationKind.Protocol &&
            args.Data is IProtocolActivatedEventArgs protocolArgs)
        {
            return protocolArgs.Uri;
        }

        // Defensive fallback: if an activation path delivers the URI as a raw
        // launch argument (Launch kind) instead of a parsed Protocol activation,
        // recover it from the command line so the handoff still works.
        if (args.Kind == ExtendedActivationKind.Launch &&
            args.Data is ILaunchActivatedEventArgs launchArgs)
        {
            return FindCascadeUri(launchArgs.Arguments);
        }
        return null;
    }

    private static Uri? FindCascadeUri(string? arguments)
    {
        if (string.IsNullOrWhiteSpace(arguments)) return null;
        foreach (var part in arguments.Split(' ', StringSplitOptions.RemoveEmptyEntries))
        {
            var token = part.Trim('"');
            if (token.StartsWith($"{ProtocolRegistration.Scheme}://", StringComparison.OrdinalIgnoreCase) &&
                Uri.TryCreate(token, UriKind.Absolute, out var uri))
            {
                return uri;
            }
        }
        return null;
    }

    // --- activation-redirect plumbing ---------------------------------------
    // RedirectActivationToAsync must be awaited, but Main runs on the STA UI
    // thread where a plain .Wait() would deadlock the COM call. Per the WinAppSDK
    // single-instancing sample, run the redirect on a thread-pool thread and pump
    // COM on this one until it signals.
    [DllImport("kernel32.dll")]
    private static extern IntPtr CreateEvent(
        IntPtr lpEventAttributes, bool bManualReset, bool bInitialState, string? lpName);

    [DllImport("kernel32.dll")]
    private static extern bool SetEvent(IntPtr hEvent);

    [DllImport("ole32.dll")]
    private static extern uint CoWaitForMultipleObjects(
        uint dwFlags, uint dwMilliseconds, ulong nHandles, IntPtr[] pHandles, out uint dwpIndex);

    private const uint CWMO_DEFAULT = 0;
    private const uint INFINITE = 0xFFFFFFFF;

    private static void RedirectActivationTo(AppActivationArguments args, AppInstance keyInstance)
    {
        var redirectSemaphore = CreateEvent(IntPtr.Zero, true, false, null);
        _ = Task.Run(() =>
        {
            keyInstance.RedirectActivationToAsync(args).AsTask().Wait();
            SetEvent(redirectSemaphore);
        });
        _ = CoWaitForMultipleObjects(
            CWMO_DEFAULT, INFINITE, 1, new[] { redirectSemaphore }, out _);
    }
}
