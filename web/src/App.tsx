import { useState } from 'react'
import { Activity } from 'lucide-react'
import RdpViewer from './components/remote/RdpViewer'
import { Header } from './components/layout/Header'
import { ConnectionForm } from './components/forms/ConnectionForm'
import { SessionStatus } from './components/remote/SessionStatus'
import { RemotePlaceholder } from './components/remote/RemotePlaceholder'
import { cn } from './lib/utils'
import './styles/App.css'

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
      <Header isConnected={isConnected} />

      <main className={cn("mx-auto px-4 sm:px-6 lg:px-8 py-8 sm:py-12 min-h-[calc(100vh-4rem)] transitions-all", isConnected ? "max-w-none" : "max-w-7xl")}>
        <div className={cn("grid gap-8 lg:gap-12 items-start h-full", isConnected ? "lg:grid-cols-1" : "lg:grid-cols-12")}>
          
          {/* Left Column: UI & Controls */}
          <div className={cn("space-y-8", isConnected ? "hidden" : "lg:col-span-5")}>
            <div className="space-y-6">
              <div className="inline-flex items-center gap-2 px-3 py-1 bg-blue-500/10 border border-blue-500/20 rounded-full text-blue-400 text-sm font-medium animate-in fade-in slide-in-from-left-4 duration-700">
                <Activity className="w-4 h-4" />
                <span>Ultra-low latency streaming active</span>
              </div>
              <h1 className="text-4xl sm:text-5xl lg:text-6xl font-black tracking-tight text-white leading-[1.1]">
                Modern Remote {" "}
                <span className="text-transparent bg-clip-text bg-gradient-to-r from-blue-400 to-indigo-500">
                  Control Hub
                </span>
              </h1>
              <p className="text-xl text-slate-400 max-w-md leading-relaxed">
                Experience high-performance remote access powered by Rust. Secure, fast, and beautifully simple.
              </p>
            </div>

            {!isConnected ? (
              <ConnectionForm 
                machineId={machineId}
                password={password}
                setMachineId={setMachineId}
                setPassword={setPassword}
                onSubmit={handleConnect}
              />
            ) : (
              <SessionStatus 
                machineId={machineId}
                onTerminate={handleTerminate}
              />
            )}
          </div>

          {/* Right Column: Viewer */}
          <div className={isConnected ? "w-full h-full min-h-[500px]" : "lg:col-span-7 h-full min-h-[400px] sm:min-h-[500px]"}>
            {isConnected ? (
              <RdpViewer
                sessionId={machineId}
                password={password}
                proxyUrl={proxyUrl}
                onDisconnect={handleTerminate}
              />
            ) : (
              <RemotePlaceholder />
            )}
          </div>

        </div>
      </main>
    </div>
  )
}

export default App
