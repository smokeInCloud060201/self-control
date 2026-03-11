import React from 'react'
import { Terminal, Key, Zap } from 'lucide-react'

interface ConnectionFormProps {
  machineId: string
  password: string
  setMachineId: (s: string) => void
  setPassword: (s: string) => void
  onSubmit: (e: React.FormEvent) => void
}

export function ConnectionForm({ 
  machineId, 
  password, 
  setMachineId, 
  setPassword, 
  onSubmit 
}: ConnectionFormProps) {
  return (
    <form
      onSubmit={onSubmit}
      className="relative group overflow-hidden bg-slate-900/40 backdrop-blur-xl p-6 sm:p-8 rounded-3xl border border-white/5 shadow-2xl space-y-8 transition-all hover:border-blue-500/30"
    >
      <div className="absolute inset-0 bg-gradient-to-br from-blue-500/5 to-transparent pointer-events-none" />

      <div className="space-y-6 relative z-10">
        <div className="space-y-2">
          <label className="text-xs font-bold text-slate-500 uppercase tracking-widest ml-1">Remote Machine ID</label>
          <div className="relative group/input">
            <Terminal className="absolute left-4 top-1/2 -translate-y-1/2 w-5 h-5 text-slate-600 group-focus-within/input:text-blue-500 transition-colors" />
            <input
              type="text"
              value={machineId}
              onChange={(e) => setMachineId(e.target.value)}
              className="w-full bg-slate-950/50 border border-white/5 rounded-2xl py-4 pl-12 pr-4 text-white focus:ring-2 focus:ring-blue-500/50 outline-none transition-all placeholder:text-slate-700"
              placeholder="e.g. 550e8400-e29b-41d4-a716-446655440000"
              required
            />
          </div>
        </div>

        <div className="space-y-2">
          <label className="text-xs font-bold text-slate-500 uppercase tracking-widest ml-1">Passkey</label>
          <div className="relative group/input">
            <Key className="absolute left-4 top-1/2 -translate-y-1/2 w-5 h-5 text-slate-600 group-focus-within/input:text-blue-500 transition-colors" />
            <input
              type="text"
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              className="w-full bg-slate-950/50 border border-white/5 rounded-2xl py-4 pl-12 pr-4 text-white focus:ring-2 focus:ring-blue-500/50 outline-none transition-all placeholder:text-slate-700 font-mono"
              placeholder="000000"
              maxLength={6}
              required
            />
          </div>
        </div>
      </div>

      <button
        type="submit"
        className="w-full relative py-4 bg-blue-600 hover:bg-blue-500 text-white rounded-2xl font-black text-lg shadow-xl shadow-blue-500/20 flex items-center justify-center gap-3 transition-all hover:scale-[1.02] active:scale-[0.98] overflow-hidden group/btn"
      >
        <div className="absolute inset-0 bg-gradient-to-r from-transparent via-white/10 to-transparent -translate-x-full group-hover/btn:animate-shimmer" />
        <Zap className="w-6 h-6 fill-current" />
        <span>Establish Connection</span>
      </button>
    </form>
  )
}
