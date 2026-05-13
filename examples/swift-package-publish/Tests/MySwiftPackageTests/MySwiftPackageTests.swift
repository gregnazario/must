import XCTest
@testable import MySwiftPackage

final class MySwiftPackageTests: XCTestCase {
    func testAdd() {
        XCTAssertEqual(Calculator.add(2, 3), 5)
    }

    func testGreet() {
        XCTAssertEqual(Calculator.greet("world"), "Hello, world!")
    }
}
