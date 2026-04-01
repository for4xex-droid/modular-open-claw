/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import React, { useState } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { Sparkles, ExternalLink, Info, Loader2, RefreshCw } from 'lucide-react';
import { useTreasure } from '../hooks/useTreasure';
import { TreasureItem } from '../types';

export const TreasureBox: React.FC = () => {
    const { items, loading, error, refresh, recordFeedback } = useTreasure();
    const [showEffect, setShowEffect] = useState(false);

    const handleClick = async (item: TreasureItem) => {
        // Record feedback (click)
        const success = await recordFeedback(item.id, 'click');
        if (success) {
            // Visual effect for Resonance increase
            setShowEffect(true);
            setTimeout(() => setShowEffect(false), 2000);
        }
        
        // Open URL in new window
        window.open(item.url, '_blank', 'noopener,noreferrer');
    };

    return (
        <div className="bg-white/5 backdrop-blur-xl border border-white/10 rounded-2xl p-6 shadow-2xl relative overflow-hidden">
            {/* Header */}
            <div className="flex items-center justify-between mb-6">
                <div className="flex items-center gap-2">
                    <div className="p-2 bg-indigo-500/20 rounded-lg">
                        <Sparkles className="w-5 h-5 text-indigo-400" />
                    </div>
                    <div>
                        <h2 className="text-lg font-bold text-white tracking-tight">エージェントの感覚 (Sense)</h2>
                        <p className="text-xs text-indigo-300/60 leading-none">AIが惹かれた「宝箱」</p>
                    </div>
                </div>
                <button 
                    onClick={() => refresh()} 
                    disabled={loading}
                    className="p-2 hover:bg-white/10 rounded-full transition-colors disabled:opacity-50"
                >
                    <RefreshCw className={`w-4 h-4 text-white/50 ${loading ? 'animate-spin' : ''}`} />
                </button>
            </div>

            {/* Error State */}
            {error && (
                <div className="p-4 bg-red-500/10 border border-red-500/20 rounded-xl text-red-400 text-sm mb-4">
                    {error}
                </div>
            )}

            {/* Loading State */}
            {loading && items.length === 0 && (
                <div className="flex flex-col items-center justify-center py-12 gap-3">
                    <Loader2 className="w-8 h-8 text-indigo-400 animate-spin" />
                    <p className="text-sm text-white/40">Senseを調律中...</p>
                </div>
            )}

            {/* Items Grid */}
            <div className="grid grid-cols-1 gap-4">
                <AnimatePresence mode="popLayout">
                    {items.map((item, index) => (
                        <motion.div
                            key={item.id}
                            initial={{ opacity: 0, y: 20 }}
                            animate={{ opacity: 1, y: 0 }}
                            exit={{ opacity: 0, scale: 0.95 }}
                            transition={{ delay: index * 0.1 }}
                            whileHover={{ scale: 1.02 }}
                            whileTap={{ scale: 0.98 }}
                            onClick={() => handleClick(item)}
                            className="group relative bg-white/5 hover:bg-white/10 border border-white/10 hover:border-indigo-500/30 rounded-xl p-4 cursor-pointer transition-all duration-300"
                        >
                            {/* Compliance Label (AS-1.6) */}
                            <div className="absolute top-2 right-3 px-1.5 py-0.5 bg-indigo-500/10 border border-indigo-500/20 rounded text-[10px] text-indigo-300 font-medium tracking-wider uppercase">
                                {item.disclosure_label}
                            </div>

                            <div className="flex flex-col gap-1">
                                <div className="flex items-center gap-2 pr-12">
                                    <span className="text-sm font-semibold text-white group-hover:text-indigo-300 transition-colors line-clamp-1">
                                        {item.title}
                                    </span>
                                    <ExternalLink className="w-3 h-3 text-white/30" />
                                </div>
                                <p className="text-xs text-white/50 line-clamp-2 leading-relaxed h-8">
                                    {item.description}
                                </p>
                                <div className="mt-2 flex items-center justify-between">
                                    <span className="text-[10px] px-2 py-0.5 bg-white/5 rounded-full text-white/40">
                                        {item.category}
                                    </span>
                                    {item.price_coins && (
                                        <span className="text-xs font-mono text-indigo-400 font-bold">
                                            {item.price_coins} <span className="text-[10px] text-white/30">COINS</span>
                                        </span>
                                    )}
                                </div>
                            </div>
                        </motion.div>
                    ))}
                </AnimatePresence>
            </div>

            {/* Empty State */}
            {!loading && items.length === 0 && !error && (
                <div className="text-center py-12 opacity-30 select-none">
                    <Info className="w-8 h-8 mx-auto mb-2" />
                    <p className="text-sm">まだ何も見つかりませんでした</p>
                </div>
            )}

            {/* Resonance Effect Overlay */}
            <AnimatePresence>
                {showEffect && (
                    <motion.div 
                        initial={{ opacity: 0, scale: 0.5, y: 0 }}
                        animate={{ opacity: 1, scale: 1, y: -50 }}
                        exit={{ opacity: 0 }}
                        className="absolute inset-0 flex items-center justify-center pointer-events-none z-50"
                    >
                        <div className="px-4 py-2 bg-indigo-500 text-white font-bold rounded-full shadow-lg shadow-indigo-500/50 flex items-center gap-2">
                            <Sparkles className="w-4 h-4" />
                            <span>共鳴度 +5!</span>
                        </div>
                    </motion.div>
                )}
            </AnimatePresence>
        </div>
    );
};
