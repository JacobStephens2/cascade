using System.Collections.Generic;
using System.Text.Json;
using System.Text.Json.Serialization;

namespace Cascade.Services;

/// <summary>
/// Mirrors the serde-tagged enums in `cascade-core`. The Rust crate is the
/// source of truth — these classes describe the JSON wire shape so we can
/// talk to the bridge in a typed way.
/// </summary>
public static class CascadeJson
{
    public static readonly JsonSerializerOptions Options = new()
    {
        PropertyNamingPolicy = JsonNamingPolicy.CamelCase,
        WriteIndented = false,
    };
}

// ---------- Commands ----------

[JsonPolymorphic(TypeDiscriminatorPropertyName = "type")]
[JsonDerivedType(typeof(PlayCommand), "play")]
[JsonDerivedType(typeof(PauseCommand), "pause")]
[JsonDerivedType(typeof(TogglePlaybackCommand), "togglePlayback")]
[JsonDerivedType(typeof(SetVolumeCommand), "setVolume")]
[JsonDerivedType(typeof(ToggleMuteCommand), "toggleMute")]
[JsonDerivedType(typeof(StartSleepTimerCommand), "startSleepTimer")]
[JsonDerivedType(typeof(StartPomodoroCommand), "startPomodoro")]
[JsonDerivedType(typeof(StartStopwatchCommand), "startStopwatch")]
[JsonDerivedType(typeof(CancelTimerCommand), "cancelTimer")]
[JsonDerivedType(typeof(TickCommand), "tick")]
[JsonDerivedType(typeof(PlatformPlaybackStartedCommand), "platformPlaybackStarted")]
[JsonDerivedType(typeof(PlatformPlaybackPausedCommand), "platformPlaybackPaused")]
[JsonDerivedType(typeof(PlatformPlaybackErrorCommand), "platformPlaybackError")]
[JsonDerivedType(typeof(SetListeningTrackingCommand), "setListeningTracking")]
[JsonDerivedType(typeof(RestoreListeningCommand), "restoreListening")]
[JsonDerivedType(typeof(ApplySyncedTotalCommand), "applySyncedTotal")]
[JsonDerivedType(typeof(ResetListeningDataCommand), "resetListeningData")]
public abstract record CascadeCommand;

public sealed record PlayCommand : CascadeCommand;
public sealed record PauseCommand : CascadeCommand;
public sealed record TogglePlaybackCommand : CascadeCommand;
public sealed record SetVolumeCommand(int Percent) : CascadeCommand;
public sealed record ToggleMuteCommand : CascadeCommand;
public sealed record StartSleepTimerCommand(int Minutes) : CascadeCommand;
public sealed record StartPomodoroCommand(int Minutes) : CascadeCommand;
public sealed record StartStopwatchCommand : CascadeCommand;
public sealed record CancelTimerCommand : CascadeCommand;
public sealed record TickCommand(ulong ElapsedMs) : CascadeCommand;
public sealed record PlatformPlaybackStartedCommand : CascadeCommand;
public sealed record PlatformPlaybackPausedCommand : CascadeCommand;
public sealed record PlatformPlaybackErrorCommand(string Message) : CascadeCommand;
public sealed record SetListeningTrackingCommand(bool Enabled) : CascadeCommand;
public sealed record RestoreListeningCommand(string Json) : CascadeCommand;
public sealed record ApplySyncedTotalCommand(ulong SyncedThroughMs, ulong ServerTotalMs) : CascadeCommand;
public sealed record ResetListeningDataCommand : CascadeCommand;

// ---------- Effects ----------

[JsonPolymorphic(TypeDiscriminatorPropertyName = "type")]
[JsonDerivedType(typeof(StartPlaybackEffect), "startPlayback")]
[JsonDerivedType(typeof(PausePlaybackEffect), "pausePlayback")]
[JsonDerivedType(typeof(SetPlatformVolumeEffect), "setPlatformVolume")]
[JsonDerivedType(typeof(PersistSettingsEffect), "persistSettings")]
[JsonDerivedType(typeof(PersistListeningEffect), "persistListening")]
public abstract record CascadeEffect;

public sealed record StartPlaybackEffect(int VolumePercent) : CascadeEffect;
public sealed record PausePlaybackEffect : CascadeEffect;
public sealed record SetPlatformVolumeEffect(int VolumePercent) : CascadeEffect;
public sealed record PersistSettingsEffect(string Json) : CascadeEffect;
public sealed record PersistListeningEffect(string Json) : CascadeEffect;

// ---------- Snapshot ----------

[JsonConverter(typeof(TimerKindConverter))]
public enum TimerKind { Off, Sleep, Pomodoro, Stopwatch, JustCompleted }

internal sealed class TimerKindConverter : System.Text.Json.Serialization.JsonConverter<TimerKind>
{
    public override TimerKind Read(ref System.Text.Json.Utf8JsonReader reader,
        System.Type typeToConvert, System.Text.Json.JsonSerializerOptions options) =>
        reader.GetString() switch
        {
            "off" => TimerKind.Off,
            "sleep" => TimerKind.Sleep,
            "pomodoro" => TimerKind.Pomodoro,
            "stopwatch" => TimerKind.Stopwatch,
            "justCompleted" => TimerKind.JustCompleted,
            var other => throw new System.Text.Json.JsonException($"unknown TimerKind '{other}'"),
        };

    public override void Write(System.Text.Json.Utf8JsonWriter writer, TimerKind value,
        System.Text.Json.JsonSerializerOptions options) =>
        writer.WriteStringValue(value switch
        {
            TimerKind.Off => "off",
            TimerKind.Sleep => "sleep",
            TimerKind.Pomodoro => "pomodoro",
            TimerKind.Stopwatch => "stopwatch",
            TimerKind.JustCompleted => "justCompleted",
            _ => "off",
        });
}

public sealed record TimerSnapshot(
    TimerKind Kind,
    string RemainingLabel,
    ulong RemainingMs,
    ulong TotalMs,
    float Progress
);

public sealed record ListeningSnapshot(
    bool TrackingEnabled,
    ulong DeviceTotalMs,
    ulong DisplayedTotalMs,
    ulong UnsyncedMs,
    string TotalLabel
);

public sealed record CascadeSnapshot(
    string Title,
    string Subtitle,
    bool IsPlaying,
    int VolumePercent,
    bool IsMuted,
    string PrimaryButtonLabel,
    TimerSnapshot Timer,
    string? ErrorMessage,
    ListeningSnapshot Listening
);

public sealed record CascadeUpdate(
    CascadeSnapshot Snapshot,
    List<CascadeEffect> Effects
);
