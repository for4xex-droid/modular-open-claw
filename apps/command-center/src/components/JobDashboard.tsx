import { useState } from 'react';
import useSWR from 'swr';
import { RefreshCw, CheckCircle, XCircle, Clock } from 'lucide-react';
import { clsx } from 'clsx';

const fetcher = (url: string) => fetch(url).then(r => r.json());

interface Job {
    id: string;
    topic: string;
    status: string;
    created_at: string;
    started_at: string | null;
    completed_at: string | null;
    creative_rating: number | null;
}

export const JobDashboard = () => {
    const { data: jobs, error, mutate, isValidating } = useSWR<Job[]>('http://localhost:3000/api/jobs', fetcher, {
        refreshInterval: 5000
    });

    const [ratingLoading, setRatingLoading] = useState<string | null>(null);

    const handleRate = async (id: string, rating: number) => {
        setRatingLoading(id);
        try {
            await fetch(`http://localhost:3000/api/jobs/${id}/rate`, {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ rating }),
            });
            mutate();
        } catch (err) {
            console.error(err);
        } finally {
            setRatingLoading(null);
        }
    };

    if (error) return <div className="p-8 text-red-500">Failed to load jobs</div>;
    if (!jobs) return <div className="p-8 text-sonar-green font-mono flex items-center gap-2"><RefreshCw className="animate-spin" /> Fetching Jobs...</div>;

    return (
        <div className="p-8 max-w-6xl mx-auto font-mono">
            <div className="flex justify-between items-center mb-8 border-b border-gray-800 pb-4">
                <h1 className="text-3xl font-bold tracking-tight text-white flex items-center gap-3">
                    <Clock className="text-sonar-green" />
                    Job Dashboard
                </h1>
                <button
                    onClick={() => mutate()}
                    className="p-2 bg-gray-800/50 hover:bg-sonar-green/20 rounded text-sonar-green transition flex items-center gap-2"
                >
                    <RefreshCw size={16} className={clsx(isValidating && "animate-spin")} />
                    Refresh
                </button>
            </div>

            <div className="bg-gray-900 border border-gray-800 rounded-lg overflow-hidden flex flex-col h-[calc(100vh-140px)]">
                <div className="overflow-y-auto w-full">
                    <table className="w-full text-left border-collapse">
                        <thead className="sticky top-0 bg-gray-900 z-10 border-b border-gray-800 tracking-wider text-gray-400 text-xs uppercase">
                            <tr>
                                <th className="p-4">Status</th>
                                <th className="p-4">Topic / ID</th>
                                <th className="p-4">Created Time</th>
                                <th className="p-4">Creative Rating (0-100)</th>
                            </tr>
                        </thead>
                        <tbody className="divide-y divide-gray-800/50">
                            {jobs.map((job: Job) => (
                                <tr key={job.id} className="hover:bg-gray-800/20 transition-colors">
                                    <td className="p-4">
                                        <StatusBadge status={job.status} />
                                    </td>
                                    <td className="p-4 max-w-xs truncate">
                                        <div className="text-gray-200 font-semibold mb-1">{job.topic}</div>
                                        <div className="text-gray-500 text-xs">{job.id.substring(0, 8)}...</div>
                                    </td>
                                    <td className="p-4 text-gray-400 text-sm">
                                        {new Date(job.created_at).toLocaleString()}
                                    </td>
                                    <td className="p-4">
                                        {job.status === "Completed" || job.status === "Processing" ? (
                                            <div className="flex items-center gap-2">
                                                <span className={clsx("font-bold text-lg w-8", job.creative_rating ? "text-sonar-green" : "text-gray-500")}>
                                                    {job.creative_rating !== null ? job.creative_rating : '-'}
                                                </span>
                                                <input
                                                    type="range"
                                                    min="0" max="100"
                                                    defaultValue={job.creative_rating || 50}
                                                    disabled={ratingLoading === job.id}
                                                    onMouseUp={(e) => handleRate(job.id, Number(e.currentTarget.value))}
                                                    className="accent-sonar-green cursor-pointer opacity-50 hover:opacity-100 transition"
                                                />
                                            </div>
                                        ) : (
                                            <span className="text-gray-600 text-sm italic">Not applicable yet</span>
                                        )}
                                    </td>
                                </tr>
                            ))}
                            {jobs.length === 0 && (
                                <tr>
                                    <td colSpan={4} className="p-8 text-center text-gray-500">
                                        No jobs recorded.
                                    </td>
                                </tr>
                            )}
                        </tbody>
                    </table>
                </div>
            </div>
        </div>
    );
};

const StatusBadge = ({ status }: { status: string }) => {
    switch (status) {
        case 'Pending':
            return <div className="inline-flex items-center gap-1.5 px-2 py-1 rounded bg-orange-500/10 text-orange-400 text-xs border border-orange-500/20"><Clock size={12} /> Pending</div>;
        case 'Processing':
            return <div className="inline-flex items-center gap-1.5 px-2 py-1 rounded bg-blue-500/10 text-blue-400 text-xs border border-blue-500/20"><RefreshCw size={12} className="animate-spin" /> Processing</div>;
        case 'Completed':
            return <div className="inline-flex items-center gap-1.5 px-2 py-1 rounded bg-sonar-green/10 text-sonar-green text-xs border border-sonar-green/20"><CheckCircle size={12} /> Completed</div>;
        case 'Failed':
            return <div className="inline-flex items-center gap-1.5 px-2 py-1 rounded bg-red-500/10 text-red-500 text-xs border border-red-500/20"><XCircle size={12} /> Failed</div>;
        default:
            return <span className="text-gray-500 text-xs">{status}</span>;
    }
};
