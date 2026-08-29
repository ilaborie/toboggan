//
//  TobogganAppUITests.swift
//  TobogganAppUITests
//

import XCTest

/// Interaction tests that need a live server.
///
/// Not run in CI — the `ios` job runs `-only-testing:TobogganAppTests`, because
/// these need `toboggan` serving a deck on port 8080. Run them locally with a
/// server up:
///
///     toboggan watch -p examples/riir-folder --port 8080
final class TobogganAppUITests: XCTestCase {
    override func setUpWithError() throws {
        continueAfterFailure = false
        // The server keeps the deck's position between runs, so without this a
        // test inherits wherever the previous one left off.
        resetDeck()
    }

    /// Sends the deck back to the first slide, straight to the server.
    ///
    /// The app deliberately has no "go to first" control — the overview covers
    /// it — so the reset goes over the REST API the same way any other client
    /// would drive the deck.
    private func resetDeck() {
        guard let url = URL(string: "http://127.0.0.1:8080/api/command") else { return }
        var request = URLRequest(url: url)
        request.httpMethod = "POST"
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        request.httpBody = Data(#"{"command":"First"}"#.utf8)
        let done = XCTestExpectation(description: "reset")
        URLSession.shared.dataTask(with: request) { _, _, _ in done.fulfill() }.resume()
        _ = XCTWaiter.wait(for: [done], timeout: 5)
    }

    /// Launches and waits for the deck, or skips.
    ///
    /// The toolbar button exists from launch and is only *enabled* once slides
    /// have arrived over the network, so waiting for existence alone taps a
    /// disabled button and silently does nothing.
    @MainActor
    private func launchWithDeck() throws -> XCUIApplication {
        let app = XCUIApplication()
        app.launch()
        let overview = app.buttons["Slides"]
        XCTAssertTrue(overview.waitForExistence(timeout: 15), "toolbar button never appeared")
        let loaded = expectation(
            for: NSPredicate(format: "isEnabled == true"),
            evaluatedWith: overview
        )
        guard XCTWaiter.wait(for: [loaded], timeout: 20) == .completed else {
            throw XCTSkip("No deck loaded — is a server serving a deck on port 8080?")
        }
        return app
    }

    /// A deck is longer than a phone screen, so the overview has to scroll.
    ///
    /// Asserted by a later slide becoming reachable, not by measuring a cell
    /// that scrolls out of the accessibility tree as soon as it leaves the
    /// screen.
    @MainActor
    func testDeckOverviewScrolls() throws {
        let app = try launchWithDeck()
        app.buttons["Slides"].tap()

        let firstCell = app.buttons.matching(
            NSPredicate(format: "label BEGINSWITH 'Slide 1: '")
        ).firstMatch
        XCTAssertTrue(firstCell.waitForExistence(timeout: 5), "the grid never appeared")

        XCTAssertTrue(firstCell.isHittable, "the first cell should start on screen")

        // The sheet's own scroll view, not the deck view behind it.
        let grid = app.scrollViews.element(boundBy: 1)
        grid.swipeUp()
        grid.swipeUp()

        XCTAssertFalse(
            firstCell.isHittable,
            "the overview grid did not scroll: the first cell is still on screen"
        )
        XCTAssertTrue(app.buttons["Done"].exists, "scrolling dismissed the sheet")
    }

    /// The overview opens where the talk is.
    ///
    /// Opening at slide one means hunting for the current slide mid-talk, which
    /// is the moment you have least attention to spare.
    @MainActor
    func testDeckOverviewOpensAtTheCurrentSlide() throws {
        let app = try launchWithDeck()

        // Walk far enough in that the current slide is off the first screen.
        let next = app.buttons["Next Slide"]
        XCTAssertTrue(next.waitForExistence(timeout: 5))
        for _ in 0..<14 {
            next.tap()
        }

        app.buttons["Slides"].tap()
        let firstCell = app.buttons.matching(
            NSPredicate(format: "label BEGINSWITH 'Slide 1: '")
        ).firstMatch
        XCTAssertTrue(firstCell.waitForExistence(timeout: 5), "the grid never appeared")

        XCTAssertFalse(
            firstCell.isHittable,
            "the overview opened at slide 1 instead of scrolling to the current slide"
        )
    }

    /// Long speaker notes have to be reachable: the deck view is the app's only
    /// scrolling surface and the floating control bar sits over its bottom edge.
    ///
    /// Skipped rather than failed when the current deck has no slide long enough
    /// to overflow — there is nothing to scroll, and asserting otherwise would
    /// make this a test of the deck rather than of the app.
    @MainActor
    func testDeckViewScrolls() throws {
        let app = try launchWithDeck()

        let deck = app.scrollViews.firstMatch
        XCTAssertTrue(deck.waitForExistence(timeout: 5), "no scroll view")

        let upNext = app.staticTexts["Up next"]
        XCTAssertTrue(upNext.waitForExistence(timeout: 5), "the deck view never rendered")

        // Walk forward looking for a slide whose content overflows.
        let next = app.buttons["Next Slide"]
        for _ in 0..<12 {
            let before = upNext.frame.origin.y
            deck.swipeUp()
            if upNext.exists, upNext.frame.origin.y != before {
                return  // it scrolled
            }
            guard next.exists, next.isEnabled else { break }
            next.tap()
        }
        throw XCTSkip("No slide in this deck overflows the screen, so nothing scrolls")
    }
}
