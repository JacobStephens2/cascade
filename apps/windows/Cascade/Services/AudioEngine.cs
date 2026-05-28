using System;
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

        // ms-appx scheme resolves to the app package's install directory in
        // packaged builds; in unpackaged builds, MediaSource.CreateFromUri
        // also accepts ms-appx as long as the file is alongside the .exe.
        var uri = new Uri("ms-appx:///Assets/waterfall.mp3");
        var item = new MediaPlaybackItem(MediaSource.CreateFromUri(uri));
        _list = new MediaPlaybackList { AutoRepeatEnabled = true };
        _list.Items.Add(item);
        _player.Source = _list;
        _player.AutoPlay = false;
        _player.IsLoopingEnabled = false; // MediaPlaybackList handles it
        _loaded = true;
    }

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
