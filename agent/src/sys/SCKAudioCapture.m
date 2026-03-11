#import <Foundation/Foundation.h>
#import <ScreenCaptureKit/ScreenCaptureKit.h>
#import <CoreMedia/CoreMedia.h>

// Expose a C function pointer type for Rust to provide
typedef void (*AudioCallback)(const uint8_t *data, size_t length);

@interface AudioCapture : NSObject <SCStreamOutput>
@property (nonatomic, assign) AudioCallback callback;
@end

@implementation AudioCapture
- (void)stream:(SCStream *)stream didOutputSampleBuffer:(CMSampleBufferRef)sampleBuffer ofType:(SCStreamOutputType)type {
    if (type != SCStreamOutputTypeAudio) return;
    
    CMBlockBufferRef blockBuffer = CMSampleBufferGetDataBuffer(sampleBuffer);
    if (!blockBuffer) return;
    
    size_t length = 0;
    char *ptr = NULL;
    
    OSStatus status = CMBlockBufferGetDataPointer(blockBuffer, 0, NULL, &length, &ptr);
    if (status == kCMBlockBufferNoErr && ptr && self.callback) {
        // Send data directly to Rust callback!
        self.callback((const uint8_t *)ptr, length);
    }
}
@end

// Keep strong references so SCK objects aren't deallocated!
static SCStream *globalStream = nil;
static AudioCapture *globalCapture = nil;

// Expose C API to Rust
void start_sck_capture(AudioCallback callback) {
    if (@available(macOS 12.3, *)) {
        [SCShareableContent getShareableContentExcludingDesktopWindows:NO onScreenWindowsOnly:YES completionHandler:^(SCShareableContent *content, NSError *error) {
            
            if (error || !content.displays.firstObject) {
                fprintf(stderr, "SCK Error: %s\n", error ? error.localizedDescription.UTF8String : "No displays");
                return;
            }
            
            SCDisplay *display = content.displays.firstObject;
            SCStreamConfiguration *config = [[SCStreamConfiguration alloc] init];
            config.capturesAudio = YES;
            config.excludesCurrentProcessAudio = YES;
            config.sampleRate = 44100;
            config.channelCount = 1;
            
            SCContentFilter *filter = [[SCContentFilter alloc] initWithDisplay:display excludingApplications:@[] exceptingWindows:@[]];
            
            globalCapture = [[AudioCapture alloc] init];
            globalCapture.callback = callback;
            
            globalStream = [[SCStream alloc] initWithFilter:filter configuration:config delegate:nil];
            
            NSError *addErr = nil;
            [globalStream addStreamOutput:globalCapture type:SCStreamOutputTypeAudio sampleHandlerQueue:dispatch_get_global_queue(DISPATCH_QUEUE_PRIORITY_DEFAULT, 0) error:&addErr];
            
            if (addErr) {
                fprintf(stderr, "SCK Add Output Error: %s\n", addErr.localizedDescription.UTF8String);
                return;
            }
            
            [globalStream startCaptureWithCompletionHandler:^(NSError *startErr) {
                if (startErr) {
                    fprintf(stderr, "SCK Start Error: %s\n", startErr.localizedDescription.UTF8String);
                } else {
                    fprintf(stderr, "--- Audio capture started natively via macOS ScreenCaptureKit ---\n");
                }
            }];
        }];
        
        // Start the run loop so this background thread can process SCK callbacks!
        [[NSRunLoop currentRunLoop] run];
    } else {
        fprintf(stderr, "ScreenCaptureKit requires macOS 12.3 or newer.\n");
    }
}
