using System;
using System.ComponentModel;
using System.IO;
using Cascade.Services;
using Cascade.ViewModels;
using Microsoft.UI.Dispatching;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Controls.Primitives;
using Microsoft.UI.Xaml.Media;
using Microsoft.UI.Xaml.Media.Animation;
using Windows.UI;

namespace Cascade;

public sealed partial class MainWindow : Window
{
    public AppViewModel ViewModel { get; }

    // Inner-ring stroke states (web .play-button__ring: .35 alpha idle, full
    // accent while playing).
    private static readonly SolidColorBrush RingIdleBrush =
        new(Color.FromArgb(0x59, 0x6E, 0xC9, 0xE2));
    private static readonly SolidColorBrush RingPlayingBrush =
        new(Color.FromArgb(0xFF, 0x6E, 0xC9, 0xE2));

    public MainWindow()
    {
        InitializeComponent();
        SetWindowIcon();
        SetupTitleBar();
        ViewModel = new AppViewModel(DispatcherQueue.GetForCurrentThread());
        ViewModel.PropertyChanged += OnViewModelPropertyChanged;
        RootGrid.Loaded += OnRootLoaded;
        Closed += (_, _) =>
        {
            ViewModel.PropertyChanged -= OnViewModelPropertyChanged;
            ViewModel.Dispose();
        };
    }

    /// <summary>
    /// Draw our own content under a transparent caption so the dark gradient and
    /// backdrop run edge-to-edge; tint the system caption buttons to match.
    /// </summary>
    private void SetupTitleBar()
    {
        ExtendsContentIntoTitleBar = true;
        SetTitleBar(AppTitleBar);

        var titleBar = GetAppWindow().TitleBar;
        var ink = Color.FromArgb(0xFF, 0xE6, 0xF1, 0xF6);
        var hover = Color.FromArgb(0x12, 0xFF, 0xFF, 0xFF);
        var pressed = Color.FromArgb(0x1F, 0xFF, 0xFF, 0xFF);
        titleBar.ButtonBackgroundColor = Microsoft.UI.Colors.Transparent;
        titleBar.ButtonInactiveBackgroundColor = Microsoft.UI.Colors.Transparent;
        titleBar.ButtonForegroundColor = ink;
        titleBar.ButtonInactiveForegroundColor = Color.FromArgb(0xFF, 0x4F, 0x6B, 0x78);
        titleBar.ButtonHoverBackgroundColor = hover;
        titleBar.ButtonHoverForegroundColor = ink;
        titleBar.ButtonPressedBackgroundColor = pressed;
        titleBar.ButtonPressedForegroundColor = ink;
    }

    private Microsoft.UI.Windowing.AppWindow GetAppWindow()
    {
        var hwnd = WinRT.Interop.WindowNative.GetWindowHandle(this);
        var windowId = Microsoft.UI.Win32Interop.GetWindowIdFromWindow(hwnd);
        return Microsoft.UI.Windowing.AppWindow.GetFromWindowId(windowId);
    }

    /// <summary>
    /// Kick off the ambient backdrop drift + outer-ring rotation once the visual
    /// tree is live, then paint the initial playing/idle visual state.
    /// </summary>
    private void OnRootLoaded(object sender, RoutedEventArgs e)
    {
        BeginLoop("DriftA");
        BeginLoop("DriftB");
        BeginLoop("DriftC");
        BeginLoop("RingSpin");
        UpdateAmbientVisuals();
    }

    private void BeginLoop(string key)
    {
        if (RootGrid.Resources[key] is Storyboard sb) sb.Begin();
    }

    private void OnViewModelPropertyChanged(object? sender, PropertyChangedEventArgs e)
    {
        // Snapshot is swapped wholesale on every core update; repaint the ambient
        // accents (glow, ring, backdrop, progress tint) from its new state.
        if (e.PropertyName == nameof(AppViewModel.Snapshot)) UpdateAmbientVisuals();
    }

    /// <summary>
    /// Mirror the web shell's playing/idle treatment: brighten the play-button
    /// glow + ring and the backdrop while playing, and shift the deep backdrop
    /// tint from cyan toward green as the active timer progresses
    /// (web: <c>hsl(200 - progress*40, 70%, …)</c>).
    /// </summary>
    private void UpdateAmbientVisuals()
    {
        if (PlayGlow is null) return; // visual tree not ready yet

        var snap = ViewModel.Snapshot;
        var playing = snap.IsPlaying;

        PlayGlow.Opacity = playing ? 0.55 : 0.0;
        PlayInnerRing.Stroke = playing ? RingPlayingBrush : RingIdleBrush;
        BackdropRoot.Opacity = playing ? 1.0 : 0.55;

        var progress = Math.Clamp(snap.Timer.Progress, 0f, 1f);
        var hue = 200.0 - progress * 40.0;
        var lightness = playing ? 0.22 : 0.14;
        var tint = HslToColor(hue, 0.70, lightness);
        if (BackdropLayerA.Fill is RadialGradientBrush brush && brush.GradientStops.Count >= 2)
        {
            brush.GradientStops[0].Color = tint;
            brush.GradientStops[1].Color = Color.FromArgb(0x00, tint.R, tint.G, tint.B);
        }
    }

    /// <summary>HSL (h in degrees, s/l in 0..1) to an opaque RGB colour.</summary>
    private static Color HslToColor(double h, double s, double l)
    {
        h = ((h % 360) + 360) % 360 / 360.0;
        double r, g, b;
        if (s == 0)
        {
            r = g = b = l;
        }
        else
        {
            var q = l < 0.5 ? l * (1 + s) : l + s - l * s;
            var p = 2 * l - q;
            r = HueToChannel(p, q, h + 1.0 / 3);
            g = HueToChannel(p, q, h);
            b = HueToChannel(p, q, h - 1.0 / 3);
        }
        return Color.FromArgb(0xFF, (byte)Math.Round(r * 255), (byte)Math.Round(g * 255), (byte)Math.Round(b * 255));
    }

    private static double HueToChannel(double p, double q, double t)
    {
        if (t < 0) t += 1;
        if (t > 1) t -= 1;
        if (t < 1.0 / 6) return p + (q - p) * 6 * t;
        if (t < 1.0 / 2) return q;
        if (t < 2.0 / 3) return p + (q - p) * (2.0 / 3 - t) * 6;
        return p;
    }

    /// <summary>
    /// Set the title-bar / taskbar icon for this unpackaged window. The .exe
    /// already embeds the icon via &lt;ApplicationIcon&gt; (Explorer/Alt-Tab),
    /// but the live window needs it set explicitly through AppWindow.
    /// </summary>
    private void SetWindowIcon()
    {
        var iconPath = Path.Combine(AppContext.BaseDirectory, "Assets", "app.ico");
        if (!File.Exists(iconPath)) return;
        var hwnd = WinRT.Interop.WindowNative.GetWindowHandle(this);
        var windowId = Microsoft.UI.Win32Interop.GetWindowIdFromWindow(hwnd);
        Microsoft.UI.Windowing.AppWindow.GetFromWindowId(windowId).SetIcon(iconPath);
    }

    [System.Runtime.InteropServices.DllImport("user32.dll")]
    private static extern bool SetForegroundWindow(IntPtr hWnd);

    [System.Runtime.InteropServices.DllImport("user32.dll")]
    private static extern bool ShowWindow(IntPtr hWnd, int nCmdShow);

    private const int SW_RESTORE = 9;

    /// <summary>
    /// Restore and foreground the window after a <c>cascade://</c> deep-link
    /// activation, so a sign-in handoff surfaces the app even if it was
    /// minimized or behind other windows.
    /// </summary>
    public void BringToFront()
    {
        var hwnd = WinRT.Interop.WindowNative.GetWindowHandle(this);
        ShowWindow(hwnd, SW_RESTORE);
        SetForegroundWindow(hwnd);
        Activate();
    }

    /// <summary>
    /// Slider raises ValueChanged on every motion frame; ignore the initial
    /// load callback (where IntermediateValue == OldValue) so we don't
    /// dispatch a `setVolume` on the snapshot's initial bind.
    /// </summary>
    private void OnVolumeChanged(object sender, RangeBaseValueChangedEventArgs e)
    {
        if (sender is Slider slider && slider.IsLoaded)
        {
            ViewModel.SetVolumeCommand.Execute(e.NewValue);
        }
    }

    private void OnStartCustom(object sender, RoutedEventArgs e)
    {
        // NumberBox.Value is NaN when the field is empty / invalid.
        var raw = CustomMinutes.Value;
        if (double.IsNaN(raw)) return;
        var minutes = (int)Math.Round(raw);
        if (minutes < 1 || minutes > 1440) return;
        ViewModel.StartCustom(minutes, sleep: CustomSleepRadio.IsChecked == true);
    }
}
