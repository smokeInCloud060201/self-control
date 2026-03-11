import ScreenCaptureKit
import Foundation

// A simple Swift helper to capture system audio via ScreenCaptureKit 
// and write raw PCM data to stdout for the Rust Agent to consume.

class AudioCapture: NSObject, SCStreamOutput {
    func stream(_ stream: SCStream, didOutputSampleBuffer sampleBuffer: CMSampleBuffer, of type: SCStreamOutputType) {
        guard type == .audio else { return }
        guard let blockBuffer = CMSampleBufferGetDataBuffer(sampleBuffer) else { return }
        
        var length = 0
        var ptr: UnsafeMutablePointer<Int8>?
        
        CMBlockBufferGetDataPointer(blockBuffer, atOffset: 0, lengthAtOffsetOut: nil, totalLengthOut: &length, dataPointerOut: &ptr)
        
        if let dataPtr = ptr {
            let data = Data(bytes: dataPtr, count: length)
            FileHandle.standardOutput.write(data)
        }
    }
}

@main
struct App {
    static func main() async {
        do {
            let content = try await SCShareableContent.excludingDesktopWindows(false, onScreenWindowsOnly: true)
            
            // Capture all system audio
            let config = SCStreamConfiguration()
            config.capturesAudio = true
            config.excludesCurrentProcessAudio = true // Don't capture yourself
            
            // Find the main display for the filter
            guard let display = content.displays.first else {
                fputs("No displays found\n", stderr)
                exit(1)
            }
            
            let filter = SCContentFilter(display: display, excludingApplications: [], exceptingWindows: [])
            
            let capture = AudioCapture()
            let stream = SCStream(filter: filter, configuration: config, delegate: nil)
            
            try stream.addStreamOutput(capture, type: .audio, sampleHandlerQueue: .global())
            try await stream.startCapture()
            
            fputs("Audio capture started via ScreenCaptureKit\n", stderr)
            
            // Keep running
            while true {
                try await Task.sleep(nanoseconds: 1_000_000_000)
            }
        } catch {
            fputs("Error starting capture: \(error)\n", stderr)
            exit(1)
        }
    }
}
