import { Monitor, Github } from 'lucide-react'
import { cn } from '../../lib/utils'

interface HeaderProps {
  isConnected: boolean
}

export function Header({ isConnected }: HeaderProps) {
  return (
    <nav className="border-b border-slate-800 bg-slate-900/50 backdrop-blur-xl sticky top-0 z-50">
      <div className={cn("mx-auto px-4 sm:px-6 lg:px-8", isConnected ? "max-w-none" : "max-w-7xl")}>
        <div className="flex items-center justify-between h-16">
          <div className="flex items-center gap-3">
            <div className="bg-gradient-to-tr from-blue-600 to-indigo-600 p-2 rounded-lg shadow-lg shadow-blue-500/20">
              <Monitor className="w-6 h-6 text-white" />
            </div>
            <span className="text-xl font-bold bg-clip-text text-transparent bg-gradient-to-r from-white to-slate-400">
              SelfControl
            </span>
          </div>
          <div className="flex items-center gap-6">
            <a href="https://github.com/smokeInCloud060201/self-control" className="flex items-center gap-2 px-4 py-2 bg-slate-800 hover:bg-slate-700 rounded-full text-sm font-medium transition-all border border-slate-700">
              <Github className="w-4 h-4" />
              GitHub
            </a>
          </div>
        </div>
      </div>
    </nav>
  )
}
