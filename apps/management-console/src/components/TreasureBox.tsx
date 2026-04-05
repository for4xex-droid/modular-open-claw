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
import { useTranslation } from '../i18n';
import './TreasureBox.css';

export const TreasureBox: React.FC = () => {
    const { items, loading, error, refresh, recordFeedback } = useTreasure();
    const { t } = useTranslation();
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
        <div className="artemis-treasure-box">
            {/* Header */}
            <div className="artemis-treasure-header">
                <div className="artemis-treasure-header-left">
                    <div className="artemis-treasure-icon-wrap">
                        <Sparkles className="artemis-treasure-icon" />
                    </div>
                    <div>
                        <h2 className="artemis-heading artemis-treasure-title">{t('treasure.title')}</h2>
                        <p className="artemis-treasure-subtitle">{t('treasure.subtitle')}</p>
                    </div>
                </div>
                <button 
                    onClick={() => refresh()} 
                    disabled={loading}
                    className="artemis-treasure-refresh"
                >
                    <RefreshCw className={`artemis-treasure-refresh-icon ${loading ? 'animate-spin' : ''}`} />
                </button>
            </div>

            {/* Error State */}
            {error && (
                <div className="artemis-treasure-error">
                    {error}
                </div>
            )}

            {/* Loading State */}
            {loading && items.length === 0 && (
                <div className="artemis-treasure-loading">
                    <Loader2 className="artemis-treasure-loading-icon animate-spin" />
                    <p className="artemis-treasure-loading-text">{t('treasure.loading')}</p>
                </div>
            )}

            {/* Items Grid */}
            <div className="artemis-treasure-grid">
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
                            className="group artemis-treasure-item"
                        >
                            {/* Compliance Label (AS-1.6) */}
                            <div className="artemis-treasure-label">
                                {item.disclosure_label}
                            </div>

                            <div className="artemis-treasure-item-content">
                                <div className="artemis-treasure-item-header">
                                    <span className="artemis-treasure-item-title">
                                        {item.title}
                                    </span>
                                    <ExternalLink className="artemis-treasure-item-link-icon" />
                                </div>
                                <p className="artemis-treasure-item-desc">
                                    {item.description}
                                </p>
                                <div className="artemis-treasure-item-footer">
                                    <span className="artemis-treasure-item-category">
                                        {item.category}
                                    </span>
                                    {item.price_coins && (
                                        <span className="artemis-treasure-item-price">
                                            {item.price_coins} <span className="artemis-treasure-item-price-unit">COINS</span>
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
                <div className="artemis-treasure-empty">
                    <Info className="artemis-treasure-empty-icon" />
                    <p className="artemis-treasure-empty-text">{t('treasure.empty')}</p>
                </div>
            )}

            {/* Resonance Effect Overlay */}
            <AnimatePresence>
                {showEffect && (
                    <motion.div 
                        initial={{ opacity: 0, scale: 0.5, y: 0 }}
                        animate={{ opacity: 1, scale: 1, y: -50 }}
                        exit={{ opacity: 0 }}
                        className="artemis-treasure-overlay"
                    >
                        <div className="artemis-treasure-overlay-badge">
                            <Sparkles className="artemis-treasure-overlay-icon" />
                            <span>{t('treasure.resonance')}</span>
                        </div>
                    </motion.div>
                )}
            </AnimatePresence>
        </div>
    );
};
