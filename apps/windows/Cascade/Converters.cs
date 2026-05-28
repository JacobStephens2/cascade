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
        kind == TimerKind.Sleep || kind == TimerKind.Pomodoro
            ? Visibility.Visible
            : Visibility.Collapsed;

    public static string PercentLabel(int percent) => $"{percent}%";

    public static bool HasText(string? text) => !string.IsNullOrEmpty(text);
}
