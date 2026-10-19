import XCTest

final class WalletUITests: XCTestCase {
    @MainActor
    func testOnboardingAndLocalValidation() {
        let app = XCUIApplication()
        app.launch()
        XCTAssertTrue(app.navigationBars["Bark Wallet"].waitForExistence(timeout: 15))
        XCTAssertTrue(app.buttons["open-wallet"].exists)
        XCTAssertTrue(app.staticTexts["RegTest · local funds only"].exists)
    }
}
