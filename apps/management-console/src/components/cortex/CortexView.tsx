import { useState, useEffect } from 'react';
import ReactMarkdown from 'react-markdown';
import rehypeSanitize from 'rehype-sanitize';

import { components } from '../../types/generated';
import { authenticatedFetch } from '../../lib/auth';
import { API_BASE } from '../../config';

type WikiArticleSummary = components['schemas']['WikiArticleSummary'];
type WikiArticleDetail = components['schemas']['WikiArticle'];
import { useTranslation } from '../../i18n';
import TrendView from './TrendView';

export default function CortexView() {
  const { t } = useTranslation();
  const [articles, setArticles] = useState<WikiArticleSummary[]>([]);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [detail, setDetail] = useState<WikiArticleDetail | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const controller = new AbortController();
    const fetchArticles = async () => {
      try {
        setLoading(true);
        setError(null);
        const res = await authenticatedFetch(`${API_BASE}/api/v1/cortex/wiki`, {
          signal: controller.signal,
        });
        if (!res.ok) throw new Error(`API Error: ${res.status}`);
        const data = await res.json();
        if (Array.isArray(data)) {
          setArticles(data);
        }
      } catch (e) {
        if (e instanceof DOMException && e.name === 'AbortError') return;
        console.error('Failed to fetch wiki articles', e);
        setError(t('cortexView.loadIndexFailed'));
      } finally {
        setLoading(false);
      }
    };
    fetchArticles();
    return () => controller.abort();
  }, []);

  useEffect(() => {
    if (!selectedId) {
      setDetail(null);
      return;
    }
    const controller = new AbortController();
    const fetchDetail = async () => {
      try {
        setLoading(true);
        setError(null);
        const safePath = encodeURIComponent(selectedId);
        const res = await authenticatedFetch(`${API_BASE}/api/v1/cortex/wiki/${safePath}`, {
          signal: controller.signal,
        });
        if (!res.ok) throw new Error(`API Error: ${res.status}`);
        const data = await res.json();
        setDetail(data);
      } catch (e) {
        if (e instanceof DOMException && e.name === 'AbortError') return;
        console.error('Failed to fetch detail', e);
        setError(t('cortexView.retrievalFailed'));
      } finally {
        setLoading(false);
      }
    };
    fetchDetail();
    return () => controller.abort();
  }, [selectedId]);

  return (
    <div style={{ display: 'grid', gridTemplateColumns: 'minmax(250px, 1fr) minmax(250px, 1fr) minmax(400px, 2fr)', gap: 'var(--layout-panel-gap)', height: 'calc(100vh - 100px)' }}>
      {/* Trend View Panel */}
      <div className="main-panel ani-fade" style={{ padding: 'var(--space-md)' }}>
        <TrendView />
      </div>

      {/* Wiki List Panel */}
      <div className="main-panel ani-fade" style={{ display: 'flex', flexDirection: 'column', gap: 'var(--space-sm)', overflowY: 'auto', padding: 'var(--space-md)' }}>
        <h3 style={{ fontSize: '1.2rem', fontWeight: 600, color: 'var(--text-primary)' }}>{t('cortexView.knowledgeIndex')}</h3>

        {loading && !articles.length && (
          <div className="ani-pulse" style={{ color: 'var(--accent-cyan)' }}>{t('cortexView.scanningIndex')}</div>
        )}

        {!loading && !articles.length && !error && (
          <div style={{ color: 'var(--text-muted)', fontSize: '0.85rem', padding: 'var(--space-sm)' }}>
            {t('cortexView.noArticles')}
          </div>
        )}

        <div style={{ display: 'flex', flexDirection: 'column', gap: 'var(--space-xs)' }}>
          {articles.map(article => (
            <div
              key={article.id}
              onClick={() => setSelectedId(article.id)}
              className="stat-card"
              style={{
                padding: 'var(--space-sm)',
                background: selectedId === article.id ? 'var(--accent-cyan-glass)' : 'var(--bg-glass-light)',
                border: selectedId === article.id ? '1px solid var(--accent-cyan)' : '1px solid var(--border-glass)',
                cursor: 'pointer',
                transition: 'all 0.2s',
                boxShadow: selectedId === article.id ? 'var(--glow-cyan)' : 'none',
              }}
            >
              <div style={{ fontWeight: 600, color: selectedId === article.id ? 'var(--accent-cyan)' : 'var(--text-primary)' }}>
                {article.title}
              </div>
              <div style={{ fontSize: '0.75rem', marginTop: 'var(--space-xs)', color: 'var(--text-muted)' }}>
                {new Date(article.updated_at).toLocaleString()} • {article.concepts?.join(', ')}
              </div>
            </div>
          ))}
        </div>
      </div>

      {/* Detail View Panel */}
      <div className="main-panel ani-slide-right" style={{ display: 'flex', flexDirection: 'column', padding: 'var(--space-lg)', overflowY: 'auto' }}>
        {error && (
          <div style={{
            padding: 'var(--space-sm)',
            background: 'rgba(var(--accent-rose-rgb), 0.1)',
            border: '1px solid var(--accent-rose)',
            color: 'var(--accent-rose)',
            borderRadius: 'var(--radius-sm)',
            marginBottom: 'var(--space-sm)',
          }}>
            {error}
          </div>
        )}

        {loading && selectedId && !detail && (
          <div className="ani-pulse" style={{ color: 'var(--accent-cyan)' }}>{t('cortexView.synchronizingThoughts')}</div>
        )}

        {detail ? (
          <div className="markdown-content">
            <h1 style={{ fontSize: '1.8rem', fontWeight: 800, marginBottom: 'var(--space-xs)', color: 'var(--accent-purple)' }}>
              {detail.title}
            </h1>

            {detail.concepts && detail.concepts.length > 0 && (
              <div style={{ display: 'flex', gap: 'var(--space-xs)', flexWrap: 'wrap', marginBottom: 'var(--space-md)' }}>
                {detail.concepts.map(c => (
                  <span key={c} style={{
                    background: 'var(--accent-cyan-glass)',
                    color: 'var(--accent-cyan)',
                    border: '1px solid var(--accent-cyan)',
                    padding: '0.2rem 0.6rem',
                    borderRadius: 'var(--radius-md)',
                    fontSize: '0.75rem',
                    fontWeight: 600,
                  }}>{c}</span>
                ))}
              </div>
            )}

            <div style={{
              background: 'var(--bg-glass-heavy)',
              padding: 'var(--space-md)',
              borderRadius: 'var(--radius-md)',
              border: '1px solid var(--border-glass)',
              lineHeight: 1.6,
            }}>
              <ReactMarkdown
                rehypePlugins={[rehypeSanitize]}
                components={{
                  h1: ({node, ...props}) => <h1 style={{color: 'var(--accent-cyan)', fontSize: '1.5em', marginTop: '1em'}} {...props} />,
                  h2: ({node, ...props}) => <h2 style={{color: 'var(--accent-purple)', fontSize: '1.2em', marginTop: '1em'}} {...props} />,
                  a: ({node, ...props}) => <a style={{color: 'var(--accent-cyan)'}} {...props} />,
                }}
              >
                {detail.content_md}
              </ReactMarkdown>
            </div>

            {detail.backlinks && detail.backlinks.length > 0 && (
              <div style={{
                marginTop: 'var(--space-lg)',
                padding: 'var(--space-sm)',
                background: 'var(--bg-glass-light)',
                borderRadius: 'var(--radius-sm)',
              }}>
                <h4 style={{ color: 'var(--text-muted)', marginBottom: 'var(--space-xs)', fontSize: '0.8rem' }}>{t('cortexView.linkedConcepts')}</h4>
                <div style={{ display: 'flex', gap: 'var(--space-xs)', flexWrap: 'wrap' }}>
                  {detail.backlinks.map(c => (
                    <span key={c} style={{
                      background: 'var(--accent-purple-glass)',
                      color: 'var(--accent-purple)',
                      border: '1px solid var(--accent-purple)',
                      padding: '0.2rem 0.5rem',
                      borderRadius: 'var(--radius-md)',
                      fontSize: '0.7rem',
                    }}>{c}</span>
                  ))}
                </div>
              </div>
            )}
          </div>
        ) : (
          !error && !loading && (
            <div style={{ display: 'flex', height: '100%', alignItems: 'center', justifyContent: 'center', color: 'var(--text-muted)' }}>
              {t('cortexView.selectArticlePrompt')}
            </div>
          )
        )}
      </div>
    </div>
  );
}
