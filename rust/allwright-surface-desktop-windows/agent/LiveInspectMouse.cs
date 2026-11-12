using System.Runtime.InteropServices;
using System.Windows.Forms;

namespace Allwright.WindowsAgent;

internal sealed record LiveInspectClick(long Sequence, int X, int Y, IntPtr Window);

internal static class LiveInspectMouse
{
    private const int WhMouseLowLevel = 14;
    private const int LeftButtonDown = 0x0201;
    private const int LeftButtonUp = 0x0202;
    private const int VirtualKeyControl = 0x11;
    private const uint RootAncestor = 2;
    private static readonly HookCallback Callback = Hook;
    private static LiveInspectClick? latest;
    private static long sequence;
    private static long armedWindow;
    private static long armedAt;
    private static int suppressButtonUp;

    public static LiveInspectClick? Latest => Volatile.Read(ref latest);

    public static void Arm(IntPtr window)
    {
        Interlocked.Exchange(ref armedWindow, window.ToInt64());
        Interlocked.Exchange(ref armedAt, Environment.TickCount64);
    }

    public static void Start()
    {
        var thread = new Thread(() =>
        {
            var hook = SetWindowsHookEx(WhMouseLowLevel, Callback, IntPtr.Zero, 0);
            if (hook == IntPtr.Zero) return;
            try { Application.Run(); }
            finally { UnhookWindowsHookEx(hook); }
        })
        {
            IsBackground = true,
            Name = "Allwright live inspector mouse monitor"
        };
        thread.SetApartmentState(ApartmentState.STA);
        thread.Start();
    }

    private static IntPtr Hook(int code, IntPtr message, IntPtr data)
    {
        if (code >= 0 && message == new IntPtr(LeftButtonUp)
            && Interlocked.Exchange(ref suppressButtonUp, 0) != 0)
            return new IntPtr(1);
        if (code >= 0 && message == new IntPtr(LeftButtonDown)
            && (GetAsyncKeyState(VirtualKeyControl) & 0x8000) != 0
            && Environment.TickCount64 - Interlocked.Read(ref armedAt) < 1_000)
        {
            var mouse = Marshal.PtrToStructure<MouseHookData>(data);
            var window = GetAncestor(WindowFromPoint(mouse.Point), RootAncestor);
            if (window != IntPtr.Zero
                && window.ToInt64() == Interlocked.Read(ref armedWindow))
            {
                Volatile.Write(ref latest, new LiveInspectClick(
                    Interlocked.Increment(ref sequence),
                    mouse.Point.X,
                    mouse.Point.Y,
                    window));
                Interlocked.Exchange(ref suppressButtonUp, 1);
                return new IntPtr(1);
            }
        }
        return CallNextHookEx(IntPtr.Zero, code, message, data);
    }

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
}
