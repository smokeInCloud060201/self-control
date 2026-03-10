import React, { useEffect, useRef, useState } from 'react';
import { AlertCircle, Loader2, Maximize2 } from 'lucide-react';
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
                        // console.log('[DEBUG] Text message received:', event.data); // Removed as per instructions
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

    return (
        <div
            ref={containerRef}
            style={{ aspectRatio: `${aspectRatio}` }}
            className={cn(
                "relative w-full bg-slate-950 rounded-3xl overflow-hidden border border-white/5 shadow-2xl transition-all outline-none",
                status === 'connected' ? "ring-2 ring-blue-500/20 shadow-blue-500/10" : ""
            )}
        >
            {status === 'connected' && (
                <div className="absolute top-6 left-6 flex items-center gap-3 z-30 animate-in fade-in zoom-in duration-500">
                    <button
                        onClick={toggleFullscreen}
                        className="p-3 bg-slate-950/40 hover:bg-slate-900/60 text-white rounded-2xl backdrop-blur-xl border border-white/10 transition-all group scale-100 hover:scale-110 active:scale-95 shadow-xl"
                        title="Fullscreen"
                    >
                        <Maximize2 className="w-5 h-5 group-hover:text-blue-400 transition-colors" />
                    </button>

                    <div className="px-4 py-2 bg-slate-950/40 backdrop-blur-xl border border-white/10 rounded-2xl shadow-xl flex items-center gap-3">
                        <div className="relative">
                            <div className="w-2.5 h-2.5 bg-green-500 rounded-full animate-ping absolute inset-0" />
                            <div className="w-2.5 h-2.5 bg-green-500 rounded-full relative" />
                        </div>
                        <span className="text-xs font-black text-white uppercase tracking-[0.2em]">Live Stream</span>
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
                    status === 'connected' ? "opacity-100" : "opacity-0"
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
    );
};

export default RdpViewer;
