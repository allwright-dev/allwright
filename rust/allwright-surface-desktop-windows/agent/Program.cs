using System.Net;
using System.Net.Sockets;
using System.Text;
using System.Text.Json;
using System.Text.Json.Nodes;
using System.Text.RegularExpressions;
using FlaUI.Core;
using FlaUI.Core.AutomationElements;
using FlaUI.Core.Capturing;
using FlaUI.Core.Conditions;
using FlaUI.Core.Definitions;
using FlaUI.Core.Input;
using FlaUI.Core.WindowsAPI;
using FlaUI.UIA3;
using FlaApplication = FlaUI.Core.Application;
using FlaCapture = FlaUI.Core.Capturing.Capture;

namespace Allwright.WindowsAgent;

internal sealed record AgentRequest(
    string Command,
    string? SessionId,
    string? AppId,
    string? Selector,
    string? Value,
    string? Key,
    string? Text,
    bool? Visible,
    uint? TimeoutMs,
    bool? TerminateRunning);

internal sealed record AppSession(string Id, string AppId, FlaApplication Application, AutomationElement Root);

internal static class Program
{
    private static readonly JsonSerializerOptions JsonOptions = new(JsonSerializerDefaults.Web)
    {
        PropertyNamingPolicy = JsonNamingPolicy.SnakeCaseLower
    };
    private static readonly UIA3Automation Automation = new();
    private static readonly Dictionary<string, AppSession> Sessions = new();

    [STAThread]
    private static void Main(string[] args)
    {
        var port = 8300;
        var portIndex = Array.IndexOf(args, "--port");
        if (portIndex >= 0 && portIndex + 1 < args.Length) port = int.Parse(args[portIndex + 1]);
        var listener = new TcpListener(IPAddress.Loopback, port);
        listener.Start();
        while (true)
        {
            using var client = listener.AcceptTcpClient();
            Handle(client);
        }
    }

    private static void Handle(TcpClient client)
    {
        object envelope;
        try
        {
            var body = ReadRequestBody(client.GetStream());
            var request = JsonSerializer.Deserialize<AgentRequest>(body, JsonOptions)
                ?? throw new InvalidOperationException("request body is empty");
            envelope = new { ok = true, result = Execute(request), error = (string?)null };
        }
        catch (Exception error)
        {
            envelope = new { ok = false, result = (object?)null, error = error.Message };
        }
        var data = JsonSerializer.SerializeToUtf8Bytes(envelope, JsonOptions);
        var header = Encoding.ASCII.GetBytes($"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {data.Length}\r\nConnection: close\r\n\r\n");
        var stream = client.GetStream();
        stream.Write(header);
        stream.Write(data);
    }

    private static byte[] ReadRequestBody(NetworkStream stream)
    {
        var bytes = new List<byte>();
        var buffer = new byte[4096];
        var headerEnd = -1;
        var contentLength = 0;
        while (true)
        {
            var read = stream.Read(buffer);
            if (read == 0) break;
            bytes.AddRange(buffer.AsSpan(0, read).ToArray());
            if (headerEnd < 0)
            {
                var text = Encoding.ASCII.GetString(bytes.ToArray());
                headerEnd = text.IndexOf("\r\n\r\n", StringComparison.Ordinal);
                if (headerEnd >= 0)
                {
                    var requestLine = text[..headerEnd].Split("\r\n")[0];
                    if (!requestLine.StartsWith("POST /v1/command ", StringComparison.Ordinal))
                        throw new InvalidOperationException("expected POST /v1/command");
                    var match = Regex.Match(text[..headerEnd], @"(?im)^Content-Length:\s*(\d+)\s*$");
                    contentLength = match.Success ? int.Parse(match.Groups[1].Value) : 0;
                    headerEnd += 4;
                }
            }
            if (headerEnd >= 0 && bytes.Count >= headerEnd + contentLength)
                return bytes.Skip(headerEnd).Take(contentLength).ToArray();
        }
        throw new InvalidOperationException("incomplete HTTP request");
    }

    private static object Execute(AgentRequest request) => request.Command switch
    {
        "status" => new { host_name = Environment.MachineName, backend = "flaui-uia3" },
        "launch" => Launch(request),
        "close" => Close(request),
        "count" => new { count = FindAll(Session(request).Root, Required(request.Selector, "selector")).Length },
        "click" => WithElement(request, element => { element.Click(); return new { }; }),
        "focus" => WithElement(request, element => { element.Focus(); return new { }; }),
        "fill" => WithElement(request, element => { Fill(element, request.Value ?? ""); return new { }; }),
        "press" => WithElement(request, element => { element.Focus(); Press(request.Key ?? "", request.Text); return new { }; }),
        "text" => WithElement(request, element => new { text = ElementText(element) }),
        "wait" => Wait(request),
        "screenshot" => Screenshot(request),
        "source" => Source(request),
        _ => throw new InvalidOperationException($"unknown command `{request.Command}`")
    };

    private static object Launch(AgentRequest request)
    {
        var appId = Required(request.AppId, "app_id");
        if (request.TerminateRunning == true)
        {
            var processName = Path.GetFileNameWithoutExtension(appId);
            foreach (var process in System.Diagnostics.Process.GetProcessesByName(processName))
                try { process.Kill(true); process.WaitForExit(5_000); } catch { }
        }
        var app = appId.Contains('!', StringComparison.Ordinal)
            ? FlaApplication.LaunchStoreApp(appId)
            : FlaApplication.Launch(appId);
        var timeout = TimeSpan.FromMilliseconds(request.TimeoutMs ?? 30_000);
        var root = app.GetMainWindow(Automation, timeout)
            ?? throw new InvalidOperationException($"application `{appId}` did not expose a main window before timeout");
        var id = $"windows-app-{Guid.NewGuid():N}";
        Sessions[id] = new AppSession(id, appId, app, root);
        return new { session_id = id, process_id = app.ProcessId };
    }

    private static object Close(AgentRequest request)
    {
        var session = Session(request);
        Sessions.Remove(session.Id);
        try { session.Application.Close(); } catch { session.Application.Kill(); }
        return new { };
    }

    private static object WithElement(AgentRequest request, Func<AutomationElement, object> operation)
    {
        var selector = Required(request.Selector, "selector");
        var timeout = TimeSpan.FromMilliseconds(request.TimeoutMs ?? 10_000);
        var deadline = DateTime.UtcNow + timeout;
        Exception? last = null;
        do
        {
            try
            {
                var element = FindAll(Session(request).Root, selector).FirstOrDefault();
                if (element is not null && !element.Properties.IsOffscreen.ValueOrDefault) return operation(element);
            }
            catch (Exception error) { last = error; }
            Thread.Sleep(100);
        } while (DateTime.UtcNow < deadline);
        throw new InvalidOperationException($"no visible Windows element matched `{selector}` before timeout", last);
    }

    private static object Wait(AgentRequest request)
    {
        var selector = Required(request.Selector, "selector");
        var expected = request.Visible ?? true;
        var deadline = DateTime.UtcNow.AddMilliseconds(request.TimeoutMs ?? 10_000);
        do
        {
            var visible = FindAll(Session(request).Root, selector).Any(e => !e.Properties.IsOffscreen.ValueOrDefault);
            if (visible == expected) return new { visible };
            Thread.Sleep(100);
        } while (DateTime.UtcNow < deadline);
        throw new InvalidOperationException($"selector `{selector}` did not become {(expected ? "visible" : "hidden")} before timeout");
    }

    private static object Screenshot(AgentRequest request)
    {
        var path = Path.Combine(Path.GetTempPath(), $"allwright-{Guid.NewGuid():N}.png");
        try
        {
            FlaCapture.Element(Session(request).Root).ToFile(path);
            return new { png_base64 = Convert.ToBase64String(File.ReadAllBytes(path)) };
        }
        finally { if (File.Exists(path)) File.Delete(path); }
    }

    private static object Source(AgentRequest request)
    {
        var snapshot = new JsonObject
        {
            ["version"] = 1,
            ["platform"] = "windows",
            ["backend"] = "uia3",
            ["root"] = SnapshotElement(Session(request).Root, 0)
        };
        return new { snapshot = snapshot.ToJsonString(JsonOptions) };
    }

    private static JsonObject SnapshotElement(AutomationElement element, int depth)
    {
        var rectangle = element.Properties.BoundingRectangle.ValueOrDefault;
        var node = new JsonObject
        {
            ["role"] = element.Properties.ControlType.ValueOrDefault.ToString().ToLowerInvariant(),
            ["name"] = element.Properties.Name.ValueOrDefault ?? "",
            ["automationId"] = element.Properties.AutomationId.ValueOrDefault ?? "",
            ["className"] = element.Properties.ClassName.ValueOrDefault ?? "",
            ["enabled"] = element.Properties.IsEnabled.ValueOrDefault,
            ["offscreen"] = element.Properties.IsOffscreen.ValueOrDefault,
            ["bounds"] = new JsonObject { ["x"] = rectangle.X, ["y"] = rectangle.Y, ["width"] = rectangle.Width, ["height"] = rectangle.Height }
        };
        var children = new JsonArray();
        if (depth < 40)
            foreach (var child in element.FindAllChildren()) children.Add(SnapshotElement(child, depth + 1));
        node["children"] = children;
        return node;
    }

    private static AutomationElement[] FindAll(AutomationElement root, string transport)
    {
        var segments = ParseSegments(transport);
        var current = new[] { root };
        foreach (var segment in segments)
            current = current.SelectMany(parent => FindSegment(parent, segment.Prefix, segment.Value)).ToArray();
        return current;
    }

    private static AutomationElement[] FindSegment(AutomationElement root, string prefix, string value)
    {
        if (prefix == "xpath") return FindXPath(root, value);
        if (prefix == "aw") return FindSemantic(root, JsonNode.Parse(value)?.AsObject() ?? new JsonObject());
        var condition = Condition(prefix, value);
        return root.FindAllDescendants(condition);
    }

    private static ConditionBase Condition(string prefix, string value)
    {
        var cf = Automation.ConditionFactory;
        if (prefix == "uia")
        {
            var split = value.Split('=', 2);
            prefix = split[0].ToLowerInvariant(); value = split.Length > 1 ? split[1] : "";
        }
        if (prefix == "css")
        {
            if (value.StartsWith('#')) return cf.ByAutomationId(value[1..]);
            var aria = Regex.Match(value, "\\[aria-label=['\\\"](?<v>.*?)['\\\"]\\]");
            if (aria.Success) return cf.ByName(aria.Groups["v"].Value);
            return IsRole(value) ? RoleCondition(value) : cf.ByAutomationId(value);
        }
        return prefix switch
        {
            "id" or "automationid" or "resourceid" => cf.ByAutomationId(value),
            "name" or "text" or "description" or "desc" => cf.ByName(value),
            "classname" or "class" => cf.ByClassName(value),
            "role" or "controltype" => RoleCondition(value),
            _ => cf.ByAutomationId(value)
        };
    }

    private static ConditionBase RoleCondition(string role)
    {
        var type = role.Trim().ToLowerInvariant() switch
        {
            "button" => ControlType.Button, "textbox" or "input" or "textarea" => ControlType.Edit,
            "checkbox" => ControlType.CheckBox, "radio" => ControlType.RadioButton,
            "combobox" => ControlType.ComboBox, "list" => ControlType.List,
            "listitem" => ControlType.ListItem, "link" or "a" => ControlType.Hyperlink,
            "img" or "image" => ControlType.Image, "text" or "heading" => ControlType.Text,
            "window" => ControlType.Window, _ => ControlType.Custom
        };
        return Automation.ConditionFactory.ByControlType(type);
    }

    private static bool IsRole(string value) => value.Trim().ToLowerInvariant() is
        "button" or "textbox" or "input" or "textarea" or "checkbox" or "radio" or
        "combobox" or "list" or "listitem" or "link" or "a" or "img" or "image" or
        "text" or "heading" or "window";

    private static AutomationElement[] FindSemantic(AutomationElement root, JsonObject spec)
    {
        var kind = spec["kind"]?.GetValue<string>() ?? "";
        var text = spec[kind == "role" ? "name" : "text"]?.GetValue<string>();
        var exact = spec["exact"]?.GetValue<bool>() ?? false;
        IEnumerable<AutomationElement> elements = kind switch
        {
            "role" => root.FindAllDescendants(RoleCondition(spec["role"]?.GetValue<string>() ?? "")),
            "testId" => root.FindAllDescendants(Automation.ConditionFactory.ByAutomationId(text ?? "")),
            _ => root.FindAllDescendants()
        };
        if (text is not null && kind != "testId") elements = elements.Where(e => exact ? ElementText(e) == text : ElementText(e).Contains(text, StringComparison.OrdinalIgnoreCase));
        return elements.ToArray();
    }

    private static AutomationElement[] FindXPath(AutomationElement root, string xpath)
    {
        var id = Regex.Match(xpath, "@(?:AutomationId|automationId|id)=['\\\"](?<v>.*?)['\\\"]");
        if (id.Success) return root.FindAllDescendants(Automation.ConditionFactory.ByAutomationId(id.Groups["v"].Value));
        var name = Regex.Match(xpath, "@(?:Name|name)=['\\\"](?<v>.*?)['\\\"]");
        if (name.Success) return root.FindAllDescendants(Automation.ConditionFactory.ByName(name.Groups["v"].Value));
        var tag = Regex.Match(xpath, @"(?:^|//)(?<v>[A-Za-z]+)").Groups["v"].Value;
        return root.FindAllDescendants(RoleCondition(tag));
    }

    private static List<(string Prefix, string Value)> ParseSegments(string transport)
    {
        var result = new List<(string, string)>();
        var matches = Regex.Matches(transport, "(?<p>[a-zA-Z]+)=(?<j>\\\"(?:\\\\.|[^\\\"\\\\])*\\\")");
        foreach (Match match in matches)
            result.Add((match.Groups["p"].Value.ToLowerInvariant(), JsonSerializer.Deserialize<string>(match.Groups["j"].Value)!));
        if (result.Count == 0)
        {
            var split = transport.Split('=', 2);
            result.Add((split.Length == 2 ? split[0].ToLowerInvariant() : "css", split.Length == 2 ? split[1] : transport));
        }
        return result;
    }

    private static void Fill(AutomationElement element, string value)
    {
        if (element.Patterns.Value.TryGetPattern(out var pattern)) { pattern.SetValue(value); return; }
        element.Focus(); Keyboard.Press(VirtualKeyShort.CONTROL); Keyboard.Press(VirtualKeyShort.KEY_A); Keyboard.Release(VirtualKeyShort.CONTROL); Keyboard.Type(value);
    }

    private static void Press(string key, string? text)
    {
        if (text is not null) { Keyboard.Type(text); return; }
        var virtualKey = key.ToLowerInvariant() switch
        {
            "enter" or "return" => VirtualKeyShort.RETURN, "tab" => VirtualKeyShort.TAB,
            "escape" => VirtualKeyShort.ESCAPE, "backspace" => VirtualKeyShort.BACK,
            "delete" => VirtualKeyShort.DELETE, "space" => VirtualKeyShort.SPACE,
            "arrowup" or "uparrow" => VirtualKeyShort.UP, "arrowdown" or "downarrow" => VirtualKeyShort.DOWN,
            "arrowleft" or "leftarrow" => VirtualKeyShort.LEFT, "arrowright" or "rightarrow" => VirtualKeyShort.RIGHT,
            _ => (VirtualKeyShort)0
        };
        if (virtualKey == 0) Keyboard.Type(key); else Keyboard.Type(virtualKey);
    }

    private static string ElementText(AutomationElement element)
    {
        if (element.Patterns.Value.TryGetPattern(out var value)) return value.Value.ValueOrDefault ?? "";
        return element.Properties.Name.ValueOrDefault ?? "";
    }

    private static AppSession Session(AgentRequest request)
    {
        var id = Required(request.SessionId, "session_id");
        return Sessions.TryGetValue(id, out var session) ? session : throw new InvalidOperationException($"unknown Windows app session `{id}`");
    }
    private static string Required(string? value, string name) => string.IsNullOrWhiteSpace(value) ? throw new InvalidOperationException($"request requires `{name}`") : value;
}
