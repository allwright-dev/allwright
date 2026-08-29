import XCTest

final class AllwrightAgentUITests: XCTestCase {

    func testAgent() throws {
        let app = XCUIApplication()
        app.launch()

        let agent = AllwrightIOSAgent(
            application: app
        )

        try agent.start()

        agent.waitForever()
    }
}
