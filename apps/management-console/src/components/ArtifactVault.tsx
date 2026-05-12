/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import { useState, useEffect, useRef } from "react";
import { motion, AnimatePresence } from "framer-motion";
import {
  Box,
  FileText,
  Code,
  Image as ImageIcon,
  Music,
  Share2,
  Database,
  Search,
  Download,
  Trash2,
  Calendar,
  User,
  Tag,
  Hash,
  Shield,
  Dna,
  X,
  Eye
} from "lucide-react";
import { API_BASE } from "../config";
import { authenticatedFetch } from "../lib/auth";
import ConfirmModal from './common/ConfirmModal';
import { useTranslation } from '../i18n';
import { useToast } from './common/Toast';

interface ArtifactFile {
  name: string;
  mime_type: string;
  size_bytes: number;
  hash: string;
}

interface ArtifactEdge {
  id: string;
  source_id: string;
  target_id: string;
  source_type: string;
  relation: string;
  metadata: any;
  created_at: string;
}

interface Artifact {
  id: string;
  title: string;
  category: string;
  tags: string[];
  created_by: string;
  dir_path: string;
  files: ArtifactFile[];
  karma_refs: string[];
  job_ref?: string;
  signature?: string;
  edges: ArtifactEdge[];
  created_at: string;
}

const ArtifactVault = () => {
  const { t } = useTranslation();
  const { showToast } = useToast();
  const [artifacts, setArtifacts] = useState<Artifact[]>([]);
  const [loading, setLoading] = useState(true);
  const [filter, setFilter] = useState<string | null>(null);
  const [searchTerm, setSearchTerm] = useState("");
  const [selectedArtifact, setSelectedArtifact] = useState<Artifact | null>(null);
  const [previewFile, setPreviewFile] = useState<{artifact: Artifact, file: ArtifactFile} | null>(null);
  const [deletingArtifactId, setDeletingArtifactId] = useState<string | null>(null);
  const iframeRef = useRef<HTMLIFrameElement>(null);

  useEffect(() => {
    const timer = setTimeout(() => {
      fetchArtifacts();
    }, 300);
    return () => clearTimeout(timer);
  }, [filter, searchTerm]);

  // Listen for JS bridge messages from the sandboxed iframe
  useEffect(() => {
    const handleMessage = (e: MessageEvent) => {
      // Security Check: Ensure the message is strictly from our sandboxed preview iframe
      // @ts-ignore
      const isTestEnv = typeof process !== 'undefined' && process.env.NODE_ENV === 'test';
      if (!isTestEnv && (!iframeRef.current || e.source !== iframeRef.current.contentWindow)) {
        return;
      }

      if (e.data && e.data.type === 'AIOME_PROMPT_FEEDBACK') {
        if (typeof e.data.payload === 'string') {
          window.dispatchEvent(new CustomEvent('aiome_inject_prompt', {
            detail: { prompt: e.data.payload, autoSend: !!e.data.autoSend }
          }));
        }
      }
    };
    window.addEventListener('message', handleMessage);
    return () => window.removeEventListener('message', handleMessage);
  }, []);

  const fetchArtifacts = async () => {
    setLoading(true);
    try {
      let url = `${API_BASE}/api/artifacts?limit=50`;
      if (filter) url += `&category=${encodeURIComponent(filter)}`;
      if (searchTerm) url += `&q=${encodeURIComponent(searchTerm)}`;

      const res = await authenticatedFetch(url);
      if (res.ok) {
        const data = await res.json();
        if (Array.isArray(data)) {
          setArtifacts(data);
        } else {
          console.error("Unexpected artifacts response format:", typeof data);
          setArtifacts([]);
        }
      }
    } catch (e) {
      console.error("Failed to fetch artifacts", e);
    } finally {
      setLoading(false);
    }
  };

  const getCategoryIcon = (category: string) => {
    switch (category) {
      case "report": return <FileText size={18} />;
      case "code": return <Code size={18} />;
      case "image": return <ImageIcon size={18} />;
      case "audio": return <Music size={18} />;
      case "expression": return <Share2 size={18} />;
      case "data": return <Database size={18} />;
      case "blueprint": return <Dna size={18} />;
      default: return <Box size={18} />;
    }
  };

  const executeDeleteArtifact = async () => {
    if (!deletingArtifactId) return;

    try {
      const res = await authenticatedFetch(`${API_BASE}/api/artifacts/${deletingArtifactId}`, {
        method: "DELETE"
      });
      if (res.ok) {
        setArtifacts(prev => prev.filter(a => a.id !== deletingArtifactId));
        if (selectedArtifact?.id === deletingArtifactId) setSelectedArtifact(null);
        showToast('success', t('artifact.deleteSuccess', { defaultValue: 'Artifact deleted successfully.' }));
        setDeletingArtifactId(null);
      } else {
        showToast('error', t('artifact.deleteFailed', { defaultValue: 'Failed to delete artifact.' }));
      }
    } catch (e) {
      console.error("Failed to delete artifact", e);
      showToast('error', t('artifact.deleteFailed', { defaultValue: 'Failed to delete artifact.' }));
    }
  };

  const handleDeleteRequest = (e: React.MouseEvent, id: string) => {
    e.stopPropagation();
    setDeletingArtifactId(id);
  };

  return (
    <div className="vault-container">
      <ConfirmModal
        isOpen={!!deletingArtifactId}
        type="danger"
        title={t('artifact.confirmDelete')}
        message={t('artifact.deleteMessage', { defaultValue: 'Are you sure you want to delete this artifact?' })}
        details={t('artifact.deleteDetails', { defaultValue: 'This action cannot be undone. Associated files will be permanently removed.' })}
        confirmText={t('common.delete', { defaultValue: 'Delete' })}
        onConfirm={executeDeleteArtifact}
        onCancel={() => setDeletingArtifactId(null)}
      />

      <div style={{ display: 'flex', gap: 'var(--space-md)', marginBottom: 'var(--space-lg)', alignItems: 'center' }}>
        <div className="search-box">
          <Search size={18} />
          <input
            type="text"
            placeholder={t('artifact.search')}
            value={searchTerm}
            onChange={(e) => setSearchTerm(e.target.value)}
          />
        </div>

        <div className="filter-chips">
          {['all', 'report', 'code', 'image', 'audio', 'expression', 'data', 'blueprint'].map((cat) => (
            <button
              key={cat}
              className={`chip ${cat === (filter || 'all') ? 'active' : ''}`}
              onClick={() => setFilter(cat === 'all' ? null : cat)}
            >
              {cat === 'all' ? 'All' : cat.charAt(0).toUpperCase() + cat.slice(1)}
            </button>
          ))}
        </div>
      </div>

      {loading ? (
        <div style={{ padding: 'var(--space-xl)', textAlign: 'center' }}>
          <Box className="ani-pulse" size={48} color="var(--accent-cyan)" style={{ margin: '0 auto 1.5rem' }} />
          <p style={{ color: 'var(--text-secondary)' }}>{t('artifact.decrypting')}</p>
        </div>
      ) : (
        <div className="artifact-grid">
          {artifacts.map((artifact) => (
            <motion.div
              key={artifact.id}
              layoutId={artifact.id}
              className="artifact-card"
              onClick={() => setSelectedArtifact(artifact)}
            >
              <div className="card-header">
                <div className="category-tag">
                  {getCategoryIcon(artifact.category)}
                  <span>{artifact.category.toUpperCase()}</span>
                </div>
                <div className="timestamp">
                  {new Date(artifact.created_at).toLocaleDateString()}
                </div>
              </div>

              <h3 className="card-title">{artifact.title}</h3>

              <div className="card-meta">
                <div className="meta-item">
                  <User size={14} />
                  <span>{artifact.created_by}</span>
                </div>
                <div className="meta-item">
                  <Hash size={14} />
                  <span>{artifact.files.length} files</span>
                </div>
              </div>

              <div className="tag-list">
                {artifact.tags.map(t => <span key={t} className="tag">#{t}</span>)}
              </div>

              {artifact.signature && (
                <div className="signature-badge">
                  <Shield size={10} />
                  <span>{t('artifact.verified')}</span>
                </div>
              )}

              <button
                className="delete-btn"
                onClick={(e) => handleDeleteRequest(e, artifact.id)}
                title={t('artifact.purge')}
              >
                <Trash2 size={14} />
              </button>
            </motion.div>
          ))}
        </div>
      )}

      {/* Details Modal */}
      <AnimatePresence>
        {selectedArtifact && (
          <>
            <motion.div
              initial={{ opacity: 0 }}
              animate={{ opacity: 1 }}
              exit={{ opacity: 0 }}
              className="modal-backdrop"
              onClick={() => setSelectedArtifact(null)}
            />
            <motion.div
              layoutId={selectedArtifact.id}
              className="artifact-modal"
            >
              <div className="modal-header">
                <div>
                  <div className="category-tag">
                    {getCategoryIcon(selectedArtifact.category)}
                    <span>{selectedArtifact.category.toUpperCase()}</span>
                  </div>
                  <h2 style={{ margin: '0.4rem 0 0', fontSize: '1.5rem' }}>{selectedArtifact.title}</h2>
                </div>
                <button 
                    onClick={() => setSelectedArtifact(null)}
                    style={{ background: 'transparent', border: 'none', color: 'var(--text-muted)', cursor: 'pointer', transition: 'color var(--speed-fast)' }}
                    className="card-hover"
                >
                    <X size={24} />
                </button>
              </div>

              <div className="modal-content">
                <div className="file-section">
                  <h3 style={{ margin: 0, fontSize: '1.2rem' }}>Files <span style={{ color: 'var(--text-muted)', fontSize: '0.8rem' }}>({selectedArtifact.files.length})</span></h3>
                  <div className="file-list">
                    {selectedArtifact.files.map(file => (
                      <div key={file.name} className="file-item">
                        <div className="file-info">
                          <FileText size={16} color="var(--accent-cyan)" />
                          <div className="file-name-meta">
                            <span className="file-name">{file.name}</span>
                            <span className="file-size">{(file.size_bytes / 1024).toFixed(1)} KB</span>
                          </div>
                        </div>
                        <div className="file-actions">
                          {file.mime_type === 'text/html' && (
                             <button
                               onClick={() => setPreviewFile({ artifact: selectedArtifact, file })}
                               className="icon-btn preview-btn"
                               style={{ color: 'var(--accent-cyan)', marginRight: '8px', cursor: 'pointer', background: 'none', border: 'none' }}
                               title="Preview HTML"
                             >
                               <Eye size={16} />
                             </button>
                           )}
                          <a
                            href={`${API_BASE}/api/artifacts/${selectedArtifact.id}/files/${encodeURIComponent(file.name)}`}
                            target="_blank"
                            rel="noreferrer"
                            className="icon-btn"
                            style={{ color: 'var(--text-muted)', transition: 'color var(--speed-normal)' }}
                          >
                            <Download size={16} />
                          </a>
                        </div>
                      </div>
                    ))}
                  </div>
                </div>

                <div className="detail-sidebar">
                  <div className="detail-group">
                    <label><User size={14} /> Generator</label>
                    <p>{selectedArtifact.created_by}</p>
                  </div>
                  <div className="detail-group">
                    <label><Calendar size={14} /> Created</label>
                    <p>{new Date(selectedArtifact.created_at).toLocaleString()}</p>
                  </div>
                  <div className="detail-group">
                    <label><Tag size={14} /> Tags</label>
                    <div className="tag-list">
                      {selectedArtifact.tags.map(t => <span key={t} className="tag">#{t}</span>)}
                    </div>
                  </div>
                  {selectedArtifact.karma_refs.length > 0 && (
                    <div className="detail-group">
                      <label><Dna size={14} /> Karma Source</label>
                      <p style={{ fontSize: '0.7rem', color: 'var(--accent-purple)' }}>{selectedArtifact.karma_refs.join(", ")}</p>
                    </div>
                  )}
                  {selectedArtifact.signature && (
                    <div className="detail-group">
                      <label><Shield size={14} /> Audit Signature</label>
                      <p className="signature-text">{selectedArtifact.signature}</p>
                    </div>
                  )}

                  {selectedArtifact.edges && selectedArtifact.edges.length > 0 && (
                    <div className="detail-group">
                      <label><Hash size={14} /> Lineage (Provenance)</label>
                      <div className="edge-list">
                        {selectedArtifact.edges.map(edge => (
                          <div key={edge.id} className="edge-item">
                            <span className="edge-relation">{edge.relation}</span>
                            <span className="edge-target">{edge.target_id === selectedArtifact.id ? "from: " + edge.source_id.slice(0, 8) : "to: " + edge.target_id.slice(0, 8)}</span>
                          </div>
                        ))}
                      </div>
                    </div>
                  )}
                </div>
              </div>
            </motion.div>
          </>
        )}
      </AnimatePresence>

      {/* HTML Preview Modal */}
      <AnimatePresence>
        {previewFile && (
          <>
            <motion.div
              initial={{ opacity: 0 }}
              animate={{ opacity: 1 }}
              exit={{ opacity: 0 }}
              onClick={() => setPreviewFile(null)}
              className="modal-backdrop"
              style={{ zIndex: 2000 }}
            />
            <motion.div
              initial={{ opacity: 0, scale: 0.95, y: 20 }}
              animate={{ opacity: 1, scale: 1, y: 0 }}
              exit={{ opacity: 0, scale: 0.95, y: 20 }}
              className="preview-modal"
            >
              <div className="preview-header">
                <div className="preview-title">
                  <FileText size={18} color="var(--accent-cyan)" />
                  <span>{previewFile.file.name}</span>
                </div>
                <button onClick={() => setPreviewFile(null)} className="close-btn">
                  <X size={20} />
                </button>
              </div>
              <div style={{ width: '100%', height: '100%', background: 'var(--white-100)', overflow: 'hidden', position: 'relative' }}>
                <iframe
                  ref={iframeRef}
                  title="HTML Preview"
                  src={`${API_BASE}/api/artifacts/${previewFile.artifact.id}/files/${previewFile.file.name}`}
                  style={{ width: '100%', height: '100%', border: 'none' }}
                  sandbox="allow-scripts allow-popups allow-popups-to-escape-sandbox" // allow-same-origin is intentionally omitted for security
                />
              </div>
            </motion.div>
          </>
        )}
      </AnimatePresence>

      <style>{`
        .vault-container {
          padding: 1rem;
        }
        .search-box {
          display: flex;
          align-items: center;
          gap: 0.8rem;
          background: var(--white-05);
          border: 1px solid var(--white-10);
          border-radius: var(--radius-md);
          padding: 0 1rem;
          height: 48px;
          flex: 1;
        }
        .search-box input {
          background: transparent;
          border: none;
          color: white;
          width: 100%;
          outline: none;
        }
        .filter-chips {
          display: flex;
          gap: 0.5rem;
        }
        .chip {
          background: var(--white-03);
          border: 1px solid var(--white-05);
          color: var(--text-secondary);
          padding: 0.5rem 1rem;
          border-radius: var(--radius-xl);
          font-size: 0.85rem;
          cursor: pointer;
          transition: all var(--speed-normal);
        }
        .chip:hover {
          background: var(--white-08);
        }
        .chip.active {
          background: var(--accent-cyan);
          color: var(--bg-primary);
          font-weight: 700;
        }
        .artifact-grid {
          display: grid;
          grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
          gap: var(--space-md);
        }
        .artifact-card {
          background: var(--bg-glass-light);
          backdrop-filter: blur(10px);
          border: 1px solid var(--border-glass);
          border-radius: var(--radius-lg);
          padding: 1.2rem;
          cursor: pointer;
          transition: transform var(--speed-normal), background var(--speed-normal);
          position: relative;
          overflow: hidden;
        }
        .artifact-card:hover {
          transform: translateY(-4px);
          background: var(--bg-glass-heavy);
          border-color: var(--accent-cyan-30);
        }
        .card-header {
          display: flex;
          justify-content: space-between;
          margin-bottom: 1rem;
          align-items: center;
        }
        .category-tag {
          display: flex;
          align-items: center;
          gap: 0.4rem;
          color: var(--accent-cyan);
          font-size: 0.7rem;
          font-weight: 700;
          letter-spacing: 0.05em;
        }
        .timestamp {
          font-size: 0.7rem;
          color: var(--text-muted);
        }
        .card-title {
          font-size: 1.1rem;
          margin: 0 0 1rem;
          color: white;
          line-height: 1.4;
        }
        .card-meta {
          display: flex;
          gap: 1rem;
          margin-bottom: 1rem;
        }
        .meta-item {
          display: flex;
          align-items: center;
          gap: 0.3rem;
          color: var(--text-secondary);
          font-size: 0.75rem;
        }
        .tag-list {
          display: flex;
          flex-wrap: wrap;
          gap: 0.4rem;
        }
        .tag {
          font-size: 0.7rem;
          color: var(--accent-purple);
          background: var(--accent-purple-15);
          padding: 0.1rem 0.4rem;
          border-radius: 4px;
        }
        .signature-badge {
          position: absolute;
          bottom: 10px;
          right: -25px;
          background: var(--accent-rose);
          color: white;
          font-size: 0.6rem;
          padding: 0.2rem 2rem;
          transform: rotate(-45deg);
          display: flex;
          align-items: center;
          gap: 2px;
          font-weight: 800;
        }
        .delete-btn {
          position: absolute;
          top: 1.2rem;
          right: 1.2rem;
          background: var(--accent-rose-10);
          border: 1px solid var(--accent-rose-20);
          color: var(--accent-rose);
          border-radius: var(--radius-sm);
          padding: 0.4rem;
          cursor: pointer;
          opacity: 0;
          transition: all var(--speed-normal);
          display: flex;
          align-items: center;
          justify-content: center;
        }
        .artifact-card:hover .delete-btn {
          opacity: 1;
        }
        .delete-btn:hover {
          background: var(--accent-rose);
          color: var(--bg-primary);
        }

        .modal-backdrop {
          position: fixed;
          inset: 0;
          background: var(--black-80);
          z-index: 100;
          backdrop-filter: blur(4px);
        }
        .artifact-modal {
          position: fixed;
          top: 10%;
          left: 15%;
          right: 15%;
          bottom: 10%;
          background: var(--bg-secondary);
          border: 1px solid var(--border-glass);
          border-radius: var(--radius-xl);
          z-index: 101;
          display: flex;
          flex-direction: column;
          box-shadow: var(--shadow-deep);
        }
        .modal-header {
          padding: 1.5rem 2rem;
          border-bottom: 1px solid var(--border-glass);
          display: flex;
          justify-content: space-between;
          align-items: center;
        }
        .modal-content {
          flex: 1;
          display: grid;
          grid-template-columns: 1fr 300px;
          overflow: hidden;
        }
        .file-section {
          padding: 2rem;
          overflow-y: auto;
          border-right: 1px solid var(--border-glass);
        }
        .file-list {
          display: flex;
          flex-direction: column;
          gap: 0.8rem;
          margin-top: 1rem;
        }
        .file-item {
          display: flex;
          justify-content: space-between;
          align-items: center;
          padding: 1rem;
          background: var(--white-03);
          border-radius: var(--radius-md);
          border: 1px solid transparent;
          transition: all var(--speed-normal);
        }
        .file-item:hover {
          background: var(--white-05);
          border-color: var(--border-glass);
        }
        .file-info {
          display: flex;
          align-items: center;
          gap: 1rem;
        }
        .file-name-meta {
          display: flex;
          flex-direction: column;
        }
        .file-name {
          font-weight: 500;
          color: white;
        }
        .file-size {
          font-size: 0.75rem;
          color: var(--text-muted);
        }
        .detail-sidebar {
          padding: 2rem;
          background: var(--black-20);
          display: flex;
          flex-direction: column;
          gap: 1.5rem;
          overflow-y: auto;
        }
        .detail-group label {
          display: flex;
          align-items: center;
          gap: 0.4rem;
          font-size: 0.7rem;
          color: var(--text-muted);
          text-transform: uppercase;
          letter-spacing: 0.05em;
          margin-bottom: 0.5rem;
        }
        .detail-group p {
          color: var(--text-secondary);
          font-size: 0.9rem;
          margin: 0;
        }
        .signature-text {
          font-family: var(--font-mono);
          font-size: 0.65rem !important;
          word-break: break-all;
          background: var(--black-40);
          padding: 0.75rem;
          border-radius: 8px;
          border: 1px solid var(--border-glass);
          line-height: 1.4;
        }
        .edge-list {
          display: flex;
          flex-direction: column;
          gap: 0.4rem;
        }
        .edge-item {
          font-size: 0.75rem;
          color: var(--text-secondary);
          background: var(--white-03);
          padding: 0.6rem;
          border-radius: var(--radius-sm);
          border-left: 3px solid var(--accent-cyan);
          display: flex;
          justify-content: space-between;
          align-items: center;
        }
        .edge-relation {
          color: var(--accent-cyan);
          font-weight: 700;
          text-transform: uppercase;
          font-size: 0.65rem;
        }
        .edge-target {
          color: var(--text-muted);
          font-family: var(--font-mono);
          font-size: 0.7rem;
        }

        @media (max-width: 1024px) {
          .modal-content {
            grid-template-columns: 1fr;
          }
          .artifact-modal {
            left: 5%;
            right: 5%;
          }
          .detail-sidebar {
            border-top: 1px solid var(--border-glass);
          }
        }
        .preview-modal {
          position: fixed;
          top: 50%;
          left: 50%;
          transform: translate(-50%, -50%);
          width: 90vw;
          height: 85vh;
          background: var(--bg-primary);
          border: 1px solid var(--white-10);
          border-radius: var(--radius-lg);
          z-index: 2001;
          display: flex;
          flex-direction: column;
          box-shadow: 0 20px 40px rgba(0,0,0,0.5);
          overflow: hidden;
        }
        .preview-header {
          padding: 1rem 1.5rem;
          background: var(--bg-secondary);
          border-bottom: 1px solid var(--white-10);
          display: flex;
          justify-content: space-between;
          align-items: center;
        }
        .preview-title {
          display: flex;
          align-items: center;
          gap: 0.8rem;
          font-family: var(--font-family-display);
          font-size: 1.1rem;
        }
        .preview-body {
          flex: 1;
          background: white; /* Default for HTML content */
        }
        .preview-iframe {
          width: 100%;
          height: 100%;
          border: none;
        }
        .close-btn {
          background: transparent;
          border: none;
          color: var(--text-muted);
          cursor: pointer;
          transition: color var(--speed-normal);
        }
        .close-btn:hover {
          color: var(--white-100);
        }
      `}</style>
    </div>
  );
};

export default ArtifactVault;
