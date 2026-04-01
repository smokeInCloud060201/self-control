import React, { useEffect, useRef, useState } from 'react';
import { AlertCircle, ChevronUp, ClipboardPaste, Keyboard, Loader2, LogOut, Maximize2, Monitor } from 'lucide-react';
import { cn } from '../../lib/utils';
import { VirtualKeyboard } from './VirtualKeyboard';

interface RdpViewerProps {
    sessionId: string;
    password?: string;
    proxyUrl: string;
    onDisconnect?: () => void;
}

const RdpViewer: React.FC<RdpViewerProps> = ({ sessionId, password, proxyUrl, onDisconnect }) => {
    const canvasRef = useRef<HTMLCanvasElement>(null);
    const containerRef = useRef<HTMLDivElement>(null);
    const [status, setStatus] = useState<'idle' | 'connecting' | 'connected' | 'error'>('idle');
    const [showToolbar, setShowToolbar] = useState(false);
    const toolbarTimerRef = useRef<number | null>(null);
    const [error, setError] = useState<string | null>(null);
    const signalingWsRef = useRef<WebSocket | null>(null);
    const rtcConnRef = useRef<RTCPeerConnection | null>(null);
    const inputDcRef = useRef<RTCDataChannel | null>(null);
    const [aspectRatio, setAspectRatio] = useState<number>(16 / 9);
    const [displays, setDisplays] = useState<any[]>([]);
    const [currentDisplay, setCurrentDisplay] = useState(0);
    const [showVirtualKeyboard, setShowVirtualKeyboard] = useState(false);
    const [isMobile, setIsMobile] = useState(false);
    const audioContextRef = useRef<AudioContext | null>(null);
    const nextAudioTimeRef = useRef<number>(0);
    const longPressTimerRef = useRef<number | null>(null);
    const lastTouchPosRef = useRef<{ x: number, y: number } | null>(null);
    const isScrollingRef = useRef<boolean>(false);

    const toggleFullscreen = () => {
        if (containerRef.current) {
            if (!document.fullscreenElement) {
                containerRef.current.requestFullscreen().catch(err => {
                    console.error(`Error attempting to enable full-screen mode: ${err.message}`);
                });
            } else {
                document.exitFullscreen();
            }
        }
    };

    useEffect(() => {
        // Simple mobile/tablet detection
        const userAgent = navigator.userAgent || navigator.vendor || (window as any).opera;
        const mobileRegex = /Android|webOS|iPhone|iPad|iPod|BlackBerry|IEMobile|Opera Mini/i;
        setIsMobile(mobileRegex.test(userAgent) || window.innerWidth < 1024);

        const handleResize = () => {
            setIsMobile(window.innerWidth < 1024);
        };
        window.addEventListener('resize', handleResize);
        return () => window.removeEventListener('resize', handleResize);
    }, []);

    useEffect(() => {
        if (!canvasRef.current) return;

        let isCleanup = false;

        if (!audioContextRef.current) {
            audioContextRef.current = new (window.AudioContext || (window as any).webkitAudioContext)({
                sampleRate: 44100,
            });
            nextAudioTimeRef.current = 0;
        }

        const startConnection = () => {
            if (signalingWsRef.current || rtcConnRef.current) return;

            setStatus('connecting');
            setError(null);

            const cleanProxyUrl = proxyUrl.endsWith('/') ? proxyUrl.slice(0, -1) : proxyUrl;
            const signalingWsUrl = `${cleanProxyUrl}/client/${sessionId}/${password || 'no-pass'}`;
            const proxyHost = new URL(cleanProxyUrl).hostname || 'localhost';

            console.log(`[DEBUG] Connecting to WebRTC Signaling Server at ${proxyHost}...`);

            try {
                const pc = new RTCPeerConnection({
                    iceServers: [
                        { urls: 'stun:stun.l.google.com:19302' },
                    ]
                });

                rtcConnRef.current = pc;

                const signalingWs = new WebSocket(signalingWsUrl);
                signalingWsRef.current = signalingWs;

                // 1. Create WebRTC DataChannels (Fully Reliable to allow seamless 256KB SCTP fragmentation)
                const videoDc = pc.createDataChannel('video_stream');
                videoDc.binaryType = 'arraybuffer';
                
                const inputDc = pc.createDataChannel('input_stream', { ordered: true });
                inputDcRef.current = inputDc;

                videoDc.onopen = () => console.log('[DEBUG] Video DataChannel Open');
                inputDc.onopen = () => {
                    console.log('[DEBUG] Input DataChannel Open');
                    setStatus('connected');
                    
                    // Send resolution
                    const width = window.screen.width * window.devicePixelRatio;
                    const height = window.screen.height * window.devicePixelRatio;
                    inputDc.send(JSON.stringify({
                        type: 'resolution_update',
                        width: Math.round(width),
                        height: Math.round(height)
                    }));
                };

                let frameChunks: ArrayBuffer[] = [];
                videoDc.onmessage = async (event) => {
                    if (isCleanup) return;
                    if (event.data instanceof ArrayBuffer) {
                        if (event.data.byteLength === 0) {
                            if (frameChunks.length === 0) return;
                            try {
                                const blob = new Blob(frameChunks, { type: 'image/jpeg' });
                                frameChunks = []; // reset for next frame
                                const bitmap = await createImageBitmap(blob);

                                const canvas = canvasRef.current;
                                if (canvas) {
                                    const ctx = canvas.getContext('2d');
                                    if (ctx) {
                                        if (canvas.width !== bitmap.width || canvas.height !== bitmap.height) {
                                            canvas.width = bitmap.width;
                                            canvas.height = bitmap.height;
                                            setAspectRatio(bitmap.width / bitmap.height);
                                        }
                                        ctx.drawImage(bitmap, 0, 0);
                                    }
                                }
                            } catch (e) {
                                console.error('[DEBUG] Frame decode error:', e);
                                frameChunks = []; // Drop corrupted frame
                            }
                        } else {
                            frameChunks.push(event.data);
                        }
                    }
                };

                // 2. Setup ICE Candidate routing
                pc.onicecandidate = (event) => {
                    if (event.candidate && inputDcRef.current?.readyState === 'open') {
                        signalingWs.send(JSON.stringify({
                            type: 'ice_candidate',
                            candidate: event.candidate
                        }));
                    }
                };

                // 3. Setup Signaling WS Routing
                signalingWs.onopen = async () => {
                    if (isCleanup) return;
                    console.log('[DEBUG] Signaling connected, creating WebRTC Offer...');
                    try {
                        const offer = await pc.createOffer();
                        await pc.setLocalDescription(offer);
                        signalingWs.send(JSON.stringify({
                            type: 'offer',
                            sdp: offer
                        }));
                    } catch (e) {
                        console.error('[DEBUG] Failed to create WebRTC offer', e);
                    }
                };

                signalingWs.onmessage = async (event) => {
                    if (isCleanup) return;
                    try {
                        const msg = JSON.parse(event.data);
                        if (msg.type === 'answer') {
                            await pc.setRemoteDescription(new RTCSessionDescription(msg.sdp));
                        } else if (msg.type === 'ice_candidate') {
                            await pc.addIceCandidate(new RTCIceCandidate(msg.candidate));
                        } else if (msg.type === 'metadata' && msg.displays) {
                            setDisplays(msg.displays);
                        } else if (msg.type === 'clipboard_sync' && msg.text) {
                            navigator.clipboard.writeText(msg.text).catch(err => console.error(err));
                        }
                    } catch (e) {}
                };

                const handleClose = (event: any) => {
                    if (isCleanup) return;
                    console.log('[DEBUG] Connection Closed');
                    setStatus('idle');
                    if (event?.code && event.code !== 1000 && event.code !== 1001) {
                        setError(`Session ended: ${event.reason || 'Network error'} (${event.code})`);
                        setStatus('error');
                    }
                    onDisconnect?.();
                };

                signalingWs.onerror = () => handleError(new Error("Signaling WS Error"));
                signalingWs.onclose = handleClose;
                pc.onconnectionstatechange = () => {
                    if (pc.connectionState === 'disconnected' || pc.connectionState === 'failed') {
                        handleClose({ code: 1006, reason: 'WebRTC Connection Failed' });
                    }
                };

            } catch (err) {
                setError('Failed to initialize WebRTC.');
                setStatus('error');
            }
        };

        const handleError = (e: any) => {
            if (isCleanup) return;
            console.error('[DEBUG] Connection Error:', e);
            setError('Connection failed. Please check credentials and proxy.');
            setStatus('error');
        };

        startConnection();

        return () => {
            isCleanup = true;
            if (signalingWsRef.current) {
                signalingWsRef.current.close(1000, 'Unmount');
                signalingWsRef.current = null;
            }
            if (rtcConnRef.current) {
                rtcConnRef.current.close();
                rtcConnRef.current = null;
            }
        };
    }, [proxyUrl, sessionId, password]);

    // Handle Input Events
    const startInteracting = () => {
        if (audioContextRef.current && audioContextRef.current.state === 'suspended') {
            audioContextRef.current.resume();
        }
    };

    const getScaledCoordinates = (clientX: number, clientY: number) => {
        if (!canvasRef.current) return null;
        const rect = canvasRef.current.getBoundingClientRect();
        const x = (clientX - rect.left) / rect.width;
        const y = (clientY - rect.top) / rect.height;
        return { x, y };
    };

    const handleMouseMove = (e: React.MouseEvent) => {
        startInteracting();
        
        // Auto-show toolbar when mouse is near top
        if (status === 'connected') {
            const rect = containerRef.current?.getBoundingClientRect();
            if (rect) {
                const relativeY = e.clientY - rect.top;
                // Show if in top 100px, hide immediately if below
                if (relativeY < 100) {
                    setShowToolbar(true);
                    if (toolbarTimerRef.current) {
                        window.clearTimeout(toolbarTimerRef.current);
                        toolbarTimerRef.current = null;
                    }
                } else {
                    // Slight delay to prevent flickering when transitioning out of toolbar
                    if (!toolbarTimerRef.current) {
                        toolbarTimerRef.current = window.setTimeout(() => {
                            setShowToolbar(false);
                            toolbarTimerRef.current = null;
                        }, 300); // Faster hide: 300ms instead of 2000ms
                    }
                }
            }
        }

        if (inputDcRef.current?.readyState === 'open') {
            const coords = getScaledCoordinates(e.clientX, e.clientY);
            if (coords) {
                inputDcRef.current.send(JSON.stringify({ type: 'MouseMove', ...coords }));
            }
        }
    };

    const handleTouchStart = (e: React.TouchEvent) => {
        startInteracting();
        if (inputDcRef.current?.readyState === 'open' && e.touches.length > 0) {
            const touch = e.touches[0];
            const coords = getScaledCoordinates(touch.clientX, touch.clientY);
            
            // Handle toolbar visibility on touch
            if (status === 'connected') {
                const rect = containerRef.current?.getBoundingClientRect();
                if (rect) {
                    const relativeY = touch.clientY - rect.top;
                    if (relativeY < 100) {
                        setShowToolbar(true);
                        if (toolbarTimerRef.current) {
                            window.clearTimeout(toolbarTimerRef.current);
                            toolbarTimerRef.current = null;
                        }
                    } else if (showToolbar && !toolbarTimerRef.current) {
                        // Auto-hide toolbar if touching content area while it's open
                        toolbarTimerRef.current = window.setTimeout(() => {
                            setShowToolbar(false);
                            toolbarTimerRef.current = null;
                        }, 300);
                    }
                }
            }

            if (coords) {
                // Always sync position first
                inputDcRef.current.send(JSON.stringify({ type: 'MouseMove', ...coords }));
                lastTouchPosRef.current = { x: touch.clientX, y: touch.clientY };

                if (e.touches.length === 1) {
                    isScrollingRef.current = false;
                    // Start long press timer for right click
                    longPressTimerRef.current = window.setTimeout(() => {
                        if (inputDcRef.current?.readyState === 'open') {
                            inputDcRef.current.send(JSON.stringify({ type: 'MouseDown', button: 'right' }));
                            // We don't send a matching MouseUp immediately to allow dragging if the agent supports it, 
                            // but usually context menu triggers on MouseDown or Click. 
                            // For simplicity, let's treat long press as a right-click "down".
                        }
                        longPressTimerRef.current = null;
                    }, 500);

                    inputDcRef.current.send(JSON.stringify({ type: 'MouseDown', button: 'left' }));
                } else if (e.touches.length === 2) {
                    isScrollingRef.current = true;
                    // Cancel left click if we transition to 2-finger scroll
                    if (longPressTimerRef.current) {
                        clearTimeout(longPressTimerRef.current);
                        longPressTimerRef.current = null;
                    }
                    inputDcRef.current.send(JSON.stringify({ type: 'MouseUp', button: 'left' }));
                }
            }
        }
    };

    const handleTouchMove = (e: React.TouchEvent) => {
        if (inputDcRef.current?.readyState === 'open' && e.touches.length > 0) {
            const touch = e.touches[0];
            const coords = getScaledCoordinates(touch.clientX, touch.clientY);
            
            if (coords) {
                if (e.touches.length === 1 && !isScrollingRef.current) {
                    // If we move significantly, cancel the long press
                    if (lastTouchPosRef.current) {
                        const dist = Math.hypot(touch.clientX - lastTouchPosRef.current.x, touch.clientY - lastTouchPosRef.current.y);
                        if (dist > 10 && longPressTimerRef.current) {
                            clearTimeout(longPressTimerRef.current);
                            longPressTimerRef.current = null;
                        }
                    }
                    inputDcRef.current.send(JSON.stringify({ type: 'MouseMove', ...coords }));
                } else if (e.touches.length === 2 && lastTouchPosRef.current) {
                    const deltaX = (touch.clientX - lastTouchPosRef.current.x);
                    const deltaY = (touch.clientY - lastTouchPosRef.current.y);
                    
                    // Send inverted scroll for more natural touch feel (swipe up to scroll down)
                    inputDcRef.current.send(JSON.stringify({ 
                        type: 'mouse_wheel', 
                        delta_x: Math.round(deltaX / 5), 
                        delta_y: Math.round(deltaY / 5) 
                    }));
                }
                
                lastTouchPosRef.current = { x: touch.clientX, y: touch.clientY };
                if (e.cancelable) e.preventDefault();
            }
        }
    };

    const handleTouchEnd = (e: React.TouchEvent) => {
        if (longPressTimerRef.current) {
            clearTimeout(longPressTimerRef.current);
            longPressTimerRef.current = null;
        }

        if (inputDcRef.current?.readyState === 'open') {
            if (!isScrollingRef.current) {
                // If we were in a right-click state, we might need a mouse up for that, 
                // but for now let's just ensure left-click is up.
                inputDcRef.current.send(JSON.stringify({ type: 'MouseUp', button: 'left' }));
                inputDcRef.current.send(JSON.stringify({ type: 'MouseUp', button: 'right' }));
            }
        }
        
        if (e.touches.length === 0) {
            isScrollingRef.current = false;
            lastTouchPosRef.current = null;
        }
    };

    const handleMouseDown = (button: 'left' | 'right') => {
        if (inputDcRef.current?.readyState === 'open') {
            inputDcRef.current.send(JSON.stringify({
                type: 'MouseDown',
                button
            }));
        }
    };

    const handleMouseUp = (button: 'left' | 'right') => {
        if (inputDcRef.current?.readyState === 'open') {
            inputDcRef.current.send(JSON.stringify({
                type: 'MouseUp',
                button
            }));
        }
    };

    const handleKeyDown = (e: React.KeyboardEvent) => {
        if (inputDcRef.current?.readyState === 'open') {
            // Intercept Shortcuts (Cmd+V on Mac, Ctrl+V otherwise)
            const isMod = e.metaKey || e.ctrlKey;
            if (isMod && e.key.toLowerCase() === 'v') {
                syncClipboard();
                return; // Don't send the V to the remote as it's handled by PasteText
            }

            inputDcRef.current.send(JSON.stringify({
                type: 'KeyDown',
                key: e.key
            }));
            // Prevent default for common shortcuts that might disrupt browser
            if (['Tab', 'Alt', 'Meta'].includes(e.key)) {
                e.preventDefault();
            }
        }
    };

    const handleKeyUp = (e: React.KeyboardEvent) => {
        if (inputDcRef.current?.readyState === 'open') {
            inputDcRef.current.send(JSON.stringify({
                type: 'KeyUp',
                key: e.key
            }));
        }
    };

    const switchDisplay = (index: number) => {
        if (inputDcRef.current?.readyState === 'open') {
            inputDcRef.current.send(JSON.stringify({ type: 'switch_display', index }));
            setCurrentDisplay(index);
        }
    };

    const syncClipboard = async () => {
        try {
            const text = await navigator.clipboard.readText();
            if (text && inputDcRef.current?.readyState === 'open') {
                inputDcRef.current.send(JSON.stringify({ type: 'paste_text', text }));
            }
        } catch (err) {
            console.error('Failed to read clipboard:', err);
        }
    };

    return (
        <div className="relative w-full h-full group/viewer">
            <div
                ref={containerRef}
                style={{ aspectRatio: `${aspectRatio}` }}
                className={cn(
                    "relative w-full bg-slate-950 rounded-2xl sm:rounded-[3rem] overflow-hidden border border-white/5 shadow-[0_0_100px_-20px_rgba(0,0,0,0.5)] transition-all outline-none",
                    status === 'connected' ? "ring-1 ring-white/10" : ""
                )}
                onMouseMove={handleMouseMove}
                onTouchStart={handleTouchStart}
                onTouchMove={handleTouchMove}
                onTouchEnd={handleTouchEnd}
                onMouseDown={(e) => {
                    const canvas = canvasRef.current;
                    if (canvas) canvas.focus();
                    handleMouseDown(e.button === 0 ? 'left' : 'right');
                }}
                onMouseUp={(e) => handleMouseUp(e.button === 0 ? 'left' : 'right')}
            >
                {/* Mobile Toolbar Toggle */}
                {isMobile && status === 'connected' && !showToolbar && (
                    <button
                        onClick={() => setShowToolbar(true)}
                        className="absolute top-4 left-4 z-[60] p-3 bg-slate-900/80 backdrop-blur-xl border border-white/10 rounded-2xl text-white shadow-2xl active:scale-95 transition-all animate-in fade-in zoom-in duration-300"
                        title="Show Toolbar"
                    >
                        <Monitor className="w-5 h-5 text-blue-400" />
                    </button>
                )}

                {status === 'connected' && (
                    <div className={cn(
                        "absolute top-4 sm:top-6 landscape:top-2 left-1/2 -translate-x-1/2 z-50 flex flex-col sm:flex-row items-center justify-between gap-3 sm:gap-4 px-2 sm:px-8 py-2 sm:py-4 landscape:py-1 bg-slate-900/80 backdrop-blur-2xl border border-white/10 rounded-xl sm:rounded-[2rem] shadow-[0_20px_50px_rgba(0,0,0,0.5)] transition-all duration-500 ease-out w-[95%] sm:w-auto overflow-hidden",
                        showToolbar ? "opacity-100 translate-y-0" : "opacity-0 -translate-y-12 pointer-events-none"
                    )}>
                        <div className="flex items-center gap-4 sm:gap-6 w-full sm:w-auto justify-center">
                            <div className="flex items-center gap-1.5 sm:gap-3 px-2 sm:px-4 py-1 sm:py-2 bg-blue-500/10 border border-blue-500/20 rounded-lg sm:rounded-2xl shrink-0">
                                <div className="relative">
                                    <div className="w-1.5 h-1.5 sm:w-2.5 sm:h-2.5 bg-green-500 rounded-full animate-ping absolute inset-0" />
                                    <div className="w-1.5 h-1.5 sm:w-2.5 sm:h-2.5 bg-green-500 rounded-full relative" />
                                </div>
                                <span className="text-[8px] sm:text-[10px] font-black text-blue-400 uppercase tracking-widest sm:tracking-[0.2em] whitespace-nowrap">Active</span>
                            </div>

                            {displays.length > 0 && (
                                <div className="flex items-center bg-slate-950/40 p-0.5 sm:p-1 rounded-xl sm:rounded-2xl border border-white/5 overflow-x-auto max-w-[150px] sm:max-w-none no-scrollbar">
                                    {displays.map((_, i) => (
                                        <button
                                            key={i}
                                            onClick={() => switchDisplay(i)}
                                            className={cn(
                                                "px-2 sm:px-4 py-1 sm:py-2 rounded-lg sm:rounded-xl text-[8px] sm:text-[10px] font-black uppercase tracking-widest transition-all gap-1.5 sm:gap-2 flex items-center min-w-[60px] sm:min-w-[100px] justify-center whitespace-nowrap",
                                                currentDisplay === i
                                                    ? "bg-white text-slate-950 shadow-xl scale-105"
                                                    : "text-slate-500 hover:text-white hover:bg-white/5"
                                            )}
                                        >
                                            <Monitor className="w-3 h-3 sm:w-3.5 sm:h-3.5" />
                                            {displays.length > 2 ? (i + 1) : `Display ${i + 1}`}
                                        </button>
                                    ))}
                                </div>
                            )}
                        </div>

                        <div className="flex items-center gap-2 sm:gap-3 shrink-0">
                            <button
                                onClick={syncClipboard}
                                className="flex items-center gap-1.5 sm:gap-2 px-2.5 sm:px-5 py-1.5 sm:py-2.5 bg-white/5 hover:bg-white/10 text-white rounded-lg sm:rounded-2xl border border-white/10 transition-all group active:scale-95 text-[8px] sm:text-[10px] font-black uppercase tracking-widest whitespace-nowrap"
                                title="Paste Local Clipboard to Remote"
                            >
                                <ClipboardPaste className="w-3 h-3 sm:w-4 sm:h-4 group-hover:text-blue-400 transition-colors" />
                                <span className="hidden sm:inline">Paste</span>
                            </button>

                            {isMobile && (
                                <button
                                    onClick={() => setShowVirtualKeyboard(!showVirtualKeyboard)}
                                    className={cn(
                                        "flex items-center gap-1.5 sm:gap-2 px-2.5 sm:px-5 py-1.5 sm:py-2.5 rounded-lg sm:rounded-2xl border transition-all group active:scale-95 text-[8px] sm:text-[10px] font-black uppercase tracking-widest whitespace-nowrap",
                                        showVirtualKeyboard 
                                            ? "bg-blue-600 border-blue-400 text-white" 
                                            : "bg-white/5 border-white/10 text-white hover:bg-white/10"
                                    )}
                                    title="Virtual Keyboard"
                                >
                                    <Keyboard className="w-3 h-3 sm:w-4 sm:h-4 group-hover:animate-pulse transition-colors" />
                                    <span className="hidden sm:inline">KB</span>
                                </button>
                            )}

                            <button
                                onClick={toggleFullscreen}
                                className="flex items-center gap-1.5 sm:gap-2 px-2.5 sm:px-5 py-1.5 sm:py-2.5 bg-white/5 hover:bg-white/10 text-white rounded-lg sm:rounded-2xl border border-white/10 transition-all group active:scale-95 text-[8px] sm:text-[10px] font-black uppercase tracking-widest whitespace-nowrap"
                                title="Fullscreen"
                            >
                                <Maximize2 className="w-3 h-3 sm:w-4 sm:h-4 group-hover:text-blue-400 transition-colors" />
                                <span className="hidden sm:inline">FS</span>
                            </button>

                            {isMobile && (
                                <button
                                    onClick={() => setShowToolbar(false)}
                                    className="flex items-center gap-1.5 sm:gap-2 px-2.5 sm:px-5 py-1.5 sm:py-2.5 bg-white/5 hover:bg-white/10 text-white rounded-lg sm:rounded-2xl border border-white/10 transition-all group active:scale-95 text-[8px] sm:text-[10px] font-black uppercase tracking-widest whitespace-nowrap"
                                    title="Hide Toolbar"
                                >
                                    <ChevronUp className="w-3 h-3 sm:w-4 sm:h-4 rotate-180 group-hover:text-blue-400 transition-colors" />
                                    <span className="hidden sm:inline">Hide</span>
                                </button>
                            )}

                            <button
                                onClick={() => onDisconnect?.()}
                                className="flex items-center gap-1.5 sm:gap-2 px-2.5 sm:px-5 py-1.5 sm:py-2.5 bg-red-500/10 hover:bg-red-500/20 text-red-500 rounded-lg sm:rounded-2xl border border-red-500/20 transition-all group active:scale-95 text-[8px] sm:text-[10px] font-black uppercase tracking-widest whitespace-nowrap"
                                title="Terminate Session"
                            >
                                <LogOut className="w-3 h-3 sm:w-4 sm:h-4 group-hover:text-red-400 transition-colors" />
                                <span className="hidden sm:inline">Exit</span>
                            </button>
                        </div>
                    </div>
                )}

                {status === 'connecting' && (
                    <div className="absolute inset-0 flex flex-col items-center justify-center bg-slate-950/90 backdrop-blur-2xl z-20">
                        <div className="relative flex items-center justify-center mb-8">
                            <div className="absolute w-32 h-32 bg-blue-500/20 rounded-full blur-3xl animate-pulse" />
                            <Loader2 className="w-16 h-16 text-blue-500 animate-spin relative" />
                        </div>
                        <h3 className="text-2xl font-bold text-white mb-2">Syncing with Node</h3>
                        <p className="text-slate-500 font-mono text-sm tracking-tighter">Establishing secure handshake with {sessionId.slice(0, 8)}...</p>
                    </div>
                )}

                {status === 'error' && (
                    <div className="absolute inset-0 flex flex-col items-center justify-center bg-slate-950/95 z-40 p-12 text-center animate-in fade-in duration-300">
                        <div className="w-24 h-24 bg-red-500/10 rounded-full flex items-center justify-center mb-8 border border-red-500/20">
                            <AlertCircle className="w-12 h-12 text-red-500" />
                        </div>
                        <h3 className="text-3xl font-black text-white mb-3 tracking-tight">Access Restricted</h3>
                        <p className="text-slate-500 max-w-sm mb-10 text-lg leading-relaxed">{error}</p>
                        <button
                            onClick={() => onDisconnect?.()}
                            className="px-10 py-4 bg-white text-slate-950 hover:bg-slate-200 rounded-2xl transition-all font-black text-sm uppercase tracking-widest shadow-2xl active:scale-95"
                        >
                            Re-initialize Session
                        </button>
                    </div>
                )}

                <canvas
                    ref={canvasRef}
                    className={cn(
                        "w-full h-full cursor-default object-contain transition-opacity duration-1000 outline-none",
                        status === 'connected' ? "opacity-100 shadow-[0_0_50px_rgba(0,0,0,0.5)]" : "opacity-0"
                    )}
                    id="remote-canvas"
                    tabIndex={0}
                    onKeyDown={handleKeyDown}
                    onKeyUp={handleKeyUp}
                    onContextMenu={(e) => e.preventDefault()}
                />

                {showVirtualKeyboard && isMobile && (
                    <VirtualKeyboard 
                        onKeyPress={(key, isDown) => {
                            if (inputDcRef.current?.readyState === 'open') {
                                inputDcRef.current.send(JSON.stringify({ 
                                    type: isDown ? 'key_down' : 'key_up', 
                                    key 
                                }));
                            }
                        }}
                        onClose={() => setShowVirtualKeyboard(false)}
                    />
                )}
            </div>
        </div>
    );
};

export default RdpViewer;
