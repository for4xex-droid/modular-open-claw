/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import React from 'react';
import { useTranslation } from '../i18n';

const Timeline = React.lazy(() => import('./Timeline'));
const DiagnosticsHistory = React.lazy(() => import('./DiagnosticsHistory'));
const PromptStatsView = React.lazy(() => import('./PromptStatsView'));

export type ActivityTab = 'timeline' | 'audit' | 'usage';

/**
 * U6-5: 「きろく系」3画面（karma / audit / prompt-stats）の統合ビュー。
 * サイドバーには「アクティビティ」1項目のみを出し、内部タブで切り替える。
 * A2UI の navigate:audit / navigate:prompt-stats は initialTab 経由で互換維持。
 */
const ActivityView: React.FC<{ initialTab?: ActivityTab }> = ({ initialTab = 'timeline' }) => {
    const { t } = useTranslation();
    const [tab, setTab] = React.useState<ActivityTab>(initialTab);

    React.useEffect(() => {
        setTab(initialTab);
    }, [initialTab]);

    const tabs: { id: ActivityTab; labelKey: string }[] = [
        { id: 'timeline', labelKey: 'activity.tab.timeline' },
        { id: 'audit', labelKey: 'activity.tab.audit' },
        { id: 'usage', labelKey: 'activity.tab.usage' },
    ];

    return (
        <div style={{ display: 'flex', flexDirection: 'column', gap: '1rem', height: '100%' }}>
            <div role="tablist" style={{ display: 'flex', gap: '0.3rem', background: 'var(--white-05)', padding: '4px', borderRadius: 'var(--radius-md)', alignSelf: 'flex-start' }}>
                {tabs.map(({ id, labelKey }) => (
                    <button
                        key={id}
                        role="tab"
                        aria-selected={tab === id}
                        data-testid={`activity-tab-${id}`}
                        onClick={() => setTab(id)}
                        style={{
                            padding: '0.4rem 1rem',
                            borderRadius: 'var(--radius-sm)',
                            border: 'none',
                            cursor: 'pointer',
                            fontSize: '0.85rem',
                            fontWeight: 600,
                            background: tab === id ? 'var(--white-10)' : 'transparent',
                            color: tab === id ? 'var(--text-primary)' : 'var(--text-secondary)',
                            transition: 'background var(--speed-fast, 0.15s), color var(--speed-fast, 0.15s)',
                        }}
                    >
                        {t(labelKey)}
                    </button>
                ))}
            </div>
            <div style={{ flex: 1, minHeight: 0 }}>
                <React.Suspense fallback={null}>
                    {tab === 'timeline' && <Timeline />}
                    {tab === 'audit' && <DiagnosticsHistory />}
                    {tab === 'usage' && <PromptStatsView />}
                </React.Suspense>
            </div>
        </div>
    );
};

export default ActivityView;
