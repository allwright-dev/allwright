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
    private static LiveInspectOverlay? overlay;

    public static LiveInspectClick? Latest => Volatile.Read(ref latest);

    public static void Arm(IntPtr window)
    {
        Interlocked.Exchange(ref armedWindow, window.ToInt64());
        Interlocked.Exchange(ref armedAt, Environment.TickCount64);
        var picker = Volatile.Read(ref overlay);
        if (picker is not null && picker.IsHandleCreated)
            picker.BeginInvoke((Action)(() => picker.Arm(window)));
    }

    public static void Start()
    {
        using var ready = new ManualResetEventSlim();
        Exception? installationError = null;
        var thread = new Thread(() =>
        {
            using var picker = new LiveInspectOverlay(Record);
            _ = picker.Handle;
            Volatile.Write(ref overlay, picker);
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
            finally
            {
                Volatile.Write(ref overlay, null);
                UnhookWindowsHookEx(hook);
            }
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
                Record(window, mouse.Point.X, mouse.Point.Y);
                Interlocked.Exchange(ref suppressedWindow, windowValue);
                Interlocked.Exchange(ref suppressUntil, now + SuppressionBurstMs);
                return new IntPtr(1);
            }
        }
        return CallNextHookEx(IntPtr.Zero, code, message, data);
    }

    private static void Record(IntPtr window, int x, int y) =>
        Volatile.Write(ref latest, new LiveInspectClick(
            Interlocked.Increment(ref sequence), x, y, window));

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

internal sealed class LiveInspectOverlay : Form
{
    private const int ExtendedStyle = -20;
    private const long ToolWindow = 0x00000080L;
    private const long NoActivate = 0x08000000L;
    private const int OwnerWindow = -8;
    private const uint WindowAbove = 3;
    private const uint NoActivatePosition = 0x0010;
    private const int MouseActivate = 0x0021;
    private const int NoActivateMouse = 3;
    private readonly Action<IntPtr, int, int> record;
    private readonly System.Windows.Forms.Timer modifierTimer;
    private IntPtr target;
    private long armedAt;

    public LiveInspectOverlay(Action<IntPtr, int, int> record)
    {
        this.record = record;
        FormBorderStyle = FormBorderStyle.None;
        ShowInTaskbar = false;
        StartPosition = FormStartPosition.Manual;
        BackColor = Color.Black;
        Opacity = 0.01;
        modifierTimer = new System.Windows.Forms.Timer { Interval = 15 };
        modifierTimer.Tick += (_, _) => UpdateVisibility();
        modifierTimer.Start();
    }

    protected override bool ShowWithoutActivation => true;
    protected override CreateParams CreateParams
    {
        get
        {
            var parameters = base.CreateParams;
            parameters.ExStyle |= (int)(ToolWindow | NoActivate);
            return parameters;
        }
    }

    public void Arm(IntPtr window)
    {
        target = window;
        armedAt = Environment.TickCount64;
        SetWindowLongPtr(Handle, OwnerWindow, window);
        Reposition();
        UpdateVisibility();
    }

    protected override void OnMouseDown(MouseEventArgs eventArgs)
    {
        if (target != IntPtr.Zero && IsControlPressed())
        {
            var point = PointToScreen(eventArgs.Location);
            record(target, point.X, point.Y);
        }
        base.OnMouseDown(eventArgs);
    }

    protected override void WndProc(ref Message message)
    {
        if (message.Msg == MouseActivate)
        {
            message.Result = new IntPtr(NoActivateMouse);
            return;
        }
        base.WndProc(ref message);
    }

    protected override void Dispose(bool disposing)
    {
        if (disposing) modifierTimer.Dispose();
        base.Dispose(disposing);
    }

    private void UpdateVisibility()
    {
        var shouldShow = target != IntPtr.Zero
            && Environment.TickCount64 - armedAt < 3_000
            && IsWindowVisible(target)
            && !IsIconic(target) && IsControlPressed();
        if (!shouldShow)
        {
            if (Visible) Hide();
            return;
        }
        if (!Visible) Show();
        Reposition();
    }

    private void Reposition()
    {
        if (target == IntPtr.Zero || !GetWindowRect(target, out var bounds)) return;
        var aboveTarget = GetWindow(target, WindowAbove);
        SetWindowPos(
            Handle,
            aboveTarget,
            bounds.Left,
            bounds.Top,
            Math.Max(1, bounds.Right - bounds.Left),
            Math.Max(1, bounds.Bottom - bounds.Top),
            NoActivatePosition);
    }

    private static bool IsControlPressed() =>
        (GetAsyncKeyState(0x11) & 0x8000) != 0;

    [StructLayout(LayoutKind.Sequential)]
    private struct WindowBounds
    {
        public int Left;
        public int Top;
        public int Right;
        public int Bottom;
    }

    [DllImport("user32.dll")]
    private static extern short GetAsyncKeyState(int virtualKey);
    [DllImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool GetWindowRect(IntPtr window, out WindowBounds bounds);
    [DllImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool IsWindowVisible(IntPtr window);
    [DllImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool IsIconic(IntPtr window);
    [DllImport("user32.dll")]
    private static extern IntPtr GetWindow(IntPtr window, uint command);
    [DllImport("user32.dll")]
    private static extern IntPtr SetWindowLongPtr(IntPtr window, int index, IntPtr value);
    [DllImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool SetWindowPos(
        IntPtr window,
        IntPtr insertAfter,
        int x,
        int y,
        int width,
        int height,
        uint flags);
}
