import React, { useMemo } from 'react';
import { renderMermaidSVG } from 'beautiful-mermaid';
import { AlertTriangle } from 'lucide-react';

interface MermaidRendererProps {
    code: string;
    className?: string;
}

/**
 * Strip executable content from SVG to prevent XSS via user-controlled Mermaid code.
 * Removes <script> tags and on* event handler attributes.
 */
function sanitizeSVG(raw: string): string {
    return raw
        .replace(/<script[\s\S]*?<\/script>/gi, '')
        .replace(/<script[\s\S]*?\/>/gi, '')
        .replace(/\s+on\w+\s*=\s*["'][^"']*["']/gi, '');
}

export const MermaidRenderer: React.FC<MermaidRendererProps> = ({ code, className }) => {
    const { svg, error } = useMemo(() => {
        try {
            // Apply tokens.css theme variables
            const rendered = renderMermaidSVG(code.trim(), {
                transparent: true,
                bg: 'var(--bg-app)',
                fg: 'var(--text-primary)',
                line: 'var(--border-glass)',
                accent: 'var(--accent-cyan)',
                muted: 'var(--text-muted)',
                surface: 'var(--black-20)',
                border: 'var(--border-glass)'
            });
            return { svg: sanitizeSVG(rendered), error: null };
        } catch (err: any) {
            console.error("Mermaid parsing failed:", err);
            return { svg: null, error: err.message || 'Unknown error' };
        }
    }, [code]);

    if (error) {
        return (
            <div className={`mermaid-error ${className || ''}`} style={{
                padding: '1rem',
                background: 'var(--accent-rose-glass)',
                border: '1px solid var(--accent-rose-30)',
                borderRadius: 'var(--radius-md)',
                color: 'var(--accent-rose)',
                display: 'flex',
                alignItems: 'flex-start',
                gap: '0.75rem',
                fontSize: '0.85rem'
            }}>
                <AlertTriangle size={16} style={{ flexShrink: 0, marginTop: '2px' }} />
                <div>
                    <div style={{ fontWeight: 600, marginBottom: '0.25rem' }}>Failed to render diagram</div>
                    <pre style={{ margin: 0, whiteSpace: 'pre-wrap', wordBreak: 'break-all', opacity: 0.8 }}>{error}</pre>
                </div>
            </div>
        );
    }

    return (
        <div 
            className={`mermaid-wrapper ${className || ''}`}
            style={{ 
                margin: '1.5rem 0',
                display: 'flex',
                justifyContent: 'center',
                overflowX: 'auto',
                background: 'var(--bg-glass)',
                padding: '1rem',
                borderRadius: 'var(--radius-lg)',
                border: '1px solid var(--border-glass)'
            }}
            // biome-ignore lint/security/noDangerouslySetInnerHtml: Trusted SVG output from beautiful-mermaid
            dangerouslySetInnerHTML={{ __html: svg as string }} 
        />
    );
};
