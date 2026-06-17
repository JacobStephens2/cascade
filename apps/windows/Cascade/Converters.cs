using Cascade.Services;
using Microsoft.UI.Xaml;

namespace Cascade;

/// <summary>
/// Static helpers callable from x:Bind. WinUI's compiled binding can invoke
/// static methods, which is cleaner than declaring full
/// IValueConverter instances for one-line transformations.
/// </summary>
public static class Converters
{
    public static Visibility TimerActiveVisibility(TimerKind kind) =>
        kind is TimerKind.Sleep or TimerKind.Pomodoro or TimerKind.Stopwatch
            ? Visibility.Visible
            : Visibility.Collapsed;

    public static string PercentLabel(int percent) => $"{percent}%";

    public static bool HasText(string? text) => !string.IsNullOrEmpty(text);

    /// Segoe MDL2 Assets glyph: Mute (E74F) when muted, Volume (E767) otherwise.
    public static string MuteGlyph(bool muted) => muted ? "" : "";

    public static string VolumeReadout(int percent, bool muted) =>
        muted ? "Muted" : $"{percent}%";

    /// Segoe Fluent / MDL2 glyph for the big circular control: Pause (E769) while
    /// playing, Play (E768) otherwise.
    public static string PrimaryGlyph(bool isPlaying) => isPlaying ? "" : "";

    public static string TrackingLabel(bool enabled) =>
        enabled ? "Tracking on" : "Tracking off";

    public static Visibility VisibleIf(bool b) =>
        b ? Visibility.Visible : Visibility.Collapsed;
}
