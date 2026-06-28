/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import { useMemo } from 'react';
import { cssVar } from '../utils/cssVar';

export const useGraphTheme = () => {
    return useMemo(() => {
        const cyan = cssVar('--accent-cyan');
        const purple = cssVar('--accent-purple');
        const rose = cssVar('--accent-rose');
        const text = cssVar('--text-primary');
        const textMuted = cssVar('--text-muted');

        return {
            nodes: {
                karmaLocal: {
                    background: cssVar('--accent-cyan-15'),
                    border: cyan,
                    highlight: {
                        background: cssVar('--accent-cyan-30'),
                        border: text
                    }
                },
                karmaForeign: {
                    background: cssVar('--accent-purple-15'),
                    border: purple,
                    highlight: {
                        background: cssVar('--accent-purple-30'),
                        border: text
                    }
                },
                artifact: {
                    background: cssVar('--accent-rose-15'),
                    border: rose,
                    highlight: {
                        background: cssVar('--accent-rose-30'),
                        border: text
                    },
                    font: rose
                }
            },
            edges: {
                default: {
                    color: cssVar('--white-10'),
                    highlight: cyan
                },
                materialized: {
                    color: cssVar('--accent-rose-20')
                }
            },
            shadow: cssVar('--black-50'),
            text,
            textMuted,
            background: {
                gradientInner: cssVar('--bg-glass-heavy'),
                gradientOuter: cssVar('--bg-primary')
            }
        };
    }, []);
};
