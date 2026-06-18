using Cascade.Services;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Media;

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

    /// Inverse of <see cref="TimerActiveVisibility"/>: the web collapses every
    /// preset section to a lone "Cancel timer" while a timer runs, so the preset
    /// block is visible only when no timer is active.
    public static Visibility TimerInactiveVisibility(TimerKind kind) =>
        kind is TimerKind.Sleep or TimerKind.Pomodoro or TimerKind.Stopwatch
            ? Visibility.Collapsed
            : Visibility.Visible;

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

    /// Web caption under the listening total: "lifetime listening" normally,
    /// "tracking paused" when off (typed uppercase here as WinUI has no
    /// text-transform).
    public static string ListeningCaption(bool enabled) =>
        enabled ? "LIFETIME LISTENING" : "TRACKING PAUSED";

    /// Web dims the total to ink-faint while tracking is off (.listening__value.is-off).
    public static Brush ListeningValueBrush(bool enabled) =>
        (Brush)Application.Current.Resources[enabled ? "InkBrush" : "InkFaintBrush"];

    /// Web signed-in label reads "Syncing · you@example.com".
    public static string SyncingLabel(string email) => $"Syncing · {email}";

    public static Visibility VisibleIf(bool b) =>
        b ? Visibility.Visible : Visibility.Collapsed;
}
