//
//  AllwrightIOSAgent.swift
//  AllwrightIOSAgent
//
//  Created by Atmaram N on 29/08/26.
//
import XCTest

final class AllwrightIOSAgent {

    private let application: XCUIApplication

    init(application: XCUIApplication) {
        self.application = application
    }

    func start() throws {
        print("🚀 Allwright iOS Agent starting...")
        print("📱 Bundle: \(application.description)")
        print("📡 Starting server...")
        print("✅ Allwright iOS Agent started")
    }

    func waitForever() {
        print("💓 Agent is alive")
        Timer.scheduledTimer(
            withTimeInterval: 5,
            repeats: true
        ) { _ in
            print("💓 Agent heartbeat")
        }
        RunLoop.current.run()
    }
}
