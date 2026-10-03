using System;
using Microsoft.UI.Dispatching;

namespace Cascade.Services;

/// <summary>
/// Drives the wall-clock <c>tick</c> command into the Rust core at the
/// interval the core's snapshot asks for (<c>tickIntervalMs</c>).
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

    /// <summary>Interval the scheduler is currently running at, in ms; 0 when stopped.</summary>
    public int IntervalMs { get; private set; }

    public void Start(Action<ulong> onTickElapsedMs, int intervalMs)
    {
        if (_timer is not null) return;

        IntervalMs = intervalMs;
        _lastTick = DateTimeOffset.UtcNow;
        _timer = _dispatcher.CreateTimer();
        _timer.Interval = TimeSpan.FromMilliseconds(intervalMs);
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
        IntervalMs = 0;
    }
}
