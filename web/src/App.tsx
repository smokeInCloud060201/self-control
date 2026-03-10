import { useState } from 'react'
import { Monitor, Shield, Zap, Github, Terminal, Key, Activity } from 'lucide-react'
import RdpViewer from './components/RdpViewer'

function App() {
  const [machineId, setMachineId] = useState('')
  const [password, setPassword] = useState('')
  const [proxyUrl] = useState(import.meta.env.VITE_PROXY_URL || 'wss://selfcontrol-api.sonbn.xyz:443')
  const [isConnected, setIsConnected] = useState(false)

  const handleConnect = (e: React.FormEvent) => {
    e.preventDefault()
    if (machineId && password) {
      console.log('User initiated connection with:', { machineId, password, proxyUrl });
      setIsConnected(true)
    }
  }

  const handleTerminate = () => {
    console.log('Terminating session...');
    setIsConnected(false)
  }

  return (
    <div className="min-h-screen bg-[#0f172a] text-slate-200 font-sans selection:bg-blue-500/30">
      {/* Header */}
      <nav className="border-b border-slate-800 bg-slate-900/50 backdrop-blur-xl sticky top-0 z-50">
        <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
          <div className="flex items-center justify-between h-16">
            <div className="flex items-center gap-3">
              <div className="bg-gradient-to-tr from-blue-600 to-indigo-600 p-2 rounded-lg shadow-lg shadow-blue-500/20">
                <Monitor className="w-6 h-6 text-white" />
              </div>
              <span className="text-xl font-bold bg-clip-text text-transparent bg-gradient-to-r from-white to-slate-400">
                RustRemote
              </span>
            </div>
            <div className="flex items-center gap-6">
              <a href="#" className="flex items-center gap-2 px-4 py-2 bg-slate-800 hover:bg-slate-700 rounded-full text-sm font-medium transition-all border border-slate-700">
                <Github className="w-4 h-4" />
                GitHub
              </a>
            </div>
          </div>
        </div>
      </nav>

      <main className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-12 h-[calc(100vh-5rem)]">
        <div className="grid lg:grid-cols-12 gap-12 items-start h-full">

          {/* Left Column: UI & Controls */}
          <div className="lg:col-span-5 space-y-8">
            <div className="space-y-6">
              <div className="inline-flex items-center gap-2 px-3 py-1 bg-blue-500/10 border border-blue-500/20 rounded-full text-blue-400 text-sm font-medium animate-in fade-in slide-in-from-left-4 duration-700">
                <Activity className="w-4 h-4" />
                <span>Ultra-low latency streaming active</span>
              </div>
              <h1 className="text-6xl font-black tracking-tight text-white leading-[1.1]">
                Modern Remote <br />
                <span className="text-transparent bg-clip-text bg-gradient-to-r from-blue-400 to-indigo-500">
                  Control Hub
                </span>
              </h1>
              <p className="text-xl text-slate-400 max-w-md leading-relaxed">
                Experience high-performance remote access powered by Rust. Secure, fast, and beautifully simple.
              </p>
            </div>

            {!isConnected ? (
              <form
                onSubmit={handleConnect}
                className="relative group overflow-hidden bg-slate-900/40 backdrop-blur-xl p-8 rounded-3xl border border-white/5 shadow-2xl space-y-8 transition-all hover:border-blue-500/30"
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
            ) : (
              <div className="bg-blue-600/10 border border-blue-500/30 p-6 rounded-2xl">
                <h3 className="text-xl font-bold text-white mb-2 flex items-center gap-2">
                  <Shield className="w-5 h-5 text-blue-400" />
                  Active Session
                </h3>
                <p className="text-blue-200/70 mb-6 font-mono text-xs">
                  Target: {machineId}
                </p>
                <button
                  onClick={handleTerminate}
                  className="w-full py-3 bg-red-600/20 hover:bg-red-600/30 text-red-400 border border-red-500/50 rounded-xl font-medium transition-colors"
                >
                  Terminate Session
                </button>
              </div>
            )}
          </div>

          {/* Right Column: Viewer */}
          <div className="lg:col-span-7 h-full min-h-[500px]">
            {isConnected ? (
              <RdpViewer
                sessionId={machineId}
                password={password}
                proxyUrl={proxyUrl}
                onDisconnect={handleTerminate}
              />
            ) : (
              <div className="h-full bg-slate-950 rounded-2xl border border-slate-800 flex flex-col items-center justify-center p-12 text-center group">
                <Monitor className="w-20 h-20 text-slate-800 mb-6" />
                <h3 className="text-2xl font-bold text-slate-600 mb-2">Waiting for Credentials</h3>
                <p className="text-slate-600 max-w-xs text-sm">Once connected, the remote screen will appear here in high definition.</p>
              </div>
            )}
          </div>

        </div>
      </main>
    </div>
  )
}

export default App
