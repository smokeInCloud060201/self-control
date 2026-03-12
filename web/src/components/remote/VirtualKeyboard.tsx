import React, { useState } from 'react';
import { CornerDownLeft, Delete as Backspace, ChevronUp, Command, Option, Cpu as Control } from 'lucide-react';
import { cn } from '../../lib/utils';

interface VirtualKeyboardProps {
    onKeyPress: (key: string, isDown: boolean) => void;
    onClose: () => void;
}

export function VirtualKeyboard({ onKeyPress, onClose }: VirtualKeyboardProps) {
    const [isShift, setIsShift] = useState(false);
    const [activeModifiers, setActiveModifiers] = useState<Set<string>>(new Set());

    const toggleModifier = (mod: string) => {
        const newModifiers = new Set(activeModifiers);
        if (newModifiers.has(mod)) {
            newModifiers.delete(mod);
            onKeyPress(mod, false);
        } else {
            newModifiers.add(mod);
            onKeyPress(mod, true);
        }
        setActiveModifiers(newModifiers);
    };

    const handleKey = (key: string) => {
        let finalKey = key;
        
        // Handle case transformation
        if (key.length === 1) {
            finalKey = isShift ? key.toUpperCase() : key.toLowerCase();
        }

        // Special handling for key mapping to match agent expectations
        if (key === 'Enter') finalKey = 'enter';
        if (key === 'Backspace') finalKey = 'backspace';
        if (key === 'Tab') finalKey = 'tab';
        if (key === 'Esc') finalKey = 'esc';
        if (key === 'Space') finalKey = 'space';

        onKeyPress(finalKey, true);
        setTimeout(() => onKeyPress(finalKey, false), 50);

        // Auto-reset shift
        if (isShift) setIsShift(false);
    };

    const Key = ({ label, value, className, icon: Icon, wide }: { label?: string, value: string, className?: string, icon?: any, wide?: boolean }) => (
        <button
            onClick={() => handleKey(value)}
            className={cn(
                "flex items-center justify-center rounded-lg h-12 bg-slate-800/80 border border-white/5 text-white active:bg-blue-600 active:scale-95 transition-all text-sm font-medium touch-none",
                wide ? "flex-[1.5]" : "flex-1",
                className
            )}
        >
            {Icon ? <Icon className="w-4 h-4" /> : (label || value)}
        </button>
    );

    const ModifierKey = ({ label, value, icon: Icon }: { label: string, value: string, icon: any }) => (
        <button
            onClick={() => toggleModifier(value)}
            className={cn(
                "flex-[1.2] flex items-center justify-center rounded-lg h-12 border transition-all text-[10px] font-black uppercase tracking-widest touch-none gap-1",
                activeModifiers.has(value) 
                    ? "bg-blue-600 border-blue-400 text-white" 
                    : "bg-slate-900/90 border-white/5 text-slate-400 active:bg-slate-800"
            )}
        >
            <Icon className="w-3 h-3" />
            <span className="hidden xs:inline">{label}</span>
        </button>
    );

    return (
        <div className="fixed bottom-0 left-0 right-0 z-[100] bg-slate-950/90 backdrop-blur-2xl border-t border-white/10 p-2 pb-8 sm:p-4 sm:pb-10 safe-area-bottom animate-in slide-in-from-bottom duration-300">
            <div className="max-w-3xl mx-auto space-y-2">
                {/* Modifiers Row */}
                <div className="flex gap-1.5 mb-2">
                    <ModifierKey label="Shift" value="shift" icon={ChevronUp} />
                    <ModifierKey label="Ctrl" value="control" icon={Control} />
                    <ModifierKey label="Option" value="alt" icon={Option} />
                    <ModifierKey label="Cmd" value="meta" icon={Command} />
                    <button 
                        onClick={onClose}
                        className="ml-auto px-4 text-xs font-bold text-slate-500 uppercase tracking-widest hover:text-white transition-colors"
                    >
                        Hide
                    </button>
                </div>

                {/* Keyboard Rows */}
                <div className="flex gap-1">
                    {['1','2','3','4','5','6','7','8','9','0'].map(n => <Key key={n} value={n} />)}
                    <Key value="Backspace" icon={Backspace} wide />
                </div>
                
                <div className="flex gap-1">
                    {['q','w','e','r','t','y','u','i','o','p'].map(k => <Key key={k} value={k} label={k.toUpperCase()} />)}
                </div>

                <div className="flex gap-1 pl-4">
                    {['a','s','d','f','g','h','j','k','l'].map(k => <Key key={k} value={k} label={k.toUpperCase()} />)}
                    <Key value="Enter" icon={CornerDownLeft} wide className="bg-blue-600/20 border-blue-500/30 text-blue-400" />
                </div>

                <div className="flex gap-1">
                    <button 
                        onClick={() => setIsShift(!isShift)}
                        className={cn("flex-[1.2] h-12 rounded-lg border transition-all touch-none flex items-center justify-center", 
                            isShift ? "bg-white text-slate-950 border-white" : "bg-slate-800/80 border-white/5 text-white")}
                    >
                        <ChevronUp className="w-4 h-4" />
                    </button>
                    {['z','x','c','v','b','n','m',',','.','/'].map(k => <Key key={k} value={k} label={k.toUpperCase()} />)}
                </div>

                <div className="flex gap-1">
                    <Key value="Tab" label="TAB" className="text-[10px]" />
                    <Key value="Esc" label="ESC" className="text-[10px]" />
                    <Key value="Space" label="SPACE" className="flex-[4] tracking-widest text-[10px]" />
                    <div className="flex flex-[1] gap-1">
                        <Key value="ArrowLeft" label="←" />
                        <div className="flex flex-col flex-1 gap-1">
                            <Key value="ArrowUp" label="↑" className="h-5.5 text-[8px]" />
                            <Key value="ArrowDown" label="↓" className="h-5.5 text-[8px]" />
                        </div>
                        <Key value="ArrowRight" label="→" />
                    </div>
                </div>
            </div>
        </div>
    );
}
