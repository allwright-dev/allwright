import Foundation
import Network
import UIKit
import XCTest

/// Native XCTest bridge used by the `mobile-ios` surface plugin.
///
/// The Rust plugin deliberately never tries to synthesize iOS input itself. It
/// sends a small JSON command protocol to this process, which owns the XCTest
/// entitlement and all `XCUIApplication` / `XCUIElement` interactions.
final class AllwrightIOSAgent {
    private let queue = DispatchQueue(label: "dev.allwright.ios-agent.http")
    private var listener: NWListener?
    private var applications: [String: XCUIApplication] = [:]
    private var applicationBundleIDs: [String: String] = [:]
    private var snapshotReferences: [String: [String: SnapshotReference]] = [:]
    private var snapshotGenerations: [String: Int] = [:]

    func start() throws {
        let rawPort = ProcessInfo.processInfo.environment["ALLWRIGHT_IOS_AGENT_PORT"] ?? "8100"
        guard let portValue = UInt16(rawPort), let port = NWEndpoint.Port(rawValue: portValue) else {
            throw AgentError("ALLWRIGHT_IOS_AGENT_PORT must be a valid TCP port")
        }

        let listener = try NWListener(using: .tcp, on: port)
        listener.newConnectionHandler = { [weak self] connection in
            self?.accept(connection)
        }
        listener.stateUpdateHandler = { state in
            switch state {
            case .ready:
                print("✅ Allwright iOS Agent listening on port \(portValue)")
            case .failed(let error):
                print("❌ Allwright iOS Agent listener failed: \(error)")
            default:
                break
            }
        }
        self.listener = listener
        listener.start(queue: queue)
        print("🚀 Allwright iOS Agent started")
    }

    func waitForever() {
        RunLoop.current.run()
    }

    private func accept(_ connection: NWConnection) {
        connection.start(queue: queue)
        receive(connection, buffer: Data())
    }

    private func receive(_ connection: NWConnection, buffer: Data) {
        connection.receive(minimumIncompleteLength: 1, maximumLength: 1_048_576) { [weak self] data, _, complete, error in
            var accumulated = buffer
            if let data { accumulated.append(data) }

            if let request = HTTPRequest.parse(accumulated) {
                let response = self?.handleHTTP(request) ?? AgentResponse.failure("agent stopped")
                self?.send(response, over: connection)
                return
            }
            if complete || error != nil {
                self?.send(.failure(error?.localizedDescription ?? "incomplete HTTP request"), over: connection)
                return
            }
            self?.receive(connection, buffer: accumulated)
        }
    }

    private func send(_ response: AgentResponse, over connection: NWConnection) {
        let body = (try? JSONSerialization.data(withJSONObject: response.object)) ?? Data(
            #"{"ok":false,"result":{},"error":"response encoding failed"}"#.utf8
        )
        let header = "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: \(body.count)\r\nConnection: close\r\n\r\n"
        var payload = Data(header.utf8)
        payload.append(body)
        connection.send(content: payload, completion: .contentProcessed { _ in connection.cancel() })
    }

    private func handleHTTP(_ request: HTTPRequest) -> AgentResponse {
        guard request.method == "POST", request.path.hasSuffix("/v1/command") else {
            return .failure("expected POST /v1/command")
        }
        do {
            guard let command = try JSONSerialization.jsonObject(with: request.body) as? [String: Any] else {
                throw AgentError("request body must be a JSON object")
            }
            return try onMain { try self.execute(command) }
        } catch {
            return .failure(error.localizedDescription)
        }
    }

    private func execute(_ request: [String: Any]) throws -> AgentResponse {
        let command = try requiredString("command", in: request)
        switch command {
        case "status":
            let environment = ProcessInfo.processInfo.environment
            let identifier = environment["ALLWRIGHT_IOS_DEVICE_ID"]
                ?? environment["SIMULATOR_UDID"]
                ?? UIDevice.current.identifierForVendor?.uuidString
                ?? "ios-device"
            return .success([
                "device_id": identifier,
                "device_name": UIDevice.current.name,
                "simulator": environment["SIMULATOR_UDID"] != nil,
                "protocol_version": 1,
            ])

        case "launch":
            let bundleID = try requiredString("bundle_id", in: request)
            let application = XCUIApplication(bundleIdentifier: bundleID)
            if request["terminate_running"] as? Bool == true {
                application.terminate()
            }
            application.launch()
            let timeout = seconds(request["timeout_ms"])
            guard application.wait(for: .runningForeground, timeout: timeout) else {
                throw AgentError("app `\(bundleID)` did not reach the foreground")
            }
            let sessionID = UUID().uuidString
            applications[sessionID] = application
            applicationBundleIDs[sessionID] = bundleID
            return .success(["session_id": sessionID, "bundle_id": bundleID])

        case "close":
            let (sessionID, application) = try application(for: request)
            application.terminate()
            applications.removeValue(forKey: sessionID)
            applicationBundleIDs.removeValue(forKey: sessionID)
            snapshotReferences.removeValue(forKey: sessionID)
            snapshotGenerations.removeValue(forKey: sessionID)
            return .success(["closed": true])

        case "click":
            let element = try resolvedElement(for: request, requiringActionability: true)
            element.tap()
            return .success(["performed": true])

        case "focus":
            let element = try resolvedElement(for: request, requiringActionability: true)
            if !element.hasFocus { element.tap() }
            return .success(["performed": true])

        case "fill":
            let element = try resolvedElement(for: request, requiringActionability: true)
            let value = try requiredString("value", in: request, allowEmpty: true)
            element.tap()
            if let existing = element.value as? String, !existing.isEmpty {
                element.typeText(String(repeating: XCUIKeyboardKey.delete.rawValue, count: existing.count))
            }
            element.typeText(value)
            return .success(["performed": true])

        case "press":
            let element = try resolvedElement(for: request, requiringActionability: true)
            let key = try requiredString("key", in: request)
            if !element.hasFocus { element.tap() }
            element.typeText(keyText(key, explicitText: request["text"] as? String))
            return .success(["performed": true])

        case "count":
            let (sessionID, application) = try application(for: request)
            let selector = try requiredString("selector", in: request)
            if selector.hasPrefix("ref=") {
                return .success(["count": (try? referencedElement(selector, sessionID: sessionID, application: application)) == nil ? 0 : 1])
            }
            return .success(["count": query(selector, in: application).count])

        case "text":
            let element = try resolvedElement(for: request)
            let text = (element.value as? String).flatMap { $0.isEmpty ? nil : $0 }
                ?? (!element.label.isEmpty ? element.label : element.title)
            return .success(["text": text])

        case "wait":
            let (sessionID, application) = try application(for: request)
            let selector = try requiredString("selector", in: request)
            let visible = request["visible"] as? Bool ?? true
            let timeout = seconds(request["timeout_ms"])
            if selector.hasPrefix("ref=") {
                let deadline = Date().addingTimeInterval(timeout)
                repeat {
                    let exists = (try? referencedElement(selector, sessionID: sessionID, application: application))?.exists == true
                    if exists == visible { return .success(["visible": visible]) }
                    RunLoop.current.run(until: Date().addingTimeInterval(0.05))
                } while Date() < deadline
                throw AgentError("reference did not become \(visible ? "visible" : "hidden") before timeout")
            }
            let element = query(selector, in: application).firstMatch
            if visible, element.waitForExistence(timeout: timeout) {
                return .success(["visible": true])
            }
            if !visible {
                let expectation = XCTNSPredicateExpectation(
                    predicate: NSPredicate(format: "exists == false"),
                    object: element
                )
                if XCTWaiter.wait(for: [expectation], timeout: timeout) == .completed {
                    return .success(["visible": false])
                }
            }
            throw AgentError("selector did not become \(visible ? "visible" : "hidden") before timeout")

        case "screenshot":
            let (_, application) = try application(for: request)
            let fullPage = request["full_page"] as? Bool ?? false
            let png = fullPage ? fullPageScreenshot(application) : application.screenshot().pngRepresentation
            return .success(["png_base64": png.base64EncodedString()])

        case "file_chooser_status":
            let (_, application) = try application(for: request)
            let cancel = application.buttons.matching(
                NSPredicate(format: "label IN[c] %@", ["Cancel", "Close"])
            ).firstMatch
            let choose = application.buttons.matching(
                NSPredicate(format: "label IN[c] %@", ["Open", "Choose", "Done", "Add"])
            ).firstMatch
            let browser = application.collectionViews.firstMatch.exists
                || application.tables.firstMatch.exists
                || application.navigationBars.firstMatch.exists
            return .success([
                "opened": cancel.exists && (choose.exists || browser),
                "multiple": application.buttons.matching(
                    NSPredicate(format: "label CONTAINS[c] %@", "Select")
                ).firstMatch.exists,
            ])

        case "select_file":
            let (_, application) = try application(for: request)
            let names = request["files"] as? [String] ?? []
            guard !names.isEmpty else { throw AgentError("select_file requires at least one file name") }
            for name in names {
                let candidate = application.descendants(matching: .any).matching(
                    NSPredicate(format: "label == %@ OR identifier == %@ OR value == %@", name, name, name)
                ).firstMatch
                guard candidate.waitForExistence(timeout: seconds(request["timeout_ms"])) else {
                    throw AgentError("iOS document picker does not show `\(name)`")
                }
                candidate.tap()
            }
            let confirmation = application.buttons.matching(
                NSPredicate(format: "label IN[c] %@", ["Open", "Choose", "Done", "Add"])
            ).firstMatch
            if confirmation.exists && confirmation.isEnabled { confirmation.tap() }
            return .success(["selected": names])

        case "source":
            let (sessionID, application) = try application(for: request)
            let mode = request["value"] as? String ?? "default"
            guard ["default", "ai", "autoexpect", "codegen"].contains(mode) else {
                throw AgentError("invalid accessibility snapshot mode `\(mode)`")
            }
            let document = accessibilityDocument(sessionID: sessionID, application: application, mode: mode)
            let data = try JSONSerialization.data(withJSONObject: document)
            return .success(["snapshot": String(decoding: data, as: UTF8.self)])

        default:
            throw AgentError("unsupported iOS agent command `\(command)`")
        }
    }

    private func application(for request: [String: Any]) throws -> (String, XCUIApplication) {
        let sessionID = try requiredString("session_id", in: request)
        guard let application = applications[sessionID] else {
            throw AgentError("unknown or closed iOS app session `\(sessionID)`")
        }
        return (sessionID, application)
    }

    private func resolvedElement(
        for request: [String: Any],
        requiringActionability: Bool = false
    ) throws -> XCUIElement {
        let (sessionID, application) = try application(for: request)
        let selector = try requiredString("selector", in: request)
        let element = selector.hasPrefix("ref=")
            ? try referencedElement(selector, sessionID: sessionID, application: application)
            : query(selector, in: application).firstMatch
        let timeout = seconds(request["timeout_ms"])
        if requiringActionability {
            let expectation = XCTNSPredicateExpectation(
                predicate: NSPredicate(format: "exists == true AND hittable == true AND enabled == true"),
                object: element
            )
            guard XCTWaiter.wait(for: [expectation], timeout: timeout) == .completed else {
                throw AgentError("iOS element `\(selector)` did not become actionable before timeout")
            }
        } else if !element.waitForExistence(timeout: timeout) {
            throw AgentError("no iOS element matched `\(selector)` before timeout")
        }
        return element
    }

    private func referencedElement(
        _ selector: String,
        sessionID: String,
        application: XCUIApplication
    ) throws -> XCUIElement {
        guard !selector.contains(" >> ") else {
            throw AgentError("snapshot references must be standalone ref=<id> selectors")
        }
        let referenceID = String(selector.dropFirst("ref=".count)).trimmingCharacters(in: CharacterSet(charactersIn: "\""))
        guard let reference = snapshotReferences[sessionID]?[referenceID] else {
            let available = snapshotReferences[sessionID]?.keys.sorted().joined(separator: ",") ?? "none"
            throw AgentError("stale or foreign iOS snapshot reference `\(referenceID)` (available: \(available)); capture a new AI accessibility snapshot")
        }
        guard debugHierarchySignature(application.debugDescription) == reference.hierarchySignature else {
            throw AgentError("stale iOS snapshot reference: accessibility hierarchy changed; capture a new AI snapshot")
        }
        var query = application.descendants(matching: reference.type)
        if !reference.identifier.isEmpty {
            query = query.matching(identifier: reference.identifier)
        } else {
            query = query.matching(NSPredicate(
                format: "label == %@ OR title == %@ OR value == %@ OR placeholderValue == %@",
                reference.label, reference.label, reference.label, reference.label
            ))
        }
        let element = query.element(boundBy: reference.matchIndex)
        guard element.exists else {
            throw AgentError("stale iOS snapshot reference: referenced element disappeared")
        }
        return element
    }

    private func accessibilityDocument(
        sessionID: String,
        application: XCUIApplication,
        mode: String
    ) -> [String: Any] {
        let hierarchy = application.debugDescription
        let hierarchySignature = debugHierarchySignature(hierarchy)
        let generation = (snapshotGenerations[sessionID] ?? 0) + 1
        snapshotGenerations[sessionID] = generation
        var references: [String: SnapshotReference] = [:]
        var children: [[String: Any]] = []
        var matchIndexes: [String: Int] = [:]

        for (index, rawLine) in hierarchy.split(separator: "\n").enumerated() {
            let line = String(rawLine).trimmingCharacters(in: .whitespacesAndNewlines)
                .trimmingCharacters(in: CharacterSet(charactersIn: "→"))
            guard let type = debugElementType(line), type != .application else { continue }
            let identifier = debugQuotedValue("identifier", in: line) ?? ""
            let label = debugQuotedValue("label", in: line)
                ?? debugQuotedValue("title", in: line)
                ?? debugQuotedValue("value", in: line)
                ?? debugQuotedValue("placeholderValue", in: line)
                ?? ""
            let frame = debugFrame(line)
            let rendered = frame.width > 0 && frame.height > 0
            if mode == "autoexpect" && !rendered { continue }
            var properties: [String: Any] = [
                "type": elementTypeName(type),
                "identifier": identifier,
                "frame": ["x": frame.origin.x, "y": frame.origin.y, "width": frame.width, "height": frame.height],
                "rendered": rendered,
                "index": index,
            ]
            if let placeholder = debugQuotedValue("placeholderValue", in: line), !placeholder.isEmpty {
                properties["placeholder"] = placeholder
            }
            let disabled = line.contains("Disabled") || line.contains("enabled: 0")
            let selected = line.contains("Selected") || line.contains("selected: 1")
            if mode == "ai" && rendered && !disabled && actionable(type) && (!identifier.isEmpty || !label.isEmpty) {
                let referenceID = "e\(generation)-\(index + 1)"
                properties["aria-ref"] = referenceID
                let matchKey = "\(type.rawValue)|\(identifier)|\(label)"
                let matchIndex = matchIndexes[matchKey, default: 0]
                matchIndexes[matchKey] = matchIndex + 1
                references[referenceID] = SnapshotReference(
                    type: type,
                    identifier: identifier,
                    label: label,
                    matchIndex: matchIndex,
                    hierarchySignature: hierarchySignature
                )
            }
            children.append([
                "role": accessibilityRole(type),
                "name": type == .secureTextField ? (debugQuotedValue("placeholderValue", in: line) ?? label) : label,
                "states": ["disabled": disabled, "selected": selected],
                "properties": properties,
                "children": [],
            ])
        }
        snapshotReferences[sessionID] = mode == "ai" ? references : [:]
        var document: [String: Any] = [
            "contextId": sessionID,
            "parentContextId": NSNull(),
            "platform": "ios",
            "root": [
                "role": "application",
                "name": application.label,
                "states": [:],
                "properties": ["bundleIdentifier": applicationBundleIDs[sessionID] ?? ""],
                "children": children,
            ],
        ]
        if mode == "ai" { document["snapshotId"] = generation }
        return ["version": 1, "documents": [document]]
    }

    private func debugQuotedValue(_ key: String, in line: String) -> String? {
        for quote in ["'", "\""] {
            let marker = "\(key): \(quote)"
            guard let start = line.range(of: marker) else { continue }
            let tail = line[start.upperBound...]
            guard let end = tail.firstIndex(of: Character(quote)) else { continue }
            return String(tail[..<end])
        }
        return nil
    }

    private func debugHierarchySignature(_ hierarchy: String) -> String {
        hierarchy.split(separator: "\n").compactMap { rawLine -> String? in
            let line = String(rawLine).trimmingCharacters(in: .whitespacesAndNewlines)
                .trimmingCharacters(in: CharacterSet(charactersIn: "→"))
            guard let type = debugElementType(line) else { return nil }
            let identifier = debugQuotedValue("identifier", in: line) ?? ""
            let label = debugQuotedValue("label", in: line) ?? ""
            let value = debugQuotedValue("value", in: line) ?? ""
            let placeholder = debugQuotedValue("placeholderValue", in: line) ?? ""
            let frame = debugFrame(line)
            return "\(type.rawValue)|\(identifier)|\(label)|\(value)|\(placeholder)|\(frame.origin.x),\(frame.origin.y),\(frame.width),\(frame.height)"
        }.joined(separator: "\u{1f}")
    }

    private func debugFrame(_ line: String) -> CGRect {
        let pattern = #"\{\{(-?[0-9.]+),\s*(-?[0-9.]+)\},\s*\{([0-9.]+),\s*([0-9.]+)\}\}"#
        guard let expression = try? NSRegularExpression(pattern: pattern),
              let match = expression.firstMatch(in: line, range: NSRange(line.startIndex..., in: line)),
              match.numberOfRanges == 5 else { return .zero }
        let values = (1..<5).compactMap { offset -> CGFloat? in
            guard let range = Range(match.range(at: offset), in: line),
                  let number = Double(line[range]) else { return nil }
            return CGFloat(number)
        }
        guard values.count == 4 else { return .zero }
        return CGRect(x: values[0], y: values[1], width: values[2], height: values[3])
    }

    private func debugElementType(_ line: String) -> XCUIElement.ElementType? {
        let name = line.split(separator: ",", maxSplits: 1).first.map(String.init) ?? ""
        switch name {
        case "Application": return .application
        case "Window": return .window
        case "Other": return .other
        case "NavigationBar": return .navigationBar
        case "Button": return .button
        case "StaticText": return .staticText
        case "TextField": return .textField
        case "SecureTextField": return .secureTextField
        case "TextView": return .textView
        case "Image": return .image
        case "Cell": return .cell
        case "CheckBox": return .checkBox
        case "Switch": return .switch
        case "RadioButton": return .radioButton
        case "Slider": return .slider
        case "ProgressIndicator": return .progressIndicator
        case "Picker": return .picker
        case "PickerWheel": return .pickerWheel
        case "Table": return .table
        case "CollectionView": return .collectionView
        case "ScrollView": return .scrollView
        case "Link": return .link
        case "WebView": return .webView
        case "MenuItem": return .menuItem
        default: return nil
        }
    }

    private func actionable(_ type: XCUIElement.ElementType) -> Bool {
        [.button, .cell, .checkBox, .link, .menuItem, .radioButton, .secureTextField, .slider, .switch, .textField].contains(type)
    }

    private func accessibilityRole(_ type: XCUIElement.ElementType) -> String {
        switch type {
        case .button: return "button"
        case .textField, .secureTextField, .textView: return "textbox"
        case .checkBox: return "checkbox"
        case .switch: return "switch"
        case .radioButton: return "radio"
        case .slider: return "slider"
        case .progressIndicator: return "progressbar"
        case .picker, .pickerWheel: return "combobox"
        case .table, .collectionView: return "list"
        case .cell: return "listitem"
        case .image: return "img"
        case .link: return "link"
        case .staticText: return "text"
        case .webView: return "document"
        default: return "generic"
        }
    }

    private func elementTypeName(_ type: XCUIElement.ElementType) -> String {
        String(describing: type)
    }

    private func fullPageScreenshot(_ application: XCUIApplication) -> Data {
        let scrollable = [application.scrollViews.firstMatch, application.tables.firstMatch, application.collectionViews.firstMatch]
            .first(where: { $0.exists && $0.isHittable })
        guard let scrollable else { return application.screenshot().pngRepresentation }
        var shots: [Data] = []
        var previous: Data?
        for _ in 0..<20 {
            let shot = application.screenshot().pngRepresentation
            if shot == previous { break }
            shots.append(shot)
            previous = shot
            scrollable.swipeUp()
        }
        for _ in 1..<shots.count { scrollable.swipeDown() }
        let images = shots.compactMap(UIImage.init(data:))
        guard images.count > 1, let first = images.first else { return shots.first ?? application.screenshot().pngRepresentation }
        let size = CGSize(width: first.size.width, height: images.reduce(0) { $0 + $1.size.height })
        return UIGraphicsImageRenderer(size: size).pngData { context in
            var y: CGFloat = 0
            for image in images {
                image.draw(at: CGPoint(x: 0, y: y))
                y += image.size.height
            }
        }
    }

    private func query(_ selector: String, in application: XCUIApplication) -> XCUIElementQuery {
        let segments = Selector.parseAll(selector)
        var candidates = application.descendants(matching: .any)
        for (index, segment) in segments.enumerated() {
            if index > 0 {
                candidates = candidates.descendants(matching: .any)
            }
            candidates = segment.matching(candidates)
        }
        return candidates
    }

    private func keyText(_ key: String, explicitText: String?) -> String {
        if let explicitText { return explicitText }
        switch key.lowercased() {
        case "enter", "return": return XCUIKeyboardKey.return.rawValue
        case "tab": return XCUIKeyboardKey.tab.rawValue
        case "escape": return XCUIKeyboardKey.escape.rawValue
        case "backspace", "delete": return XCUIKeyboardKey.delete.rawValue
        case "space": return XCUIKeyboardKey.space.rawValue
        case "uparrow": return XCUIKeyboardKey.upArrow.rawValue
        case "downarrow": return XCUIKeyboardKey.downArrow.rawValue
        case "leftarrow": return XCUIKeyboardKey.leftArrow.rawValue
        case "rightarrow": return XCUIKeyboardKey.rightArrow.rawValue
        default: return key
        }
    }

    private func requiredString(_ key: String, in object: [String: Any], allowEmpty: Bool = false) throws -> String {
        guard let value = object[key] as? String, allowEmpty || !value.isEmpty else {
            throw AgentError("request requires `\(key)`")
        }
        return value
    }

    private func seconds(_ milliseconds: Any?) -> TimeInterval {
        let value = (milliseconds as? NSNumber)?.doubleValue ?? 10_000
        return max(value / 1_000, 0.001)
    }

    private func onMain<T>(_ work: @escaping () throws -> T) throws -> T {
        if Thread.isMainThread { return try work() }
        return try DispatchQueue.main.sync(execute: work)
    }
}

private enum Selector {
    case identifier(String)
    case label(String)
    case type(XCUIElement.ElementType)
    case predicate(NSPredicate)

    func matching(_ query: XCUIElementQuery) -> XCUIElementQuery {
        switch self {
        case .identifier(let value):
            return query.matching(identifier: value)
        case .label(let value):
            return query.matching(NSPredicate(format: "label == %@ OR title == %@ OR value == %@", value, value, value))
        case .type(let value):
            return query.matching(NSPredicate(format: "elementType == %d", value.rawValue))
        case .predicate(let predicate):
            return query.matching(predicate)
        }
    }

    static func parseAll(_ transport: String) -> [Selector] {
        var segments: [Selector] = []
        var index = transport.startIndex
        while index < transport.endIndex {
            while index < transport.endIndex, transport[index].isWhitespace {
                index = transport.index(after: index)
            }
            guard index < transport.endIndex,
                  let equals = transport[index...].firstIndex(of: "=") else { break }
            let prefix = String(transport[index..<equals])
            let bodyStart = transport.index(after: equals)
            guard bodyStart < transport.endIndex, transport[bodyStart] == "\"" else { break }
            var cursor = transport.index(after: bodyStart)
            var escaped = false
            var bodyEnd: String.Index?
            while cursor < transport.endIndex {
                let character = transport[cursor]
                if escaped {
                    escaped = false
                } else if character == "\\" {
                    escaped = true
                } else if character == "\"" {
                    bodyEnd = transport.index(after: cursor)
                    break
                }
                cursor = transport.index(after: cursor)
            }
            guard let bodyEnd else { break }
            segments.append(parse("\(prefix)=\(transport[bodyStart..<bodyEnd])"))
            index = bodyEnd
        }
        return segments.isEmpty ? [parse(transport)] : segments
    }

    static func parse(_ transport: String) -> Selector {
        let (prefix, encoded) = splitTransport(transport)
        let value = decodeJSONString(encoded)

        if prefix == "css" {
            if value.hasPrefix("#") { return .identifier(String(value.dropFirst())) }
            if let label = cssAttribute("aria-label", in: value) { return .label(label) }
            if let identifier = cssAttribute("data-testid", in: value) { return .identifier(identifier) }
            switch value.lowercased() {
            case "button": return .type(.button)
            case "a": return .type(.link)
            case "img": return .type(.image)
            case "input", "textarea": return .predicate(NSCompoundPredicate(orPredicateWithSubpredicates: [
                NSPredicate(format: "elementType == %d", XCUIElement.ElementType.textField.rawValue),
                NSPredicate(format: "elementType == %d", XCUIElement.ElementType.secureTextField.rawValue),
                NSPredicate(format: "elementType == %d", XCUIElement.ElementType.textView.rawValue),
            ]))
            default: return .identifier(value)
            }
        }
        if prefix == "uia", let separator = value.firstIndex(of: "=") {
            let key = value[..<separator].lowercased()
            let body = String(value[value.index(after: separator)...])
            switch key {
            case "text": return .label(body)
            case "textcontains": return .predicate(NSPredicate(format: "label CONTAINS %@ OR value CONTAINS %@", body, body))
            case "description", "desc", "resourceid": return .identifier(body)
            case "classname": return .type(elementType(body))
            case "enabled": return .predicate(NSPredicate(format: "enabled == %@", body.lowercased() == "true" ? NSNumber(value: true) : NSNumber(value: false)))
            default: return .label(body)
            }
        }
        if prefix == "aw", let data = value.data(using: .utf8),
           let spec = try? JSONSerialization.jsonObject(with: data) as? [String: Any] {
            return semantic(spec)
        }
        if prefix == "xpath" {
            if let name = attribute("name", in: value) { return .identifier(name) }
            if let label = attribute("label", in: value) { return .label(label) }
            if let type = value.split(separator: "/").last?.split(separator: "[").first {
                return .type(elementType(String(type)))
            }
        }
        return .identifier(value)
    }

    private static func semantic(_ spec: [String: Any]) -> Selector {
        let kind = spec["kind"] as? String ?? ""
        let exact = spec["exact"] as? Bool ?? false
        switch kind {
        case "role":
            var predicates: [NSPredicate] = []
            let role = (spec["role"] as? String ?? "").lowercased()
            predicates.append(rolePredicate(role))
            if let name = spec["name"] {
                predicates.append(textPredicate(name, exact: exact, fields: ["label", "title", "value", "placeholderValue"]))
            }
            if let disabled = spec["disabled"] as? Bool {
                predicates.append(NSPredicate(format: "enabled == %@", NSNumber(value: !disabled)))
            }
            if let selected = spec["selected"] as? Bool {
                predicates.append(NSPredicate(format: "selected == %@", NSNumber(value: selected)))
            }
            if let checked = spec["checked"] as? Bool {
                predicates.append(NSPredicate(format: "value == %@ OR selected == %@", checked ? "1" : "0", NSNumber(value: checked)))
            }
            return .predicate(NSCompoundPredicate(andPredicateWithSubpredicates: predicates))
        case "text":
            return .predicate(textPredicate(spec["text"], exact: exact, fields: ["label", "title", "value", "placeholderValue"]))
        case "label":
            return .predicate(textPredicate(spec["text"], exact: exact, fields: ["label", "title", "placeholderValue"]))
        case "testId":
            return .predicate(textPredicate(spec["text"], exact: true, fields: ["identifier"]))
        default:
            return .predicate(NSPredicate(value: false))
        }
    }

    private static func textPredicate(_ raw: Any?, exact: Bool, fields: [String]) -> NSPredicate {
        if let matcher = raw as? [String: Any], let pattern = matcher["regex"] as? String {
            let flags = matcher["flags"] as? String ?? ""
            let inlineFlags = String(flags.filter { "ims".contains($0) })
            let resolvedPattern = inlineFlags.isEmpty ? pattern : "(?\(inlineFlags))\(pattern)"
            return NSCompoundPredicate(orPredicateWithSubpredicates: fields.map {
                NSPredicate(format: "\($0) MATCHES %@", resolvedPattern)
            })
        }
        let text = raw as? String ?? ""
        let operatorName = exact ? "==" : "CONTAINS[c]"
        return NSCompoundPredicate(orPredicateWithSubpredicates: fields.map {
            NSPredicate(format: "\($0) \(operatorName) %@", text)
        })
    }

    private static func rolePredicate(_ role: String) -> NSPredicate {
        let types: [XCUIElement.ElementType]
        switch role {
        case "button": types = [.button]
        case "textbox": types = [.textField, .secureTextField]
        case "checkbox", "switch": types = [.switch]
        case "radio": types = [.radioButton]
        case "slider": types = [.slider]
        case "progressbar": types = [.progressIndicator]
        case "combobox": types = [.picker, .pickerWheel]
        case "list": types = [.table, .collectionView]
        case "listitem": types = [.cell]
        case "img", "image": types = [.image]
        case "link": types = [.link]
        case "text", "heading": types = [.staticText]
        case "generic": types = [.any]
        default: return NSPredicate(value: false)
        }
        if types == [.any] { return NSPredicate(value: true) }
        return NSCompoundPredicate(orPredicateWithSubpredicates: types.map {
            NSPredicate(format: "elementType == %d", $0.rawValue)
        })
    }

    private static func splitTransport(_ value: String) -> (String, String) {
        guard let separator = value.firstIndex(of: "=") else { return ("css", value) }
        return (String(value[..<separator]).lowercased(), String(value[value.index(after: separator)...]))
    }

    private static func decodeJSONString(_ value: String) -> String {
        guard value.hasPrefix("\"") else { return value }
        return (try? JSONDecoder().decode(String.self, from: Data(value.utf8))) ?? value
    }

    private static func attribute(_ name: String, in xpath: String) -> String? {
        for quote in ["'", "\""] {
            let marker = "@\(name)=\(quote)"
            guard let start = xpath.range(of: marker) else { continue }
            let tail = xpath[start.upperBound...]
            guard let end = tail.firstIndex(of: Character(quote)) else { continue }
            return String(tail[..<end])
        }
        return nil
    }

    private static func cssAttribute(_ name: String, in selector: String) -> String? {
        for quote in ["'", "\""] {
            let marker = "[\(name)=\(quote)"
            guard let start = selector.range(of: marker) else { continue }
            let tail = selector[start.upperBound...]
            guard let end = tail.firstIndex(of: Character(quote)) else { continue }
            return String(tail[..<end])
        }
        return nil
    }

    private static func elementType(_ raw: String) -> XCUIElement.ElementType {
        switch raw.lowercased().replacingOccurrences(of: "xcuielementtype", with: "") {
        case "button": return .button
        case "textfield", "input": return .textField
        case "securetextfield": return .secureTextField
        case "statictext", "text": return .staticText
        case "image": return .image
        case "cell": return .cell
        case "switch": return .switch
        case "link": return .link
        case "webview": return .webView
        case "scrollview": return .scrollView
        default: return .any
        }
    }
}

private struct HTTPRequest {
    let method: String
    let path: String
    let body: Data

    static func parse(_ data: Data) -> HTTPRequest? {
        let delimiter = Data("\r\n\r\n".utf8)
        guard let headerRange = data.range(of: delimiter),
              let header = String(data: data[..<headerRange.lowerBound], encoding: .utf8) else { return nil }
        let lines = header.components(separatedBy: "\r\n")
        let requestLine = lines.first?.split(separator: " ") ?? []
        guard requestLine.count >= 2 else { return nil }
        let contentLength = lines.dropFirst().compactMap { line -> Int? in
            let parts = line.split(separator: ":", maxSplits: 1)
            guard parts.count == 2, parts[0].lowercased() == "content-length" else { return nil }
            return Int(parts[1].trimmingCharacters(in: .whitespaces))
        }.first ?? 0
        let bodyStart = headerRange.upperBound
        guard data.count >= bodyStart + contentLength else { return nil }
        return HTTPRequest(
            method: String(requestLine[0]),
            path: String(requestLine[1]),
            body: data.subdata(in: bodyStart..<(bodyStart + contentLength))
        )
    }
}

private struct SnapshotReference {
    let type: XCUIElement.ElementType
    let identifier: String
    let label: String
    let matchIndex: Int
    let hierarchySignature: String
}

private struct AgentResponse {
    let object: [String: Any]

    static func success(_ result: [String: Any]) -> AgentResponse {
        AgentResponse(object: ["ok": true, "result": result])
    }

    static func failure(_ message: String) -> AgentResponse {
        AgentResponse(object: ["ok": false, "result": [:], "error": message])
    }
}

private struct AgentError: LocalizedError {
    let errorDescription: String?
    init(_ message: String) { errorDescription = message }
}
