import useSWR from 'swr';
import { Database, Lightbulb, RefreshCw, Layers } from 'lucide-react';
import { clsx } from 'clsx';

const fetcher = (url: string) => fetch(url).then(r => r.json());

interface Karma {
    id: string;
    job_id: string;
    skill_id: string;
    lesson: string;
    karma_type: string;
    weight: number;
    created_at: string;
    last_applied_at: string | null;
    soul_version_hash: string;
}

export const KarmaViewer = () => {
    const { data: karmas, error, mutate, isValidating } = useSWR<Karma[]>('http://localhost:3000/api/karma', fetcher, {
        refreshInterval: 10000
    });

    if (error) return <div className="p-8 text-red-500">Failed to load Karma</div>;
    if (!karmas) return <div className="p-8 text-sonar-green flex items-center gap-2"><RefreshCw className="animate-spin" /> Digging into Samsara Memories...</div>;

    return (
        <div className="p-8 max-w-6xl mx-auto font-mono">
            <div className="flex justify-between items-center mb-8 border-b border-gray-800 pb-4">
                <h1 className="text-3xl font-bold tracking-tight text-white flex items-center gap-3">
                    <Database className="text-purple-500" />
                    Karma Viewer
                </h1>
                <button
                    onClick={() => mutate()}
                    className="p-2 bg-gray-800/50 hover:bg-purple-500/20 rounded text-purple-400 transition flex items-center gap-2"
                >
                    <RefreshCw size={16} className={clsx(isValidating && "animate-spin")} />
                    Sync
                </button>
            </div>

            <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6 overflow-y-auto max-h-[calc(100vh-160px)] pb-20">
                {karmas.map((karma: Karma) => (
                    <div
                        key={karma.id}
                        className="flex flex-col bg-gray-900 border border-gray-800 rounded-lg p-5 hover:border-purple-500/50 transition duration-300 relative group overflow-hidden"
                    >
                        {/* Holographic accent */}
                        <div className="absolute inset-0 bg-gradient-to-br from-purple-500/5 to-transparent opacity-0 group-hover:opacity-100 transition-opacity"></div>

                        <div className="flex items-start justify-between mb-4 relative">
                            <span className="bg-gray-800 text-purple-400 text-xs px-2 py-1 rounded-md tracking-widest font-bold">
                                {karma.skill_id || "global"}
                            </span>
                            <span className="text-gray-500 text-xs font-mono">{new Date(karma.created_at).toLocaleDateString()}</span>
                        </div>

                        <div className="mb-4 text-gray-200 text-sm leading-relaxed flex-grow relative">
                            <Lightbulb className="inline-block mr-2 text-yellow-500 mb-1" size={16} />
                            {karma.lesson}
                        </div>

                        <div className="flex items-center justify-between text-xs text-gray-500 mt-auto border-t border-gray-800/50 pt-3 relative">
                            <span className="flex items-center gap-1">
                                <Layers size={12} /> Weight: {karma.weight}
                            </span>
                            <span className={clsx(
                                "px-2 py-0.5 rounded",
                                karma.karma_type === "Synthesized" ? "bg-purple-500/10 text-purple-400 border border-purple-500/20" : "bg-gray-800"
                            )}>
                                {karma.karma_type}
                            </span>
                        </div>

                        <div className="text-[10px] text-gray-600 truncate mt-2">
                            Soul: {karma.soul_version_hash || "legacy"}
                        </div>
                    </div>
                ))}

                {karmas.length === 0 && (
                    <div className="col-span-full py-20 text-center text-gray-500 flex flex-col items-center">
                        <Database size={48} className="mb-4 opacity-50" />
                        <p>No Karma has been distilled yet.</p>
                        <p className="text-sm">Run 'SamsaraNow' to synthesize new knowledge.</p>
                    </div>
                )}
            </div>
        </div>
    );
};
