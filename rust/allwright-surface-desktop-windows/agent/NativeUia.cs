using System.Runtime.InteropServices;
using Com = Interop.UIAutomationClient;

namespace Allwright.WindowsAgent;

internal sealed class UiaProviderTimeoutException : TimeoutException
{
    public UiaProviderTimeoutException(string operation, bool waitingForPreviousCall = false)
        : base(waitingForPreviousCall
            ? $"Windows UI Automation provider was still completing an earlier call before {operation}"
            : $"Windows UI Automation provider timed out during {operation}") { }
}

internal static class UiaCall
{
    private const int DefaultTimeoutMs = 2_500;
    private static readonly SemaphoreSlim ProviderGate = new(1, 1);

    public static T Run<T>(string operation, Func<T> work, int timeoutMs = DefaultTimeoutMs)
    {
        if (!ProviderGate.Wait(timeoutMs))
            throw new UiaProviderTimeoutException(operation, waitingForPreviousCall: true);
        var task = Task.Run(() =>
        {
            try { return work(); }
            finally { ProviderGate.Release(); }
        });
        try
        {
            if (!task.Wait(timeoutMs))
            {
                _ = task.ContinueWith(faulted => _ = faulted.Exception,
                    TaskContinuationOptions.OnlyOnFaulted | TaskContinuationOptions.ExecuteSynchronously);
                throw new UiaProviderTimeoutException(operation);
            }
            return task.Result;
        }
        catch (AggregateException error) { throw error.InnerException ?? error; }
    }

    public static T Or<T>(string operation, Func<T> work, T fallback)
    {
        try { return Run(operation, work); }
        catch { return fallback; }
    }
}

internal sealed record HoverElementSnapshot(
    string Role,
    string Name,
    string AutomationId,
    string ClassName,
    bool Enabled,
    bool Offscreen,
    System.Drawing.RectangleF Bounds);

internal sealed class NativeAutomation
{
    private readonly Com.IUIAutomation client;
    public NativeAutomation()
    {
        // Program.Main is STA for native input compatibility. Create the UIA
        // client in the process MTA used by bounded provider work so direct,
        // provider-independent condition construction cannot initialize an
        // STA-bound COM wrapper and deadlock worker calls.
        client = Task.Run(() => CreateClient()).GetAwaiter().GetResult();
        ConditionFactory = new(this);
    }
    internal Com.IUIAutomation Client => client;
    public NativeConditionFactory ConditionFactory { get; }
    public AutomationElement GetDesktop() =>
        new(this, UiaCall.Run("desktop root lookup", () => Client.GetRootElement()));
    public AutomationElement FromHandle(IntPtr handle) => AutomationElement.FromWindow(this, handle);
    public AutomationElement FromPoint(double x, double y) => new(
        this,
        UiaCall.Run("element from point", () =>
            Client.ElementFromPoint(new Com.tagPOINT
            {
                x = (int)Math.Round(x),
                y = (int)Math.Round(y)
            })));

    public static HoverElementSnapshot? TryHoverElementFromPoint(
        double x,
        double y,
        int timeoutMs = 300)
    {
        var task = Task.Run(() =>
        {
            var hoverClient = CreateClient(150, 200);
            var element = hoverClient.ElementFromPoint(new Com.tagPOINT
            {
                x = (int)Math.Round(x),
                y = (int)Math.Round(y)
            });
            var rectangle = element.CurrentBoundingRectangle;
            return new HoverElementSnapshot(
                ControlTypeExtensions.FromNative(element.CurrentControlType)
                    .ToString().ToLowerInvariant(),
                element.CurrentName ?? "",
                element.CurrentAutomationId ?? "",
                element.CurrentClassName ?? "",
                element.CurrentIsEnabled != 0,
                element.CurrentIsOffscreen != 0,
                new System.Drawing.RectangleF(
                    rectangle.left,
                    rectangle.top,
                    rectangle.right - rectangle.left,
                    rectangle.bottom - rectangle.top));
        });
        try
        {
            if (task.Wait(timeoutMs)) return task.Result;
            _ = task.ContinueWith(faulted => _ = faulted.Exception,
                TaskContinuationOptions.OnlyOnFaulted | TaskContinuationOptions.ExecuteSynchronously);
        }
        catch { }
        return null;
    }

    internal Com.IUIAutomationCacheRequest CreateElementCache()
    {
        var cache = Client.CreateCacheRequest();
        cache.TreeScope = Com.TreeScope.TreeScope_Element;
        cache.AutomationElementMode = Com.AutomationElementMode.AutomationElementMode_Full;
        cache.TreeFilter = Client.ControlViewCondition;
        foreach (var property in new[]
        {
            Com.UIA_PropertyIds.UIA_NativeWindowHandlePropertyId,
            Com.UIA_PropertyIds.UIA_ProcessIdPropertyId,
            Com.UIA_PropertyIds.UIA_NamePropertyId,
            Com.UIA_PropertyIds.UIA_AutomationIdPropertyId,
            Com.UIA_PropertyIds.UIA_ClassNamePropertyId,
            Com.UIA_PropertyIds.UIA_IsEnabledPropertyId,
            Com.UIA_PropertyIds.UIA_IsOffscreenPropertyId,
            Com.UIA_PropertyIds.UIA_HasKeyboardFocusPropertyId,
            Com.UIA_PropertyIds.UIA_BoundingRectanglePropertyId,
            Com.UIA_PropertyIds.UIA_ControlTypePropertyId,
        }) cache.AddProperty(property);
        return cache;
    }

    private static Com.IUIAutomation CreateClient(
        int connectionTimeoutMs = 1_000,
        int transactionTimeoutMs = 1_500)
    {
        var automation = new Com.CUIAutomation8Class();
        if (automation is Com.IUIAutomation2 bounded)
        {
            bounded.ConnectionTimeout = (uint)connectionTimeoutMs;
            bounded.TransactionTimeout = (uint)transactionTimeoutMs;
        }
        return automation;
    }
}

internal sealed class ConditionBase
{
    internal ConditionBase(Com.IUIAutomationCondition condition) { Inner = condition; }
    internal Com.IUIAutomationCondition Inner { get; }
}

internal sealed class NativeConditionFactory
{
    private readonly NativeAutomation automation;
    public NativeConditionFactory(NativeAutomation automation) { this.automation = automation; }
    public ConditionBase ByAutomationId(string value) => Create(Com.UIA_PropertyIds.UIA_AutomationIdPropertyId, value);
    public ConditionBase ByName(string value) => Create(Com.UIA_PropertyIds.UIA_NamePropertyId, value);
    public ConditionBase ByClassName(string value) => Create(Com.UIA_PropertyIds.UIA_ClassNamePropertyId, value);
    public ConditionBase ByControlType(ControlType value) => Create(Com.UIA_PropertyIds.UIA_ControlTypePropertyId, value.ToNative());
    // Property conditions are constructed inside the UIA client and do not
    // call the application provider. Keeping this outside the provider gate
    // prevents a slow SAP query from being misreported as condition creation.
    private ConditionBase Create(int property, object value) => new(
        automation.Client.CreatePropertyCondition(property, value));
}

internal enum ControlType
{
    Custom, Button, Edit, CheckBox, RadioButton, ComboBox, List, ListItem,
    Hyperlink, Image, Text, Window
}

internal static class ControlTypeExtensions
{
    public static int ToNative(this ControlType value) => value switch
    {
        ControlType.Button => Com.UIA_ControlTypeIds.UIA_ButtonControlTypeId,
        ControlType.Edit => Com.UIA_ControlTypeIds.UIA_EditControlTypeId,
        ControlType.CheckBox => Com.UIA_ControlTypeIds.UIA_CheckBoxControlTypeId,
        ControlType.RadioButton => Com.UIA_ControlTypeIds.UIA_RadioButtonControlTypeId,
        ControlType.ComboBox => Com.UIA_ControlTypeIds.UIA_ComboBoxControlTypeId,
        ControlType.List => Com.UIA_ControlTypeIds.UIA_ListControlTypeId,
        ControlType.ListItem => Com.UIA_ControlTypeIds.UIA_ListItemControlTypeId,
        ControlType.Hyperlink => Com.UIA_ControlTypeIds.UIA_HyperlinkControlTypeId,
        ControlType.Image => Com.UIA_ControlTypeIds.UIA_ImageControlTypeId,
        ControlType.Text => Com.UIA_ControlTypeIds.UIA_TextControlTypeId,
        ControlType.Window => Com.UIA_ControlTypeIds.UIA_WindowControlTypeId,
        _ => Com.UIA_ControlTypeIds.UIA_CustomControlTypeId,
    };

    public static ControlType FromNative(int value)
    {
        if (value == Com.UIA_ControlTypeIds.UIA_ButtonControlTypeId) return ControlType.Button;
        if (value == Com.UIA_ControlTypeIds.UIA_EditControlTypeId) return ControlType.Edit;
        if (value == Com.UIA_ControlTypeIds.UIA_CheckBoxControlTypeId) return ControlType.CheckBox;
        if (value == Com.UIA_ControlTypeIds.UIA_RadioButtonControlTypeId) return ControlType.RadioButton;
        if (value == Com.UIA_ControlTypeIds.UIA_ComboBoxControlTypeId) return ControlType.ComboBox;
        if (value == Com.UIA_ControlTypeIds.UIA_ListControlTypeId) return ControlType.List;
        if (value == Com.UIA_ControlTypeIds.UIA_ListItemControlTypeId) return ControlType.ListItem;
        if (value == Com.UIA_ControlTypeIds.UIA_HyperlinkControlTypeId) return ControlType.Hyperlink;
        if (value == Com.UIA_ControlTypeIds.UIA_ImageControlTypeId) return ControlType.Image;
        if (value == Com.UIA_ControlTypeIds.UIA_TextControlTypeId) return ControlType.Text;
        if (value == Com.UIA_ControlTypeIds.UIA_WindowControlTypeId) return ControlType.Window;
        return ControlType.Custom;
    }
}

internal readonly struct PropertyValue<T>
{
    public PropertyValue(T value) { ValueOrDefault = value; }
    public T ValueOrDefault { get; }
}

internal sealed class AutomationElement
{
    private readonly NativeAutomation automation;
    private Com.IUIAutomationElement? inner;
    private readonly IntPtr fallbackWindow;
    private readonly bool cached;

    internal AutomationElement(
        NativeAutomation automation,
        Com.IUIAutomationElement inner,
        IntPtr fallbackWindow = default,
        bool cached = false)
    {
        this.automation = automation;
        this.inner = inner;
        this.fallbackWindow = fallbackWindow;
        this.cached = cached;
    }

    private AutomationElement(NativeAutomation automation, IntPtr fallbackWindow)
    {
        this.automation = automation;
        this.fallbackWindow = fallbackWindow;
    }

    public static AutomationElement FromWindow(NativeAutomation automation, IntPtr handle) => new(automation, handle);
    public ElementProperties Properties => new(this);
    public ElementPatterns Patterns => new(this);

    internal Com.IUIAutomationElement? TryInner()
    {
        if (inner is not null) return inner;
        if (fallbackWindow == IntPtr.Zero) return null;
        inner = UiaCall.Or<Com.IUIAutomationElement?>(
            "window attachment", () => automation.Client.ElementFromHandle(fallbackWindow), null);
        return inner;
    }

    internal T Property<T>(string operation, Func<Com.IUIAutomationElement, T> read, T fallback)
    {
        var element = TryInner();
        return element is null ? fallback : UiaCall.Or(operation, () => read(element), fallback);
    }

    internal object? Pattern(int patternId)
    {
        var element = TryInner();
        return element is null ? null : UiaCall.Or<object?>(
            "pattern lookup", () => element.GetCurrentPattern(patternId), null);
    }

    public AutomationElement[] FindAllDescendants(ConditionBase? condition = null) => Find(Com.TreeScope.TreeScope_Descendants, condition);
    public AutomationElement[] FindAllChildren(ConditionBase? condition = null) => Find(Com.TreeScope.TreeScope_Children, condition);
    private AutomationElement[] Find(Com.TreeScope scope, ConditionBase? condition)
    {
        var element = TryInner();
        if (element is null) return [];
        AutomationElement[] Enumerate()
        {
            var actualCondition = condition?.Inner ?? automation.Client.CreateTrueCondition();
            var useCache = scope == Com.TreeScope.TreeScope_Children && condition is null;
            var collection = useCache
                ? element.FindAllBuildCache(scope, actualCondition, automation.CreateElementCache())
                : element.FindAll(scope, actualCondition);
            var result = new AutomationElement[collection.Length];
            for (var index = 0; index < result.Length; index++)
                result[index] = new AutomationElement(
                    automation,
                    collection.GetElement(index),
                    cached: useCache);
            return result;
        }
        // Hierarchy snapshots tolerate a slow child branch and mark/truncate
        // what they could read. Locator queries must surface the provider
        // timeout so their outer retry deadline can make another observation.
        return scope == Com.TreeScope.TreeScope_Children && condition is null
            ? UiaCall.Or("child enumeration", Enumerate, Array.Empty<AutomationElement>())
            : UiaCall.Run("element enumeration", Enumerate);
    }

    public void Focus()
    {
        var element = TryInner();
        if (element is not null) UiaCall.Run("focus", () => { element.SetFocus(); return true; });
    }

    public void Click(string button = "left", uint clickCount = 1)
    {
        if (clickCount is < 1 or > 3)
            throw new InvalidOperationException("click_count must be between 1 and 3");
        if (button == "left" && clickCount == 1
            && Pattern(Com.UIA_PatternIds.UIA_InvokePatternId) is Com.IUIAutomationInvokePattern invoke)
        {
            UiaCall.Run("invoke", () => { invoke.Invoke(); return true; });
            return;
        }
        var bounds = Properties.BoundingRectangle.ValueOrDefault;
        WindowRoot().AsWindow().Activate();
        var point = new System.Drawing.Point(
            (int)Math.Round(bounds.X + bounds.Width / 2), (int)Math.Round(bounds.Y + bounds.Height / 2));
        for (var index = 0; index < clickCount; index++)
        {
            switch (button)
            {
                case "left": NativeMouse.LeftClick(point); break;
                case "middle": NativeMouse.MiddleClick(point); break;
                case "right": NativeMouse.RightClick(point); break;
                default: throw new InvalidOperationException($"unsupported mouse button `{button}`; expected left, middle, or right");
            }
            if (index + 1 < clickCount) Thread.Sleep(60);
        }
    }

    public NativeWindow AsWindow() => new(this);

    public AutomationElement WindowRoot()
    {
        if (Properties.NativeWindowHandle.ValueOrDefault != 0) return this;
        var current = TryInner();
        if (current is null) return this;
        var walker = UiaCall.Or<Com.IUIAutomationTreeWalker?>("tree walker lookup", () => automation.Client.ControlViewWalker, null);
        if (walker is null) return this;
        for (var depth = 0; depth < 64; depth++)
        {
            var child = current;
            current = UiaCall.Or<Com.IUIAutomationElement?>("parent lookup", () => walker.GetParentElement(child), null);
            if (current is null) break;
            var wrapped = new AutomationElement(automation, current);
            if (wrapped.Properties.NativeWindowHandle.ValueOrDefault != 0) return wrapped;
        }
        return this;
    }

    internal IntPtr FallbackWindow => fallbackWindow;
    internal bool HasCachedProperties => cached;
}

internal sealed class ElementProperties
{
    private readonly AutomationElement element;
    public ElementProperties(AutomationElement element) { this.element = element; }
    public PropertyValue<int> NativeWindowHandle => new(element.FallbackWindow != IntPtr.Zero
        ? unchecked((int)element.FallbackWindow.ToInt64())
        : element.Property("native window handle read", value => unchecked((int)(element.HasCachedProperties
            ? value.CachedNativeWindowHandle : value.CurrentNativeWindowHandle).ToInt64()), 0));
    public PropertyValue<int> ProcessId => new(element.FallbackWindow != IntPtr.Zero
        ? NativeWindow.ProcessId(element.FallbackWindow)
        : element.Property("process id read", value => element.HasCachedProperties ? value.CachedProcessId : value.CurrentProcessId, 0));
    public PropertyValue<string> Name => new(element.FallbackWindow != IntPtr.Zero
        ? NativeWindow.Title(element.FallbackWindow)
        : element.Property("name read", value => (element.HasCachedProperties ? value.CachedName : value.CurrentName) ?? "", ""));
    public PropertyValue<string> AutomationId => new(element.Property(
        "automation id read", value => (element.HasCachedProperties ? value.CachedAutomationId : value.CurrentAutomationId) ?? "", ""));
    public PropertyValue<string> ClassName => new(element.FallbackWindow != IntPtr.Zero
        ? NativeWindow.ClassName(element.FallbackWindow)
        : element.Property("class name read", value => (element.HasCachedProperties ? value.CachedClassName : value.CurrentClassName) ?? "", ""));
    public PropertyValue<bool> IsEnabled => new(element.FallbackWindow != IntPtr.Zero
        ? NativeWindow.IsEnabled(element.FallbackWindow)
        : element.Property("enabled state read", value => (element.HasCachedProperties ? value.CachedIsEnabled : value.CurrentIsEnabled) != 0, false));
    public PropertyValue<bool> IsOffscreen => new(element.FallbackWindow != IntPtr.Zero
        ? !NativeWindow.IsVisible(element.FallbackWindow)
        : element.Property("offscreen state read", value => (element.HasCachedProperties ? value.CachedIsOffscreen : value.CurrentIsOffscreen) != 0, true));
    public PropertyValue<bool> HasKeyboardFocus => new(element.Property(
        "focus state read", value => (element.HasCachedProperties ? value.CachedHasKeyboardFocus : value.CurrentHasKeyboardFocus) != 0, false));
    public PropertyValue<System.Drawing.RectangleF> BoundingRectangle => new(element.FallbackWindow != IntPtr.Zero
        ? NativeWindow.Bounds(element.FallbackWindow)
        : element.Property("bounds read", value => Rectangle(element.HasCachedProperties
            ? value.CachedBoundingRectangle : value.CurrentBoundingRectangle), System.Drawing.RectangleF.Empty));
    public PropertyValue<ControlType> ControlType => new(element.FallbackWindow != IntPtr.Zero
        ? Allwright.WindowsAgent.ControlType.Window
        : element.Property("control type read", value => ControlTypeExtensions.FromNative(element.HasCachedProperties
            ? value.CachedControlType : value.CurrentControlType), Allwright.WindowsAgent.ControlType.Custom));
    private static System.Drawing.RectangleF Rectangle(Com.tagRECT value) =>
        new(value.left, value.top, value.right - value.left, value.bottom - value.top);
}

internal sealed class ElementPatterns
{
    private readonly AutomationElement element;
    public ElementPatterns(AutomationElement element) { this.element = element; }
    public PatternAccessor<NativeValuePattern> Value => new(
        () => element.Pattern(Com.UIA_PatternIds.UIA_ValuePatternId) is Com.IUIAutomationValuePattern value ? new(value) : null);
    public PatternAccessor<NativeScrollPattern> Scroll => new(
        () => element.Pattern(Com.UIA_PatternIds.UIA_ScrollPatternId) is Com.IUIAutomationScrollPattern value ? new(value) : null);
    public PatternAccessor<NativeTogglePattern> Toggle => new(
        () => element.Pattern(Com.UIA_PatternIds.UIA_TogglePatternId) is Com.IUIAutomationTogglePattern value ? new(value) : null);
    public PatternAccessor<NativeSelectionItemPattern> SelectionItem => new(
        () => element.Pattern(Com.UIA_PatternIds.UIA_SelectionItemPatternId) is Com.IUIAutomationSelectionItemPattern value ? new(value) : null);
}

internal sealed class PatternAccessor<T> where T : class
{
    private readonly Func<T?> resolve;
    public PatternAccessor(Func<T?> resolve) { this.resolve = resolve; }
    public bool IsSupported => resolve() is not null;
    public T Pattern => resolve() ?? throw new InvalidOperationException("UI Automation pattern is unavailable");
    public bool TryGetPattern(out T pattern) { pattern = resolve()!; return pattern is not null; }
}

internal sealed class NativeValuePattern
{
    private readonly Com.IUIAutomationValuePattern pattern;
    public NativeValuePattern(Com.IUIAutomationValuePattern pattern) { this.pattern = pattern; }
    public PropertyValue<string> Value => new(UiaCall.Or("value read", () => pattern.CurrentValue ?? "", ""));
    public PropertyValue<bool> IsReadOnly => new(UiaCall.Or("read-only read", () => pattern.CurrentIsReadOnly != 0, true));
    public void SetValue(string value) => UiaCall.Run("value set", () => { pattern.SetValue(value); return true; });
}

internal enum ScrollAmount { NoAmount, LargeIncrement, LargeDecrement }

internal sealed class NativeScrollPattern
{
    private readonly Com.IUIAutomationScrollPattern pattern;
    public NativeScrollPattern(Com.IUIAutomationScrollPattern pattern) { this.pattern = pattern; }
    public PropertyValue<bool> VerticallyScrollable => new(UiaCall.Or("scroll state", () => pattern.CurrentVerticallyScrollable != 0, false));
    public PropertyValue<double> VerticalScrollPercent => new(UiaCall.Or("scroll percent", () => pattern.CurrentVerticalScrollPercent, -1d));
    public void Scroll(ScrollAmount horizontal, ScrollAmount vertical) => UiaCall.Run("scroll", () =>
    {
        pattern.Scroll(Map(horizontal), Map(vertical)); return true;
    });
    public void SetScrollPercent(double horizontal, double vertical) => UiaCall.Run("scroll restore", () =>
    {
        pattern.SetScrollPercent(horizontal, vertical); return true;
    });
    private static Com.ScrollAmount Map(ScrollAmount amount) => amount switch
    {
        ScrollAmount.LargeIncrement => Com.ScrollAmount.ScrollAmount_LargeIncrement,
        ScrollAmount.LargeDecrement => Com.ScrollAmount.ScrollAmount_LargeDecrement,
        _ => Com.ScrollAmount.ScrollAmount_NoAmount,
    };
}

internal enum ToggleState { Off, On, Indeterminate }

internal sealed class NativeTogglePattern
{
    private readonly Com.IUIAutomationTogglePattern pattern;
    public NativeTogglePattern(Com.IUIAutomationTogglePattern pattern) { this.pattern = pattern; }
    public PropertyValue<ToggleState> ToggleState => new(UiaCall.Or("toggle state", () => pattern.CurrentToggleState switch
    {
        Com.ToggleState.ToggleState_On => Allwright.WindowsAgent.ToggleState.On,
        Com.ToggleState.ToggleState_Indeterminate => Allwright.WindowsAgent.ToggleState.Indeterminate,
        _ => Allwright.WindowsAgent.ToggleState.Off,
    }, Allwright.WindowsAgent.ToggleState.Off));
}

internal sealed class NativeSelectionItemPattern
{
    private readonly Com.IUIAutomationSelectionItemPattern pattern;
    public NativeSelectionItemPattern(Com.IUIAutomationSelectionItemPattern pattern) { this.pattern = pattern; }
    public PropertyValue<bool> IsSelected => new(UiaCall.Or("selection state", () => pattern.CurrentIsSelected != 0, false));
}

internal sealed class NativeWindow
{
    private readonly AutomationElement element;
    public NativeWindow(AutomationElement element) { this.element = element; }
    public void Close()
    {
        if (element.Pattern(Com.UIA_PatternIds.UIA_WindowPatternId) is Com.IUIAutomationWindowPattern window)
        {
            UiaCall.Run("window close", () => { window.Close(); return true; }); return;
        }
        var handle = new IntPtr(element.Properties.NativeWindowHandle.ValueOrDefault);
        if (handle != IntPtr.Zero) PostMessage(handle, 0x0010, IntPtr.Zero, IntPtr.Zero);
    }
    public void Activate()
    {
        var handle = new IntPtr(element.Properties.NativeWindowHandle.ValueOrDefault);
        if (handle == IntPtr.Zero)
            throw new InvalidOperationException("the Windows input target has no native window handle");
        if (IsIconic(handle)) _ = ShowWindowAsync(handle, 9);

        var currentThread = GetCurrentThreadId();
        var targetThread = GetWindowThreadProcessId(handle, out var targetProcessId);
        var foreground = GetForegroundWindow();
        var foregroundThread = foreground == IntPtr.Zero
            ? 0
            : GetWindowThreadProcessId(foreground, out _);
        var attachedTarget = targetThread != 0 && targetThread != currentThread
            && AttachThreadInput(currentThread, targetThread, true);
        var attachedForeground = foregroundThread != 0
            && foregroundThread != currentThread
            && foregroundThread != targetThread
            && AttachThreadInput(currentThread, foregroundThread, true);
        try
        {
            _ = BringWindowToTop(handle);
            _ = SetForegroundWindow(handle);
            _ = SetFocus(handle);
        }
        finally
        {
            if (attachedForeground) _ = AttachThreadInput(currentThread, foregroundThread, false);
            if (attachedTarget) _ = AttachThreadInput(currentThread, targetThread, false);
        }

        Thread.Sleep(50);
        var actualForeground = GetForegroundWindow();
        if (actualForeground == IntPtr.Zero) return;
        _ = GetWindowThreadProcessId(actualForeground, out var actualProcessId);
        if (actualProcessId != targetProcessId)
            throw new InvalidOperationException(
                "Windows refused to activate the inspected application window; input was not sent to the inspector");
    }
    [DllImport("user32.dll")] private static extern bool PostMessage(IntPtr handle, uint message, IntPtr wParam, IntPtr lParam);
    public static int ProcessId(IntPtr handle)
    {
        if (handle == IntPtr.Zero) return 0;
        GetWindowThreadProcessId(handle, out var id); return unchecked((int)id);
    }
    public static string Title(IntPtr handle) => WindowText(handle);
    public static string ClassName(IntPtr handle)
    {
        if (handle == IntPtr.Zero) return "";
        var value = new System.Text.StringBuilder(256);
        _ = GetClassName(handle, value, value.Capacity); return value.ToString();
    }
    public static bool IsVisible(IntPtr handle) => handle != IntPtr.Zero && IsWindowVisible(handle);
    public static bool IsEnabled(IntPtr handle) => handle != IntPtr.Zero && IsWindowEnabled(handle);
    public static System.Drawing.RectangleF Bounds(IntPtr handle)
    {
        if (handle != IntPtr.Zero && GetWindowRect(handle, out var bounds))
            return new(bounds.Left, bounds.Top, bounds.Right - bounds.Left, bounds.Bottom - bounds.Top);
        return System.Drawing.RectangleF.Empty;
    }
    private static string WindowText(IntPtr handle)
    {
        if (handle == IntPtr.Zero) return "";
        var length = GetWindowTextLength(handle);
        var value = new System.Text.StringBuilder(length + 1);
        _ = GetWindowText(handle, value, value.Capacity); return value.ToString();
    }
    [StructLayout(LayoutKind.Sequential)] private struct Rect { public int Left, Top, Right, Bottom; }
    [DllImport("user32.dll")] private static extern uint GetWindowThreadProcessId(IntPtr handle, out uint processId);
    [DllImport("kernel32.dll")] private static extern uint GetCurrentThreadId();
    [DllImport("user32.dll")] private static extern bool AttachThreadInput(uint attach, uint attachTo, bool value);
    [DllImport("user32.dll")] private static extern IntPtr GetForegroundWindow();
    [DllImport("user32.dll")] private static extern bool BringWindowToTop(IntPtr handle);
    [DllImport("user32.dll")] private static extern bool SetForegroundWindow(IntPtr handle);
    [DllImport("user32.dll")] private static extern IntPtr SetFocus(IntPtr handle);
    [DllImport("user32.dll")] private static extern bool IsIconic(IntPtr handle);
    [DllImport("user32.dll")] private static extern bool ShowWindowAsync(IntPtr handle, int command);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] private static extern int GetWindowText(IntPtr handle, System.Text.StringBuilder text, int max);
    [DllImport("user32.dll")] private static extern int GetWindowTextLength(IntPtr handle);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] private static extern int GetClassName(IntPtr handle, System.Text.StringBuilder text, int max);
    [DllImport("user32.dll")] private static extern bool IsWindowVisible(IntPtr handle);
    [DllImport("user32.dll")] private static extern bool IsWindowEnabled(IntPtr handle);
    [DllImport("user32.dll")] private static extern bool GetWindowRect(IntPtr handle, out Rect bounds);
}

internal enum MouseButton { Left }

internal static class Mouse
{
    public static void LeftClick(System.Drawing.Point point) => NativeMouse.LeftClick(point);
    public static void RightClick(System.Drawing.Point point) => NativeMouse.RightClick(point);
    public static void LeftDoubleClick(System.Drawing.Point point) { LeftClick(point); Thread.Sleep(60); LeftClick(point); }
    public static void MoveTo(System.Drawing.Point point) => SetCursorPos(point.X, point.Y);
    public static void Drag(System.Drawing.Point from, System.Drawing.Point to, MouseButton _) => NativeMouse.Drag(from, to);
    public static void Scroll(int amount) => mouse_event(0x0800, 0, 0, amount * 120, UIntPtr.Zero);
    public static void HorizontalScroll(int amount) => mouse_event(0x01000, 0, 0, amount * 120, UIntPtr.Zero);
    [DllImport("user32.dll")] private static extern bool SetCursorPos(int x, int y);
    [DllImport("user32.dll")] private static extern void mouse_event(uint flags, uint dx, uint dy, int data, UIntPtr extra);
}

internal static class NativeMouse
{
    public static void LeftClick(System.Drawing.Point point)
    {
        SetCursorPos(point.X, point.Y);
        mouse_event(0x0002, 0, 0, 0, UIntPtr.Zero); mouse_event(0x0004, 0, 0, 0, UIntPtr.Zero);
    }
    public static void MiddleClick(System.Drawing.Point point)
    {
        SetCursorPos(point.X, point.Y);
        mouse_event(0x0020, 0, 0, 0, UIntPtr.Zero); mouse_event(0x0040, 0, 0, 0, UIntPtr.Zero);
    }
    public static void RightClick(System.Drawing.Point point)
    {
        SetCursorPos(point.X, point.Y);
        mouse_event(0x0008, 0, 0, 0, UIntPtr.Zero); mouse_event(0x0010, 0, 0, 0, UIntPtr.Zero);
    }
    public static void Drag(System.Drawing.Point from, System.Drawing.Point to)
    {
        SetCursorPos(from.X, from.Y); mouse_event(0x0002, 0, 0, 0, UIntPtr.Zero);
        for (var step = 1; step <= 12; step++)
        {
            SetCursorPos(from.X + (to.X - from.X) * step / 12, from.Y + (to.Y - from.Y) * step / 12);
            Thread.Sleep(10);
        }
        mouse_event(0x0004, 0, 0, 0, UIntPtr.Zero);
    }
    [DllImport("user32.dll")] private static extern bool SetCursorPos(int x, int y);
    [DllImport("user32.dll")] private static extern void mouse_event(uint flags, uint dx, uint dy, int data, UIntPtr extra);
}

internal enum VirtualKeyShort : ushort
{
    BACK = 0x08, TAB = 0x09, RETURN = 0x0D, CONTROL = 0x11, ESCAPE = 0x1B,
    SPACE = 0x20, PRIOR = 0x21, NEXT = 0x22, LEFT = 0x25, UP = 0x26,
    RIGHT = 0x27, DOWN = 0x28, DELETE = 0x2E, KEY_A = 0x41
}

internal static class Keyboard
{
    public static void Type(string text)
    {
        foreach (var character in text) { SendUnicode(character, false); SendUnicode(character, true); }
    }
    public static void Type(VirtualKeyShort key) { Press(key); Release(key); }
    public static void Press(VirtualKeyShort key) => SendVirtualKey(key, false);
    public static void Release(VirtualKeyShort key) => SendVirtualKey(key, true);
    private static void SendUnicode(char character, bool up) => Send(new Input
    {
        Type = 1, Data = new InputUnion { Keyboard = new KeyboardInput { Scan = character, Flags = 0x0004u | (up ? 0x0002u : 0) } }
    });
    private static void SendVirtualKey(VirtualKeyShort key, bool up) => Send(new Input
    {
        Type = 1, Data = new InputUnion { Keyboard = new KeyboardInput { VirtualKey = (ushort)key, Flags = up ? 0x0002u : 0 } }
    });
    private static void Send(Input input)
    {
        if (SendInput(1, [input], Marshal.SizeOf<Input>()) != 1) throw new InvalidOperationException("Windows SendInput failed");
    }
    [StructLayout(LayoutKind.Sequential)] private struct Input { public uint Type; public InputUnion Data; }
    [StructLayout(LayoutKind.Explicit)] private struct InputUnion { [FieldOffset(0)] public KeyboardInput Keyboard; }
    [StructLayout(LayoutKind.Sequential)] private struct KeyboardInput
    {
        public ushort VirtualKey, Scan; public uint Flags, Time; public UIntPtr ExtraInfo;
    }
    [DllImport("user32.dll", SetLastError = true)] private static extern uint SendInput(uint count, Input[] inputs, int size);
}
