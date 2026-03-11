import { Monitor } from 'lucide-react'

export function RemotePlaceholder() {
  return (
    <div className="h-full bg-slate-950 rounded-2xl border border-slate-800 flex flex-col items-center justify-center p-12 text-center group">
      <Monitor className="w-20 h-20 text-slate-800 mb-6" />
      <h3 className="text-2xl font-bold text-slate-600 mb-2">Waiting for Credentials</h3>
      <p className="text-slate-600 max-w-xs text-sm">Once connected, the remote screen will appear here in high definition.</p>
    </div>
  )
}
