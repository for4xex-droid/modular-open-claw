/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import React, { useMemo } from 'react';
import { useSystemVitality } from '../hooks/useSystemVitality';
import { SoTEvent } from '../types';

export const SoTProgressBar: React.FC = () => {
    const { events } = useSystemVitality();
    
    // We only care about the latest active Session or recently ended one.
    // Instead of using effect and state which causes double-renders, we use derived state.
    const currentSession = useMemo(() => {
        let activeSession: {
            id: string;
            roles: string[];
            currentRound: number;
            status: 'active' | 'ended';
            scores: [string, number][];
            events: SoTEvent[];
        } | null = null;
        
        // Find all SoT events and sort oldest first to reconstruct chronological state
        const sotEvents = events
            .filter(e => e.type === 'sot_progress')
            .map(e => e.data as SoTEvent)
            .reverse();

        for (const se of sotEvents) {
            const { type, data } = se.event;
            switch(type) {
                case 'SessionStart':
                    if (!activeSession || activeSession.id !== data.session_id) {
                        activeSession = {
                            id: data.session_id,
                            roles: [],
                            currentRound: 0,
                            status: 'active',
                            scores: [],
                            events: [se]
                        };
                    }
                    break;
                case 'RoleStart':
                case 'RoleOutput':
                    if (!activeSession) {
                        // Resiliency: Fallback if we connected mid-session
                        activeSession = {
                            id: data.session_id,
                            roles: [],
                            currentRound: data.round,
                            status: 'active',
                            scores: [],
                            events: []
                        };
                    }
                    if (activeSession.id === data.session_id) {
                        activeSession.currentRound = data.round;
                        if (type === 'RoleStart' && !activeSession.roles.includes(data.role)) {
                            activeSession.roles.push(data.role);
                        }
                    }
                    break;
                case 'Score':
                    if (activeSession && activeSession.id === data.session_id) {
                        activeSession.scores = data.scores;
                    }
                    break;
                case 'SessionEnd':
                    if (activeSession && activeSession.id === data.session_id) {
                        activeSession.status = 'ended';
                    }
                    break;
            }
        }
        
        return activeSession;
    }, [events]);

    if (!currentSession || currentSession.status === 'ended') {
        return null;
    }

    return (
        <div className="fixed bottom-20 left-1/2 transform -translate-x-1/2 w-96 bg-gray-900/80 backdrop-blur border border-indigo-500/30 rounded-xl p-4 shadow-xl z-50 animate-in slide-in-from-bottom">
            <div className="flex items-center justify-between mb-2">
                <h3 className="text-sm font-bold text-indigo-300 flex items-center gap-2">
                    <span className="relative flex h-3 w-3">
                      <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-indigo-400 opacity-75"></span>
                      <span className="relative inline-flex rounded-full h-3 w-3 bg-indigo-500"></span>
                    </span>
                    Society of Thought Active
                </h3>
                <span className="text-xs text-gray-400 px-2 py-0.5 bg-gray-800 rounded">
                    Round {currentSession.currentRound}
                </span>
            </div>
            
            <div className="space-y-2">
                {currentSession.roles.map((role: string, idx: number) => {
                    const isFocus = idx === currentSession.roles.length - 1;
                    return (
                        <div key={role} className={`text-xs p-2 rounded border ${isFocus ? 'border-indigo-400 bg-indigo-500/10 text-indigo-200' : 'border-gray-700 bg-gray-800 text-gray-400'}`}>
                            {isFocus ? (
                                <div className="flex justify-between">
                                    <span>{role} is thinking...</span>
                                    <span className="animate-pulse">..</span>
                                </div>
                            ) : (
                                <span>{role} completed</span>
                            )}
                        </div>
                    );
                })}
            </div>
            
            {currentSession.scores.length > 0 && (
                <div className="mt-3 pt-2 border-t border-gray-700/50">
                    <div className="text-[10px] text-gray-500 mb-1 uppercase font-semibold">Latest Scores</div>
                    <div className="flex gap-2">
                        {currentSession.scores.map(([metric, score]: [string, number]) => (
                            <div key={metric} className={`text-xs px-1.5 py-0.5 rounded ${score >= 4 ? 'bg-green-500/20 text-green-300' : score >= 3 ? 'bg-yellow-500/20 text-yellow-300' : 'bg-red-500/20 text-red-300'}`}>
                                {metric}: {score}/5
                            </div>
                        ))}
                    </div>
                </div>
            )}
        </div>
    );
};
