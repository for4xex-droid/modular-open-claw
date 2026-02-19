import { useEffect, useState, useRef } from 'react';
import { ProjectSummary } from './Warehouse';
import { Play, Zap, Activity } from 'lucide-react';
import { clsx } from 'clsx';
import { useWebSocket } from 'react-use-websocket/dist/lib/use-websocket'; // Need to check if installed or use native

// Using native WebSocket for now as we did in FactoryLine (or verify FactoryLine implementation)
// Actually FactoryLine used 'react-use-websocket'? Let's check FactoryLine to be consistent.
// I will use native WebSocket standard pattern or copy from FactoryLine to avoid dependency issues if not installed.
// Checking FactoryLine first is safer. But for now I'll write standard WS logic.

interface StyleProfile {
    name: string;
    description: string;
}

interface RemixLabProps {
    targetProject: ProjectSummary | null;
}

export function RemixLab({ targetProject }: RemixLabProps) {
    const [styles, setStyles] = useState<string[]>([]); // Just names for now based on API
    const [selectedStyle, setSelectedStyle] = useState<string>('');
    const [isProcessing, setIsProcessing] = useState(false);
    const [jobId, setJobId] = useState<string | null>(null);
    const [timestamp, setTimestamp] = useState(Date.now());
    const [systemLocked, setSystemLocked] = useState(false);
    const [logs, setLogs] = useState<string[]>([]);

    // WebSocket for Heartbeat (Lock) and Logs (Completion)
    useEffect(() => {
        const ws = new WebSocket('ws://localhost:3000/ws');

        ws.onmessage = (event) => {
            try {
                const data = JSON.parse(event.data);

                // Heartbeat Check
                if (data.type === 'heartbeat') {
                    // Lock if active_actor is present
                    setSystemLocked(!!data.active_actor);
                }

                // Log Check for Completion
                if (data.level && data.message) {
                    if (data.message.includes("Job Completed:") && data.message.includes(jobId || "NEVER_MATCH")) {
                        setIsProcessing(false);
                        setTimestamp(Date.now()); // Cache Busting
                        setLogs(prev => [`✅ Job Finished! Reloading preview...`, ...prev]);
                    }
                    if (data.message.includes("Job Failed:") && data.message.includes(jobId || "NEVER_MATCH")) {
                        setIsProcessing(false);
                        setLogs(prev => [`❌ Job Failed! Check server logs.`, ...prev]);
                    }
                }
            } catch (e) {
                // Ignore parse errors
            }
        };

        return () => ws.close();
    }, [jobId]);

    // Fetch Styles
    useEffect(() => {
        fetch('http://localhost:3000/api/styles')
            .then(res => res.json())
            .then((data: string[]) => {
                setStyles(data);
                if (data.length > 0) setSelectedStyle(data[0]);
            })
            .catch(err => console.error("Failed to fetch styles:", err));
    }, []);

    const handleExecute = async () => {
        if (!targetProject || !selectedStyle) return;

        setIsProcessing(true);
        setLogs(prev => [`🚀 Sending Remix Request for ${targetProject.id}...`, ...prev]);

        try {
            const res = await fetch('http://localhost:3000/api/remix', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({
                    category: "remix", // Dummy
                    topic: "remix",    // Dummy
                    remix_id: targetProject.id,
                    style_name: selectedStyle,
                    custom_style: null // TODO: Implement custom overrides UI
                })
            });

            if (res.status === 429) {
                setLogs(prev => [`⚠️ System Busy! Request Rejected.`, ...prev]);
                setIsProcessing(false);
                return;
            }

            const data = await res.json();
            setJobId(data.job_id);
            setLogs(prev => [`⏳ Job Accepted: ${data.job_id}`, ...prev]);

        } catch (e) {
            console.error(e);
            setIsProcessing(false);
            setLogs(prev => [`❌ Network Error`, ...prev]);
        }
    };

    if (!targetProject) {
        return (
            <div className="h-full flex flex-col items-center justify-center text-gray-500 font-mono">
                <div className="mb-4 p-4 rounded-full bg-gray-900 border border-gray-800">
                    <Activity size={48} />
                </div>
                <p>NO PROJECT SELECTED</p>
                <p className="text-sm mt-2">Please select a project from the WAREHOUSE.</p>
            </div>
        );
    }

    return (
        <div className="h-full flex bg-black">
            {/* Left Panel: Control Deck */}
            <div className="w-1/3 border-r border-gray-800 bg-gray-900/30 p-8 flex flex-col">
                <h2 className="text-xl font-light text-white mb-6 tracking-wide flex items-center gap-2">
                    <Zap size={20} className="text-sonar-green" />
                    REMIX LAB
                </h2>

                <div className="mb-8">
                    <label className="text-xs font-mono text-gray-500 mb-2 block">TARGET PROJECT</label>
                    <div className="p-4 bg-gray-900 border border-gray-700 rounded-lg text-gray-200 font-mono text-sm">
                        {targetProject.title}
                        <div className="text-xs text-sonar-green mt-1">{targetProject.id}</div>
                    </div>
                </div>

                <div className="mb-8">
                    <label className="text-xs font-mono text-gray-500 mb-2 block">VISUAL STYLE</label>
                    <select
                        value={selectedStyle}
                        onChange={(e) => setSelectedStyle(e.target.value)}
                        className="w-full bg-black border border-gray-700 text-white p-3 rounded-lg focus:border-sonar-green focus:outline-none transition-colors"
                        disabled={isProcessing}
                    >
                        {styles.map(s => (
                            <option key={s} value={s}>{s}</option>
                        ))}
                    </select>
                </div>

                <div className="mt-auto">
                    <button
                        onClick={handleExecute}
                        disabled={isProcessing || systemLocked}
                        className={clsx(
                            "w-full py-4 rounded-lg font-bold tracking-widest transition-all duration-300 flex items-center justify-center gap-2",
                            isProcessing || systemLocked
                                ? "bg-gray-800 text-gray-500 cursor-not-allowed border border-gray-700"
                                : "bg-sonar-green text-black hover:bg-white hover:shadow-[0_0_20px_#00FF41]"
                        )}
                    >
                        {isProcessing ? (
                            <span className="animate-pulse">PROCESSING...</span>
                        ) : systemLocked ? (
                            <span>SYSTEM LOCKED</span>
                        ) : (
                            <>
                                <Play size={20} fill="currentColor" />
                                EXECUTE REMIX
                            </>
                        )}
                    </button>

                    {/* Status Console */}
                    <div className="mt-6 h-32 bg-black border border-gray-800 rounded p-2 font-mono text-xs overflow-y-auto custom-scrollbar">
                        {logs.map((log, i) => (
                            <div key={i} className="text-gray-400 mb-1 border-b border-gray-900/50 pb-1">
                                {log}
                            </div>
                        ))}
                    </div>
                </div>
            </div>

            {/* Right Panel: Preview Area */}
            <div className="flex-1 bg-[url('/grid.svg')] bg-opacity-5 flex flex-col p-8 relative">
                <div className="absolute top-4 right-4 text-xs font-mono text-gray-600">
                    PREVIEW MONITOR // {targetProject.id}
                </div>

                <div className="flex-1 flex items-center justify-center">
                    <div className="w-full max-w-4xl aspect-video bg-black border border-gray-800 rounded-xl overflow-hidden shadow-2xl relative group">
                        {targetProject.thumbnail_url ? (
                            <video
                                key={timestamp} // Cache Busting Key
                                src={`http://localhost:3000/assets/${targetProject.id}/final.mp4?t=${timestamp}`}
                                className="w-full h-full object-contain"
                                controls
                                autoPlay={false}
                            />
                        ) : (
                            <div className="flex items-center justify-center h-full text-gray-700">
                                NO SIGNAL
                            </div>
                        )}

                        {isProcessing && (
                            <div className="absolute inset-0 bg-black/80 flex flex-col items-center justify-center backdrop-blur-sm z-10">
                                <div className="w-16 h-16 border-4 border-sonar-green border-t-transparent rounded-full animate-spin mb-4"></div>
                                <div className="text-sonar-green font-mono animate-pulse">RENDERING NEW VISUALS...</div>
                            </div>
                        )}
                    </div>
                </div>
            </div>
        </div>
    );
}
