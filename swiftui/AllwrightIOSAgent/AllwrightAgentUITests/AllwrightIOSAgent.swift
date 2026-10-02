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
            let identifier = environment["SIMULATOR_UDID"]
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
            return .success(["session_id": sessionID, "bundle_id": bundleID])

        case "close":
            let (sessionID, application) = try application(for: request)
            application.terminate()
            applications.removeValue(forKey: sessionID)
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
            let (_, application) = try application(for: request)
            let selector = try requiredString("selector", in: request)
            return .success(["count": query(selector, in: application).count])

        case "text":
            let element = try resolvedElement(for: request)
            let text = (element.value as? String).flatMap { $0.isEmpty ? nil : $0 }
                ?? (!element.label.isEmpty ? element.label : element.title)
            return .success(["text": text])

        case "wait":
            let (_, application) = try application(for: request)
            let selector = try requiredString("selector", in: request)
            let visible = request["visible"] as? Bool ?? true
            let timeout = seconds(request["timeout_ms"])
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
            return .success(["png_base64": application.screenshot().pngRepresentation.base64EncodedString()])

        case "source":
            let (sessionID, application) = try application(for: request)
            let document: [String: Any] = [
                "version": 1,
                "documents": [[
                    "id": sessionID,
                    "platform": "ios",
                    "application": [
                        "role": "application",
                        "name": application.label,
                        "states": [:],
                        "properties": ["debug_description": application.debugDescription],
                        "children": [],
                    ],
                ]],
            ]
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
        let (_, application) = try application(for: request)
        let selector = try requiredString("selector", in: request)
        let element = query(selector, in: application).firstMatch
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

    private func query(_ selector: String, in application: XCUIApplication) -> XCUIElementQuery {
        let parsed = Selector.parse(selector)
        let all = application.descendants(matching: .any)
        switch parsed {
        case .identifier(let value):
            return all.matching(identifier: value)
        case .label(let value):
            return all.matching(NSPredicate(format: "label == %@ OR title == %@ OR value == %@", value, value, value))
        case .type(let value):
            return all.matching(NSPredicate(format: "elementType == %d", value.rawValue))
        case .predicate(let predicate):
            return all.matching(predicate)
        }
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

    static func parse(_ transport: String) -> Selector {
        let (prefix, encoded) = splitTransport(transport)
        let value = decodeJSONString(encoded)

        if prefix == "css" {
            let identifier = value.hasPrefix("#") ? String(value.dropFirst()) : value
            return .identifier(identifier)
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
        if prefix == "xpath" {
            if let name = attribute("name", in: value) { return .identifier(name) }
            if let label = attribute("label", in: value) { return .label(label) }
            if let type = value.split(separator: "/").last?.split(separator: "[").first {
                return .type(elementType(String(type)))
            }
        }
        return .identifier(value)
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
