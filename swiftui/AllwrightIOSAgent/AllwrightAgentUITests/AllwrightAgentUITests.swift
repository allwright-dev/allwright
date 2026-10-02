import XCTest

final class AllwrightAgentUITests: XCTestCase {

    func testAgent() throws {
        let agent = AllwrightIOSAgent()

        try agent.start()

        agent.waitForever()
    }
}
