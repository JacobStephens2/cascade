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
///
/// The core hands us the final gain (0–1, mute and curve already applied);
/// we write it to <c>MediaPlayer.Volume</c> as given.
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

    public void Start(double gain)
    {
        EnsureLoaded();
        _player.Volume = gain;
        _player.Play();
    }

    public void Pause() => _player.Pause();

    public void SetVolume(double gain) => _player.Volume = gain;

    public void Dispose()
    {
        _player.Dispose();
    }
}
