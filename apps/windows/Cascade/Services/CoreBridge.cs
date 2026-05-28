using System;
using System.Runtime.InteropServices;
using System.Text;
using System.Threading;

namespace Cascade.Services;

/// <summary>
/// P/Invoke wrapper around the hand-rolled C ABI exported by
/// `cascade-uniffi` (see <c>crates/cascade-uniffi/src/lib.rs</c>).
///
/// The DLL lives next to the .exe (copied by <c>build-rust.ps1</c>). We
/// load it on first use; all subsequent calls are direct P/Invokes.
///
/// All strings cross the boundary as UTF-8 (Rust uses CString). C# Marshal's
/// default Ansi/Unicode encodings would corrupt JSON, so we encode/decode
/// manually.
/// </summary>
public sealed class CoreBridge : IDisposable
{
    private const string DllName = "cascade_uniffi";

    [DllImport(DllName, EntryPoint = "cascade_new", CallingConvention = CallingConvention.Cdecl)]
    private static extern IntPtr CascadeNew();

    [DllImport(DllName, EntryPoint = "cascade_restore_or_new", CallingConvention = CallingConvention.Cdecl)]
    private static extern IntPtr CascadeRestoreOrNew(IntPtr settingsJsonUtf8);

    [DllImport(DllName, EntryPoint = "cascade_snapshot", CallingConvention = CallingConvention.Cdecl)]
    private static extern IntPtr CascadeSnapshot(IntPtr handle);

    [DllImport(DllName, EntryPoint = "cascade_dispatch", CallingConvention = CallingConvention.Cdecl)]
    private static extern IntPtr CascadeDispatch(IntPtr handle, IntPtr commandJsonUtf8);

    [DllImport(DllName, EntryPoint = "cascade_free_string", CallingConvention = CallingConvention.Cdecl)]
    private static extern void CascadeFreeString(IntPtr ptr);

    [DllImport(DllName, EntryPoint = "cascade_free_handle", CallingConvention = CallingConvention.Cdecl)]
    private static extern void CascadeFreeHandle(IntPtr handle);

    private IntPtr _handle;
    private readonly Lock _lock = new();

    public CoreBridge(string? persistedSettingsJson)
    {
        if (string.IsNullOrEmpty(persistedSettingsJson))
        {
            _handle = CascadeNew();
        }
        else
        {
            var utf8 = ToCStringUtf8(persistedSettingsJson);
            try
            {
                _handle = CascadeRestoreOrNew(utf8);
            }
            finally
            {
                Marshal.FreeHGlobal(utf8);
            }
        }
        if (_handle == IntPtr.Zero)
        {
            throw new InvalidOperationException("cascade_new returned a null handle");
        }
    }

    public string Snapshot()
    {
        lock (_lock)
        {
            EnsureNotDisposed();
            var raw = CascadeSnapshot(_handle);
            return ConsumeRustString(raw, "snapshot");
        }
    }

    public string Dispatch(string commandJson)
    {
        lock (_lock)
        {
            EnsureNotDisposed();
            var utf8 = ToCStringUtf8(commandJson);
            try
            {
                var raw = CascadeDispatch(_handle, utf8);
                return ConsumeRustString(raw, "dispatch");
            }
            finally
            {
                Marshal.FreeHGlobal(utf8);
            }
        }
    }

    public void Dispose()
    {
        lock (_lock)
        {
            if (_handle != IntPtr.Zero)
            {
                CascadeFreeHandle(_handle);
                _handle = IntPtr.Zero;
            }
        }
        GC.SuppressFinalize(this);
    }

    ~CoreBridge() => Dispose();

    private void EnsureNotDisposed()
    {
        if (_handle == IntPtr.Zero)
        {
            throw new ObjectDisposedException(nameof(CoreBridge));
        }
    }

    private static string ConsumeRustString(IntPtr raw, string fnName)
    {
        if (raw == IntPtr.Zero)
        {
            throw new InvalidOperationException(
                $"cascade_{fnName} returned NULL — bad JSON in / out of the core");
        }
        try
        {
            return PtrToUtf8String(raw);
        }
        finally
        {
            CascadeFreeString(raw);
        }
    }

    /// <summary>
    /// Allocate an unmanaged null-terminated UTF-8 buffer holding <paramref name="s"/>.
    /// Caller is responsible for <see cref="Marshal.FreeHGlobal"/>.
    /// </summary>
    private static IntPtr ToCStringUtf8(string s)
    {
        var bytes = Encoding.UTF8.GetBytes(s);
        var ptr = Marshal.AllocHGlobal(bytes.Length + 1);
        Marshal.Copy(bytes, 0, ptr, bytes.Length);
        Marshal.WriteByte(ptr, bytes.Length, 0);
        return ptr;
    }

    /// <summary>Read a Rust-owned null-terminated UTF-8 string into a managed C# string.</summary>
    private static string PtrToUtf8String(IntPtr ptr)
    {
        // Walk to the null terminator.
        var length = 0;
        while (Marshal.ReadByte(ptr, length) != 0) length++;
        var bytes = new byte[length];
        Marshal.Copy(ptr, bytes, 0, length);
        return Encoding.UTF8.GetString(bytes);
    }
}
