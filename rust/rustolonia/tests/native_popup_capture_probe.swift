import AppKit
import CoreGraphics
import ScreenCaptureKit

@main
struct Probe {
    @MainActor
    static func main() async {
        guard CGPreflightScreenCaptureAccess() else {
            print("BLOCKED: screen-recording permission is not granted to this probe process; no permission prompt requested")
            exit(2)
        }
        let app = NSApplication.shared
        app.setActivationPolicy(.accessory)
        let window = NSWindow(contentRect: NSRect(x: 80, y: 80, width: 200, height: 120),
                              styleMask: .borderless, backing: .buffered, defer: false)
        window.title = "Rustolonia popup capture feasibility"
        window.backgroundColor = .red
        window.ignoresMouseEvents = true
        window.alphaValue = 0
        do {
            let before = try await SCShareableContent.excludingDesktopWindows(false, onScreenWindowsOnly: false)
            print("hidden windowNumber=\(window.windowNumber) discoverable=\(before.windows.contains { $0.windowID == CGWindowID(window.windowNumber) })")
            guard before.windows.contains(where: { $0.windowID == CGWindowID(window.windowNumber) }) else {
                print("FAILED: hidden native popup cannot be resolved as an SCWindow")
                exit(1)
            }
            window.orderFrontRegardless()
            let cloaked = try await SCShareableContent.excludingDesktopWindows(false, onScreenWindowsOnly: false)
            print("alpha-zero shown windowNumber=\(window.windowNumber) discoverable=\(cloaked.windows.contains { $0.windowID == CGWindowID(window.windowNumber) })")
            window.orderOut(nil)
        } catch {
            print("BLOCKED: ScreenCaptureKit enumeration failed: \(error)")
            exit(1)
        }
    }
}
