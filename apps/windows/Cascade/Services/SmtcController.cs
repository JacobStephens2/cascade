using System;
using Windows.Media;
using Windows.Media.Playback;

namespace Cascade.Services;

/// <summary>
/// Wires the Windows System Media Transport Controls (volume flyout, media
/// keys, headphone buttons) up to our Cascade dispatch loop.
///
/// The MediaPlayer already auto-publishes basic state; here we override
/// metadata to "Cascade — Waterfall focus sound" and route button presses
/// back to the ViewModel.
/// </summary>
public sealed class SmtcController : IDisposable
{
    private readonly SystemMediaTransportControls _smtc;
    private Action<CascadeCommand>? _dispatch;

    public SmtcController(MediaPlayer player)
    {
        _smtc = player.SystemMediaTransportControls;
        _smtc.IsEnabled = true;
        _smtc.IsPlayEnabled = true;
        _smtc.IsPauseEnabled = true;
        _smtc.IsStopEnabled = false;
        _smtc.IsNextEnabled = false;
        _smtc.IsPreviousEnabled = false;
        _smtc.ButtonPressed += OnButtonPressed;
    }

    public void BindDispatch(Action<CascadeCommand> dispatch)
    {
        _dispatch = dispatch;
    }

    public void Update(CascadeSnapshot snapshot)
    {
        var u = _smtc.DisplayUpdater;
        u.Type = MediaPlaybackType.Music;
        u.MusicProperties.Title = snapshot.Title;
        u.MusicProperties.Artist = snapshot.Subtitle;
        u.Update();
        _smtc.PlaybackStatus = snapshot.IsPlaying
            ? MediaPlaybackStatus.Playing
            : MediaPlaybackStatus.Paused;
    }

    private void OnButtonPressed(SystemMediaTransportControls _, SystemMediaTransportControlsButtonPressedEventArgs e)
    {
        var dispatch = _dispatch;
        if (dispatch is null) return;
        switch (e.Button)
        {
            case SystemMediaTransportControlsButton.Play:
                dispatch(new PlayCommand());
                break;
            case SystemMediaTransportControlsButton.Pause:
                dispatch(new PauseCommand());
                break;
        }
    }

    public void Dispose()
    {
        _smtc.ButtonPressed -= OnButtonPressed;
    }
}
