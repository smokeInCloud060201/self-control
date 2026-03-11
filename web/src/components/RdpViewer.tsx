import React, { useEffect, useRef, useState } from 'react';
import { AlertCircle, ClipboardPaste, Loader2, Maximize2, Monitor } from 'lucide-react';
import { cn } from '../lib/utils';

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
    const [error, setError] = useState<string | null>(null);
    const wsRef = useRef<WebSocket | null>(null);
    const [aspectRatio, setAspectRatio] = useState<number>(16 / 9);
    const [displays, setDisplays] = useState<any[]>([]);
    const [currentDisplay, setCurrentDisplay] = useState<number>(0);

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
        if (!canvasRef.current) return;

        let isCleanup = false;

        const startConnection = () => {
            if (wsRef.current) return;

            setStatus('connecting');
            setError(null);

            const cleanProxyUrl = proxyUrl.endsWith('/') ? proxyUrl.slice(0, -1) : proxyUrl;
            const wsUrl = `${cleanProxyUrl}/client/${sessionId}/${password || 'no-pass'}`;

            console.log('[DEBUG] Connecting to WebSocket:', wsUrl);

            try {
                const ws = new WebSocket(wsUrl);
                ws.binaryType = 'arraybuffer';
                wsRef.current = ws;

                ws.onopen = () => {
                    if (isCleanup) { ws.close(); return; }
                    setStatus('connected');

                    // Send client resolution to Agent for adaptive workspace
                    const width = window.screen.width * window.devicePixelRatio;
                    const height = window.screen.height * window.devicePixelRatio;
                    ws.send(JSON.stringify({
                        type: 'resolution_update',
                        width: Math.round(width),
                        height: Math.round(height)
                    }));
                };

                ws.onmessage = async (event) => {
                    if (isCleanup) return;
                    if (event.data instanceof ArrayBuffer) {
                        try {
                            const blob = new Blob([event.data], { type: 'image/jpeg' });
                            const bitmap = await createImageBitmap(blob);

                            const canvas = canvasRef.current;
                            if (canvas) {
                                const ctx = canvas.getContext('2d');
                                if (ctx) {
                                    if (canvas.width !== bitmap.width || canvas.height !== bitmap.height) {
                                        canvas.width = bitmap.width;
                                        canvas.height = bitmap.height;
                                        setAspectRatio(bitmap.width / bitmap.height);
                                        console.log(`[DEBUG] Resolution: ${bitmap.width}x${bitmap.height}, Ratio: ${bitmap.width / bitmap.height}`);
                                    }
                                    ctx.drawImage(bitmap, 0, 0);
                                }
                            }
                        } catch (e) {
                            console.error('[DEBUG] Frame decode error:', e);
                        }
                    } else if (typeof event.data === 'string') {
                        try {
                            const json = JSON.parse(event.data);
                            if (json.type === 'metadata' && json.displays) {
                                console.log('[DEBUG] Displays found:', json.displays);
                                setDisplays(json.displays);
                            } else if (json.type === 'clipboard_sync') {
                                console.log('[DEBUG] Clipboard sync received');
                                if (json.text) {
                                    navigator.clipboard.writeText(json.text).catch(err => {
                                        console.error('Failed to write to local clipboard:', err);
                                    });
                                }
                            }
                        } catch (e) { }
                    }
                };

                ws.onerror = (e) => {
                    if (isCleanup) return;
                    console.error('[DEBUG] WebSocket Error:', e);
                    setError('Connection failed. Please check credentials and proxy.');
                    setStatus('error');
                };

                ws.onclose = (event) => {
                    if (isCleanup) {
                        // console.log('[DEBUG] WebSocket closed via cleanup'); // Removed as per instructions
                        return;
                    }
                    console.log('[DEBUG] WebSocket Closed:', event.code, event.reason);
                    setStatus('idle');
                    if (event.code !== 1000 && event.code !== 1001) { // 1000: Normal Closure, 1001: Going Away
                        setError(`Session ended: ${event.reason || 'Network error'} (${event.code})`);
                        setStatus('error');
                    }
                    onDisconnect?.();
                };
            } catch (err) {
                setError('Failed to initialize WebSocket.');
                setStatus('error');
            }
        };

        startConnection();

        return () => {
            // console.log('[DEBUG] Cleaning up RdpViewer effect'); // Removed as per instructions
            isCleanup = true;
            if (wsRef.current) {
                wsRef.current.close(1000, 'Unmount');
                wsRef.current = null;
            }
        };
    }, [proxyUrl, sessionId, password]);

    // Handle Input Events
    const handleMouseMove = (e: React.MouseEvent) => {
        if (wsRef.current?.readyState === WebSocket.OPEN && canvasRef.current) {
            const rect = canvasRef.current.getBoundingClientRect();
            // Using nativeEvent.offsetX is more accurate relative to the content area
            const x = e.nativeEvent.offsetX / rect.width;
            const y = e.nativeEvent.offsetY / rect.height;

            wsRef.current.send(JSON.stringify({ type: 'MouseMove', x, y }));
        }
    };

    const handleMouseDown = (button: 'left' | 'right') => {
        if (wsRef.current?.readyState === WebSocket.OPEN) {
            wsRef.current.send(JSON.stringify({
                type: 'MouseDown',
                button
            }));
        }
    };

    const handleMouseUp = (button: 'left' | 'right') => {
        if (wsRef.current?.readyState === WebSocket.OPEN) {
            wsRef.current.send(JSON.stringify({
                type: 'MouseUp',
                button
            }));
        }
    };

    const handleKeyDown = (e: React.KeyboardEvent) => {
        if (wsRef.current?.readyState === WebSocket.OPEN) {
            // Intercept Shortcuts (Cmd+V on Mac, Ctrl+V otherwise)
            const isMod = e.metaKey || e.ctrlKey;
            if (isMod && e.key.toLowerCase() === 'v') {
                syncClipboard();
                return; // Don't send the V to the remote as it's handled by PasteText
            }

            wsRef.current.send(JSON.stringify({
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
        if (wsRef.current?.readyState === WebSocket.OPEN) {
            wsRef.current.send(JSON.stringify({
                type: 'KeyUp',
                key: e.key
            }));
        }
    };

    const switchDisplay = (index: number) => {
        if (wsRef.current?.readyState === WebSocket.OPEN) {
            wsRef.current.send(JSON.stringify({ type: 'switch_display', index }));
            setCurrentDisplay(index);
        }
    };

    const syncClipboard = async () => {
        try {
            const text = await navigator.clipboard.readText();
            if (text && wsRef.current?.readyState === WebSocket.OPEN) {
                wsRef.current.send(JSON.stringify({ type: 'paste_text', text }));
            }
        } catch (err) {
            console.error('Failed to read clipboard:', err);
        }
    };

    return (
        <div className="flex flex-col gap-6 w-full max-w-7xl mx-auto">
            {status === 'connected' && (
                <div className="flex flex-wrap items-center justify-between gap-4 px-8 py-4 bg-slate-900/50 backdrop-blur-xl border border-white/5 rounded-[2rem] shadow-2xl animate-in slide-in-from-top-4 duration-700">
                    <div className="flex items-center gap-6">
                        <div className="flex items-center gap-3 px-4 py-2 bg-blue-500/10 border border-blue-500/20 rounded-2xl">
                            <div className="relative">
                                <div className="w-2.5 h-2.5 bg-green-500 rounded-full animate-ping absolute inset-0" />
                                <div className="w-2.5 h-2.5 bg-green-500 rounded-full relative" />
                            </div>
                            <span className="text-[10px] font-black text-blue-400 uppercase tracking-[0.2em]">Active Session</span>
                        </div>

                        {displays.length > 1 && (
                            <div className="flex items-center bg-slate-950/40 p-1 rounded-2xl border border-white/5">
                                {displays.map((_, i) => (
                                    <button
                                        key={i}
                                        onClick={() => switchDisplay(i)}
                                        className={cn(
                                            "px-4 py-2 rounded-xl text-[10px] font-black uppercase tracking-widest transition-all gap-2 flex items-center min-w-[100px] justify-center",
                                            currentDisplay === i
                                                ? "bg-white text-slate-950 shadow-xl scale-105"
                                                : "text-slate-500 hover:text-white hover:bg-white/5"
                                        )}
                                    >
                                        <Monitor className="w-3.5 h-3.5" />
                                        Display {i + 1}
                                    </button>
                                ))}
                            </div>
                        )}
                    </div>

                    <div className="flex items-center gap-3">
                        <span className="text-slate-600 font-mono text-[10px] uppercase tracking-widest mr-4">ID: {sessionId.slice(0, 8)}</span>

                        <button
                            onClick={syncClipboard}
                            className="flex items-center gap-2 px-5 py-2.5 bg-white/5 hover:bg-white/10 text-white rounded-2xl border border-white/10 transition-all group active:scale-95 text-[10px] font-black uppercase tracking-widest"
                            title="Paste Local Clipboard to Remote"
                        >
                            <ClipboardPaste className="w-4 h-4 group-hover:text-blue-400 transition-colors" />
                            Paste to Remote
                        </button>

                        <button
                            onClick={toggleFullscreen}
                            className="flex items-center gap-2 px-5 py-2.5 bg-white/5 hover:bg-white/10 text-white rounded-2xl border border-white/10 transition-all group active:scale-95 text-[10px] font-black uppercase tracking-widest"
                            title="Fullscreen"
                        >
                            <Maximize2 className="w-4 h-4 group-hover:text-blue-400 transition-colors" />
                            Fullscreen
                        </button>
                    </div>
                </div>
            )}

            <div
                ref={containerRef}
                style={{ aspectRatio: `${aspectRatio}` }}
                className={cn(
                    "relative w-full bg-slate-950 rounded-[3rem] overflow-hidden border border-white/5 shadow-[0_0_100px_-20px_rgba(0,0,0,0.5)] transition-all outline-none",
                    status === 'connected' ? "ring-1 ring-white/10" : ""
                )}
            >
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
                    onMouseMove={handleMouseMove}
                    onMouseDown={(e) => {
                        e.currentTarget.focus();
                        handleMouseDown(e.button === 0 ? 'left' : 'right');
                    }}
                    onMouseUp={(e) => handleMouseUp(e.button === 0 ? 'left' : 'right')}
                    onKeyDown={handleKeyDown}
                    onKeyUp={handleKeyUp}
                    onContextMenu={(e) => e.preventDefault()}
                />
            </div>
        </div>
    );
};

export default RdpViewer;
