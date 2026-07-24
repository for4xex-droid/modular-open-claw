/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import React, { useState } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import AiomeAvatar from '../AiomeAvatar';
import VrmRenderer from '../../lib/vrm/VrmRenderer';
import ErrorBoundary from '../common/ErrorBoundary';
import { useAvatarCharacter } from '../../hooks/AvatarContext';
import { isDioramaVisible } from '../../lib/dioramaVisibleTabs';
import type { DisplayMode } from '../../hooks/useDisplayMode';

interface DioramaViewProps {
    status: 'idle' | 'thinking' | 'speaking' | 'learning' | 'meditating' | 'awakened';
    mode: DisplayMode;
    activeTab: string;
}

const containerStyleBase: React.CSSProperties = {
    position: 'fixed',
    top: 0,
    bottom: 0,
    zIndex: 0,
    pointerEvents: 'none',
    transform: 'translateY(11vh)',
};

const DioramaView: React.FC<DioramaViewProps> = ({ status, mode, activeTab }) => {
    const [hasError, setHasError] = useState(false);
    const { getAssetPath } = useAvatarCharacter();
    const modelUrl = mode === 'vrm' ? getAssetPath('vrm') : '';

    const isDashboard = activeTab === 'dashboard';
    const leftOffset = 'calc(var(--layout-sidebar-width) + var(--layout-main-padding))';
    const rightOffset = isDashboard
        ? 'calc(var(--layout-main-padding) + var(--layout-right-panel-width) + var(--layout-panel-gap))'
        : 'var(--layout-main-padding)';

    React.useEffect(() => {
        setHasError(false);
    }, [mode]);

    const hidden = mode === 'off' || !isDioramaVisible(activeTab);

    const containerStyle: React.CSSProperties = {
        ...containerStyleBase,
        left: leftOffset,
        right: rightOffset,
    };

    const renderAvatar = () => {
        if (mode === 'lite' || hasError) {
            const liteStatus: 'idle' | 'thinking' | 'awakened' =
                (status === 'thinking' || status === 'learning' || status === 'speaking') ? 'thinking' :
                    (status === 'awakened') ? 'awakened' : 'idle';
            return (
                <div style={{ ...containerStyle, display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
                    <AiomeAvatar status={liteStatus} size={400} />
                </div>
            );
        }

        return (
            <div style={{ ...containerStyle, overflow: 'hidden' }}>
                <ErrorBoundary
                    fallback={null}
                    onError={() => {
                        console.error('Canvas crash detected, falling back to lite mode');
                        setHasError(true);
                    }}
                >
                    {mode === 'vrm' && (
                        <VrmRenderer
                            modelUrl={modelUrl}
                            avatarState={status}
                        />
                    )}
                </ErrorBoundary>
            </div>
        );
    };

    return (
        <AnimatePresence>
            {!hidden && (
                <motion.div
                    key="diorama-overlay"
                    initial={{ opacity: 0 }}
                    animate={{ opacity: 1 }}
                    exit={{ opacity: 0 }}
                    transition={{ duration: 0.2 }}
                    style={{ pointerEvents: 'none' }}
                >
                    {renderAvatar()}
                </motion.div>
            )}
        </AnimatePresence>
    );
};

export default DioramaView;
