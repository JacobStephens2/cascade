using System;
using System.Runtime.InteropServices;

namespace Cascade.Services;

/// <summary>
/// Prevents Windows from sleeping during long focus / sleep-timer sessions.
///
/// <c>ES_CONTINUOUS | ES_SYSTEM_REQUIRED</c> keeps the system awake without
/// preventing the display from sleeping — the exact policy for an 8-hour
/// session. Acquired on <c>StartPlayback</c>, released on <c>PausePlayback</c>.
/// Mirrors the macOS <c>IOPMAssertion</c> path.
/// </summary>
public sealed class PowerController
{
    [Flags]
    private enum ES : uint
    {
        Continuous = 0x80000000,
        SystemRequired = 0x00000001,
        AwaymodeRequired = 0x00000040,
    }

    [DllImport("kernel32.dll", SetLastError = true)]
    private static extern uint SetThreadExecutionState(ES esFlags);

    private bool _held;

    public void Acquire()
    {
        if (_held) return;
        SetThreadExecutionState(ES.Continuous | ES.SystemRequired);
        _held = true;
    }

    public void Release()
    {
        if (!_held) return;
        SetThreadExecutionState(ES.Continuous); // clears all flags except CONTINUOUS
        _held = false;
    }
}
