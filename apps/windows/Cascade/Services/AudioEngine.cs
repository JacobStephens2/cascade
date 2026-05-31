using System;
using System.Diagnostics;
using System.IO;
using Windows.Media.Core;
using Windows.Media.Playback;

namespace Cascade.Services;

/// <summary>
/// Wraps Windows.Media.Playback.MediaPlayer + MediaPlaybackList for gapless
/// looping of the bundled waterfall asset.
///
/// MediaPlaybackList with AutoRepeatEnabled = true is the documented Windows
/// pattern for looping a single track without the seam that
/// `IsLoopingEnabled` on a raw MediaPlayer produces.
///
/// SystemMediaTransportControls is wired through the same MediaPlayer
/// instance — see <see cref="SmtcController"/>.
/// </summary>
public sealed class AudioEngine : IDisposable
{
    private readonly MediaPlayer _player = new();
    private MediaPlaybackList? _list;
    private bool _loaded;

    public MediaPlayer Player => _player;

    public void EnsureLoaded()
    {
        if (_loaded) return;

        // This app ships unpackaged (WindowsPackageType=None), so the
        // ms-appx:/// scheme has no package identity to resolve against and
        // MediaPlayer silently fails to open the asset. Load it by its real
        // path next to the executable instead. AppContext.BaseDirectory is the
        // install directory in both packaged and unpackaged builds, so this
        // works everywhere.
        var assetPath = Path.Combine(AppContext.BaseDirectory, "Assets", "waterfall.mp3");
        _player.MediaFailed += OnMediaFailed;
        var item = new MediaPlaybackItem(MediaSource.CreateFromUri(new Uri(assetPath)));
        _list = new MediaPlaybackList { AutoRepeatEnabled = true };
        _list.Items.Add(item);
        _player.Source = _list;
        _player.AutoPlay = false;
        _player.IsLoopingEnabled = false; // MediaPlaybackList handles it
        _loaded = true;
    }

    private static void OnMediaFailed(MediaPlayer sender, MediaPlayerFailedEventArgs args) =>
        Debug.WriteLine($"[Cascade] MediaFailed: {args.Error} - {args.ErrorMessage}");

    public void Start(int volumePercent)
    {
        EnsureLoaded();
        _player.Volume = PerceptualVolume(volumePercent);
        _player.Play();
    }

    public void Pause() => _player.Pause();

    public void SetVolume(int volumePercent) =>
        _player.Volume = PerceptualVolume(volumePercent);

    /// <summary>Square-law curve so 50% feels like ~half loudness. Matches the
    /// web shell.</summary>
    private static double PerceptualVolume(int percent)
    {
        var clamped = Math.Clamp(percent, 0, 100) / 100.0;
        return clamped * clamped;
    }

    public void Dispose()
    {
        _player.Dispose();
    }
}
