using System.Runtime.InteropServices;
using System.ComponentModel;
using System.Windows.Forms;

namespace Allwright.WindowsAgent;

internal sealed record LiveInspectClick(long Sequence, int X, int Y, IntPtr Window);

internal static class LiveInspectMouse
{
    private const int WhMouseLowLevel = 14;
    private const int LeftButtonDown = 0x0201;
    private const int LeftButtonUp = 0x0202;
    private const int RightButtonDown = 0x0204;
    private const int RightButtonUp = 0x0205;
    private const int VirtualKeyControl = 0x11;
    private const uint RootAncestor = 2;
    private const long ArmLifetimeMs = 10_000;
    private const long SuppressionBurstMs = 750;
    private static readonly HookCallback Callback = Hook;
    private static LiveInspectClick? latest;
    private static long sequence;
    private static long armedWindow;
    private static long armedAt;
    private static long suppressedWindow;
    private static long suppressUntil;

    public static LiveInspectClick? Latest => Volatile.Read(ref latest);

    public static void Arm(IntPtr window)
    {
        Interlocked.Exchange(ref armedWindow, window.ToInt64());
        Interlocked.Exchange(ref armedAt, Environment.TickCount64);
    }

    public static void Start()
    {
        using var ready = new ManualResetEventSlim();
        Exception? installationError = null;
        var thread = new Thread(() =>
        {
            var hook = SetWindowsHookEx(
                WhMouseLowLevel,
                Callback,
                GetModuleHandle(null),
                0);
            if (hook == IntPtr.Zero)
            {
                installationError = new Win32Exception(
                    Marshal.GetLastWin32Error(),
                    "failed to install the Windows live-inspector mouse hook");
                ready.Set();
                return;
            }
            ready.Set();
            try { Application.Run(); }
            finally { UnhookWindowsHookEx(hook); }
        })
        {
            IsBackground = true,
            Name = "Allwright live inspector mouse monitor"
        };
        thread.SetApartmentState(ApartmentState.STA);
        thread.Start();
        if (!ready.Wait(TimeSpan.FromSeconds(5)))
            throw new TimeoutException("timed out installing the Windows live-inspector mouse hook");
        if (installationError is not null) throw installationError;
    }

    private static IntPtr Hook(int code, IntPtr message, IntPtr data)
    {
        if (code >= 0 && IsButtonMessage(message))
        {
            var mouse = Marshal.PtrToStructure<MouseHookData>(data);
            var window = GetAncestor(WindowFromPoint(mouse.Point), RootAncestor);
            var now = Environment.TickCount64;
            var windowValue = window.ToInt64();
            if (windowValue != 0
                && windowValue == Interlocked.Read(ref suppressedWindow)
                && now <= Interlocked.Read(ref suppressUntil))
                return new IntPtr(1);
            if (IsButtonDown(message)
                && (GetAsyncKeyState(VirtualKeyControl) & 0x8000) != 0
                && now - Interlocked.Read(ref armedAt) < ArmLifetimeMs
                && windowValue != 0
                && windowValue == Interlocked.Read(ref armedWindow))
            {
                Volatile.Write(ref latest, new LiveInspectClick(
                    Interlocked.Increment(ref sequence),
                    mouse.Point.X,
                    mouse.Point.Y,
                    window));
                Interlocked.Exchange(ref suppressedWindow, windowValue);
                Interlocked.Exchange(ref suppressUntil, now + SuppressionBurstMs);
                return new IntPtr(1);
            }
        }
        return CallNextHookEx(IntPtr.Zero, code, message, data);
    }

    private static bool IsButtonMessage(IntPtr message) =>
        message == new IntPtr(LeftButtonDown)
        || message == new IntPtr(LeftButtonUp)
        || message == new IntPtr(RightButtonDown)
        || message == new IntPtr(RightButtonUp);

    private static bool IsButtonDown(IntPtr message) =>
        message == new IntPtr(LeftButtonDown)
        || message == new IntPtr(RightButtonDown);

    private delegate IntPtr HookCallback(int code, IntPtr message, IntPtr data);

    [StructLayout(LayoutKind.Sequential)]
    private struct NativePoint { public int X; public int Y; }

    [StructLayout(LayoutKind.Sequential)]
    private struct MouseHookData
    {
        public NativePoint Point;
        public uint MouseData;
        public uint Flags;
        public uint Time;
        public UIntPtr ExtraInfo;
    }

    [DllImport("user32.dll", SetLastError = true)]
    private static extern IntPtr SetWindowsHookEx(
        int hookType, HookCallback callback, IntPtr module, uint threadId);
    [DllImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool UnhookWindowsHookEx(IntPtr hook);
    [DllImport("user32.dll")]
    private static extern IntPtr CallNextHookEx(
        IntPtr hook, int code, IntPtr message, IntPtr data);
    [DllImport("user32.dll")]
    private static extern short GetAsyncKeyState(int virtualKey);
    [DllImport("user32.dll")]
    private static extern IntPtr WindowFromPoint(NativePoint point);
    [DllImport("user32.dll")]
    private static extern IntPtr GetAncestor(IntPtr window, uint flags);
    [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    private static extern IntPtr GetModuleHandle(string? moduleName);
}
