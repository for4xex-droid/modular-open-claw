import { useMemo } from 'react';
import { cssVar } from '../utils/cssVar';

export const useGraphTheme = () => {
    return useMemo(() => {
        const cyan = cssVar('--accent-cyan', '#00f2ff');
        const purple = cssVar('--accent-purple', '#bc8cff');
        const rose = cssVar('--accent-rose', '#ff4d94');
        const text = cssVar('--text-primary', '#ffffff');
        const textMuted = cssVar('--text-muted', '#888');

        return {
            nodes: {
                karmaLocal: {
                    background: cssVar('--accent-cyan-15', 'rgba(0, 242, 255, 0.15)'),
                    border: cyan,
                    highlight: {
                        background: cssVar('--accent-cyan-30', 'rgba(0, 242, 255, 0.3)'),
                        border: text
                    }
                },
                karmaForeign: {
                    background: cssVar('--accent-purple-15', 'rgba(188, 140, 255, 0.15)'),
                    border: purple,
                    highlight: {
                        background: cssVar('--accent-purple-30', 'rgba(188, 140, 255, 0.3)'),
                        border: text
                    }
                },
                artifact: {
                    background: cssVar('--accent-rose-15', 'rgba(255, 77, 148, 0.15)'),
                    border: rose,
                    highlight: {
                        background: cssVar('--accent-rose-30', 'rgba(255, 77, 148, 0.3)'),
                        border: text
                    },
                    font: rose
                }
            },
            edges: {
                default: {
                    color: cssVar('--white-10', 'rgba(255, 255, 255, 0.1)'),
                    highlight: cyan
                },
                materialized: {
                    color: cssVar('--accent-rose-20', 'rgba(255, 77, 148, 0.2)')
                }
            },
            shadow: cssVar('--black-50', 'rgba(0, 0, 0, 0.5)'),
            text,
            textMuted,
            background: {
                gradientInner: cssVar('--bg-glass-heavy', 'rgba(16, 20, 28, 0.8)'),
                gradientOuter: cssVar('--bg-primary', '#0b0b0f')
            }
        };
    }, []);
};
