import { Shield } from 'lucide-react'

interface SessionStatusProps {
  machineId: string
  onTerminate: () => void
}

export function SessionStatus({ machineId, onTerminate }: SessionStatusProps) {
  return (
    <div className="bg-blue-600/10 border border-blue-500/30 p-6 rounded-2xl animate-in fade-in duration-500">
      <h3 className="text-xl font-bold text-white mb-2 flex items-center gap-2">
        <Shield className="w-5 h-5 text-blue-400" />
        Active Session
      </h3>
      <p className="text-blue-200/70 mb-6 font-mono text-xs">
        Target: {machineId}
      </p>
      <button
        onClick={onTerminate}
        className="w-full py-3 bg-red-600/20 hover:bg-red-600/30 text-red-400 border border-red-500/50 rounded-xl font-medium transition-colors"
      >
        Terminate Session
      </button>
    </div>
  )
}
