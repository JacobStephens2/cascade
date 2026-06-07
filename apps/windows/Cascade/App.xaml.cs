using System;
using Microsoft.UI.Xaml;
using Microsoft.Windows.AppLifecycle;

namespace Cascade;

public partial class App : Application
{
    public static MainWindow? Window { get; private set; }

    public App()
    {
        InitializeComponent();
    }

    protected override void OnLaunched(LaunchActivatedEventArgs args)
    {
        Window = new MainWindow();
        Window.Activate();

        // If this launch itself came from a cascade://auth?token=… link, finish
        // the sign-in now that the window (and its view-model) exist.
        var uri = Program.ExtractProtocolUri(AppInstance.GetCurrent().GetActivatedEventArgs());
        if (uri is not null)
        {
            HandleProtocolActivation(uri);
        }
    }

    /// <summary>
    /// Route a <c>cascade://auth?token=…</c> activation — whether it launched
    /// the app or was redirected here from a second instance — to the running
    /// window's view-model, on the UI thread. The view-model's existing
    /// <c>SignInWithLinkAsync</c> extracts the token and verifies it.
    /// </summary>
    public static void HandleProtocolActivation(Uri uri)
    {
        var window = Window;
        if (window is null || !IsAuthUri(uri)) return;

        window.DispatcherQueue.TryEnqueue(() =>
        {
            window.BringToFront();
            _ = window.ViewModel.SignInWithLinkAsync(uri.OriginalString);
        });
    }

    /// <summary>Accept only auth handoffs (host "auth" or a token query).</summary>
    private static bool IsAuthUri(Uri uri) =>
        string.Equals(uri.Host, "auth", StringComparison.OrdinalIgnoreCase) ||
        uri.OriginalString.Contains("token=", StringComparison.OrdinalIgnoreCase);
}
