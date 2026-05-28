using System;
using Microsoft.UI.Dispatching;

namespace Cascade.Services;

/// <summary>
/// Drives the wall-clock <c>tick</c> command into the Rust core while a
/// timer is running. Matches the web app's 250ms cadence so the countdown
/// reads smoothly.
///
/// Owns nothing platform-specific past <see cref="DispatcherQueueTimer"/>.
/// </summary>
public sealed class TickScheduler
{
    private readonly DispatcherQueue _dispatcher;
    private DispatcherQueueTimer? _timer;
    private DateTimeOffset _lastTick;

    public TickScheduler(DispatcherQueue dispatcher)
    {
        _dispatcher = dispatcher;
    }

    public void Start(Action<ulong> onTickElapsedMs)
    {
        if (_timer is not null) return;

        _lastTick = DateTimeOffset.UtcNow;
        _timer = _dispatcher.CreateTimer();
        _timer.Interval = TimeSpan.FromMilliseconds(250);
        _timer.IsRepeating = true;
        _timer.Tick += (_, _) =>
        {
            var now = DateTimeOffset.UtcNow;
            var elapsed = (ulong)(now - _lastTick).TotalMilliseconds;
            _lastTick = now;
            onTickElapsedMs(elapsed);
        };
        _timer.Start();
    }

    public void Stop()
    {
        _timer?.Stop();
        _timer = null;
    }
}
