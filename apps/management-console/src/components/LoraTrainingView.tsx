/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import React, { useState, useEffect } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import {
    Settings,
    Play,
    Activity,
    BrainCircuit,
    Database,
    RefreshCw,
    Network
} from 'lucide-react';
import { API_BASE } from '../config';
import { authenticatedFetch } from '../lib/auth';
import { useTranslation } from '../i18n';

const LoraTrainingView: React.FC = () => {
    const { t } = useTranslation();
    const [baseModel, setBaseModel] = useState('mistral-7b');
    const [datasetId, setDatasetId] = useState('');
    const [epochs, setEpochs] = useState<number>(3);
    const [lr, setLr] = useState<number>(0.0001);
    const [loraRank, setLoraRank] = useState<number>(16);
    const [batchSize, setBatchSize] = useState<number>(4);
    
    const [isTraining, setIsTraining] = useState(false);
    const [jobId, setJobId] = useState<string | null>(null);
    const [status, setStatus] = useState<string>('');
    const [error, setError] = useState<string | null>(null);

    useEffect(() => {
        let interval: number;

        const checkStatus = async () => {
            if (!jobId) return;
            try {
                const res = await authenticatedFetch(`${API_BASE}/api/v1/lora/status/${jobId}`);
                if (res.ok) {
                    const data = await res.json();
                    setStatus(data.status);
                    if (data.status === 'Completed' || data.status === 'Failed') {
                        setIsTraining(false);
                    }
                } else {
                    setStatus('Error fetching status');
                }
            } catch (err) {
                console.error(err);
                setStatus('Network Error');
            }
        };

        if (isTraining && jobId) {
            checkStatus(); // Initial check
            interval = setInterval(checkStatus, 3000);
        }

        return () => {
            if (interval) clearInterval(interval);
        };
    }, [isTraining, jobId]);

    const handleStartTraining = async () => {
        if (!datasetId) {
            setError('Dataset ID is required');
            return;
        }
        
        setError(null);
        setIsTraining(true);
        setStatus('Starting...');
        
        try {
            const res = await authenticatedFetch(`${API_BASE}/api/v1/lora/train`, {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({
                    base_model: baseModel,
                    dataset_id: datasetId,
                    params: {
                        epochs,
                        lr,
                        lora_rank: loraRank,
                        batch_size: batchSize
                    }
                })
            });
            
            if (res.ok) {
                const data = await res.json();
                setJobId(data.job_id);
                setStatus('Pending');
            } else {
                const text = await res.text();
                setError(text || 'Failed to start training');
                setIsTraining(false);
            }
        } catch (err: unknown) {
            console.error(err);
            if (err instanceof Error) {
                setError(err.message);
            } else {
                setError('Network Error or Unknown Failure');
            }
            setIsTraining(false);
        }
    };

    return (
        <div className="ani-fade" style={{ display: 'grid', gridTemplateColumns: 'minmax(300px, 1fr) 1fr', gap: 'var(--space-md)', height: 'calc(100vh - 180px)' }}>
            {/* Left Panel: Configuration */}
            <div className="main-panel" style={{ padding: '2rem', display: 'flex', flexDirection: 'column', overflowY: 'auto' }}>
                <div style={{ display: 'flex', alignItems: 'center', gap: '0.75rem', marginBottom: '2rem', color: 'var(--accent-purple)' }}>
                    <BrainCircuit size={28} />
                    <h2 style={{ margin: 0, fontWeight: 700 }}>{t('lora.title')}</h2>
                </div>

                <p style={{ color: 'var(--text-secondary)', fontSize: '0.9rem', marginBottom: '2rem', lineHeight: 1.5 }}>
                    Configure and launch self-supervised domain adaptation. The Autotuner will optimize hyperparameters based on the target dataset.
                </p>

                {error && (
                    <div style={{ padding: '1rem', background: 'rgba(255,50,50,0.1)', color: 'var(--accent-rose)', borderRadius: '8px', marginBottom: '1.5rem', fontSize: '0.85rem' }}>
                        {error}
                    </div>
                )}

                <div style={{ display: 'flex', flexDirection: 'column', gap: '1.5rem', marginBottom: '2rem' }}>
                    <div className="form-group">
                        <label style={{ display: 'flex', alignItems: 'center', gap: '0.5rem', fontSize: '0.8rem', color: 'var(--text-muted)' }}>
                            <Network size={14} /> {t('lora.baseModel')}
                        </label>
                        <select 
                            value={baseModel} 
                            onChange={(e) => setBaseModel(e.target.value)}
                            disabled={isTraining}
                            style={{ padding: '0.75rem', background: 'rgba(0,0,0,0.3)', border: '1px solid rgba(255,255,255,0.1)', borderRadius: '8px', color: 'white', marginTop: '0.5rem', width: '100%' }}
                        >
                            <option value="mistral-7b">Mistral 7B (Instruct)</option>
                            <option value="llama-3-8b">Llama 3 8B</option>
                            <option value="qwen-1.5-7b">Qwen 1.5 7B</option>
                        </select>
                    </div>

                    <div className="form-group">
                        <label style={{ display: 'flex', alignItems: 'center', gap: '0.5rem', fontSize: '0.8rem', color: 'var(--text-muted)' }}>
                            <Database size={14} /> Knowledge Dataset ID
                        </label>
                        <input 
                            type="text" 
                            placeholder="e.g. core-skills-v2"
                            value={datasetId}
                            onChange={(e) => setDatasetId(e.target.value)}
                            disabled={isTraining}
                            style={{ padding: '0.75rem', background: 'rgba(0,0,0,0.3)', border: '1px solid rgba(255,255,255,0.1)', borderRadius: '8px', color: 'white', marginTop: '0.5rem', width: '100%' }}
                        />
                    </div>

                    <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '1rem' }}>
                        <div className="form-group">
                            <label style={{ fontSize: '0.8rem', color: 'var(--text-muted)' }}>{t('lora.epochs')}</label>
                            <input 
                                type="number" 
                                min="1" max="100"
                                value={epochs}
                                onChange={(e) => setEpochs(Number(e.target.value))}
                                disabled={isTraining}
                                style={{ padding: '0.75rem', background: 'rgba(0,0,0,0.3)', border: '1px solid rgba(255,255,255,0.1)', borderRadius: '8px', color: 'white', marginTop: '0.5rem', width: '100%' }}
                            />
                        </div>
                        <div className="form-group">
                            <label style={{ fontSize: '0.8rem', color: 'var(--text-muted)' }}>{t('lora.learningRate')}</label>
                            <input 
                                type="number" 
                                step="0.00001"
                                value={lr}
                                onChange={(e) => setLr(Number(e.target.value))}
                                disabled={isTraining}
                                style={{ padding: '0.75rem', background: 'rgba(0,0,0,0.3)', border: '1px solid rgba(255,255,255,0.1)', borderRadius: '8px', color: 'white', marginTop: '0.5rem', width: '100%' }}
                            />
                        </div>
                    </div>

                    <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '1rem' }}>
                        <div className="form-group">
                            <label style={{ fontSize: '0.8rem', color: 'var(--text-muted)' }}>{t('lora.loraRank')}</label>
                            <input 
                                type="number" 
                                min="4" max="256" step="4"
                                value={loraRank}
                                onChange={(e) => setLoraRank(Number(e.target.value))}
                                disabled={isTraining}
                                style={{ padding: '0.75rem', background: 'rgba(0,0,0,0.3)', border: '1px solid rgba(255,255,255,0.1)', borderRadius: '8px', color: 'white', marginTop: '0.5rem', width: '100%' }}
                            />
                        </div>
                        <div className="form-group">
                            <label style={{ fontSize: '0.8rem', color: 'var(--text-muted)' }}>{t('lora.batchSize')}</label>
                            <input 
                                type="number" 
                                min="1" max="128" step="1"
                                value={batchSize}
                                onChange={(e) => setBatchSize(Number(e.target.value))}
                                disabled={isTraining}
                                style={{ padding: '0.75rem', background: 'rgba(0,0,0,0.3)', border: '1px solid rgba(255,255,255,0.1)', borderRadius: '8px', color: 'white', marginTop: '0.5rem', width: '100%' }}
                            />
                        </div>
                    </div>
                </div>

                <div style={{ marginTop: 'auto' }}>
                    <button 
                        onClick={handleStartTraining}
                        disabled={isTraining || !datasetId}
                        style={{ 
                            width: '100%', 
                            padding: '1rem', 
                            background: isTraining ? 'rgba(255,255,255,0.1)' : 'var(--accent-purple)', 
                            color: isTraining ? 'var(--text-muted)' : '#fff',
                            border: 'none', 
                            borderRadius: '8px', 
                            fontSize: '1rem', 
                            fontWeight: 700,
                            cursor: isTraining || !datasetId ? 'not-allowed' : 'pointer',
                            display: 'flex',
                            alignItems: 'center',
                            justifyContent: 'center',
                            gap: '0.5rem',
                            transition: 'all 0.2s'
                        }}
                    >
                        {isTraining ? <><RefreshCw className="ani-spin" size={18} /> Optimizing...</> : <><Play size={18} /> Initialize Training</>}
                    </button>
                </div>
            </div>

            {/* Right Panel: Job Status & Analytics */}
            <div className="main-panel" style={{ padding: '2rem', display: 'flex', flexDirection: 'column' }}>
                <h3 style={{ display: 'flex', alignItems: 'center', gap: '0.5rem', color: 'var(--text-primary)', marginBottom: '1.5rem', fontSize: '1.1rem' }}>
                    <Activity size={18} color="var(--accent-cyan)" /> Telemetry & Status
                </h3>

                {!jobId ? (
                    <div style={{ flex: 1, display: 'flex', alignItems: 'center', justifyContent: 'center', color: 'var(--text-muted)', flexDirection: 'column', gap: '1rem' }}>
                        <Settings size={48} style={{ opacity: 0.2 }} />
                        <p>{t('lora.noActiveSession')}</p>
                    </div>
                ) : (
                    <AnimatePresence>
                        <motion.div 
                            initial={{ opacity: 0, scale: 0.95 }}
                            animate={{ opacity: 1, scale: 1 }}
                            style={{ flex: 1, display: 'flex', flexDirection: 'column' }}
                        >
                            <div style={{ background: 'rgba(0,0,0,0.3)', border: '1px solid rgba(255,255,255,0.05)', borderRadius: '12px', padding: '1.5rem', marginBottom: '1.5rem' }}>
                                <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: '1rem' }}>
                                    <span style={{ color: 'var(--text-muted)', fontSize: '0.85rem' }}>{t('lora.jobId')}</span>
                                    <span style={{ fontFamily: 'monospace', color: 'var(--accent-cyan)', fontSize: '0.85rem' }}>{jobId}</span>
                                </div>
                                <div style={{ display: 'flex', justifyContent: 'space-between' }}>
                                    <span style={{ color: 'var(--text-muted)', fontSize: '0.85rem' }}>{t('lora.status')}</span>
                                    <span style={{ 
                                        color: status === 'Completed' ? 'var(--accent-emerald)' : 
                                               status === 'Failed' ? 'var(--accent-rose)' : 
                                               'var(--accent-amber)',
                                        fontWeight: 700,
                                        fontSize: '0.85rem'
                                    }}>
                                        {status.toUpperCase()}
                                    </span>
                                </div>
                            </div>

                            <div style={{ flex: 1, background: '#0a0a0a', border: '1px solid rgba(255,255,255,0.05)', borderRadius: '12px', padding: '1rem', overflowY: 'auto', fontFamily: 'monospace', fontSize: '0.8rem', color: 'var(--text-secondary)' }}>
                                <div style={{ color: 'var(--accent-cyan)', marginBottom: '0.5rem' }}>&gt; System initialized...</div>
                                <div>&gt; Base model: {baseModel}</div>
                                <div>&gt; Target dataset: {datasetId}</div>
                                <div>&gt; LR: {lr}, Epochs: {epochs}</div>
                                <div>&gt; Rank: {loraRank}, Batch: {batchSize}</div>
                                {isTraining && (
                                    <div style={{ marginTop: '1rem', color: 'var(--accent-amber)' }}>
                                        <RefreshCw className="ani-spin" size={14} style={{ display: 'inline', verticalAlign: 'middle', marginRight: '0.5rem' }} />
                                        Computing gradients...
                                    </div>
                                )}
                                {status === 'Completed' && (
                                    <div style={{ marginTop: '1rem', color: 'var(--accent-emerald)' }}>
                                        &gt; [SUCCESS] Model weights successfully exported to Vault.
                                    </div>
                                )}
                            </div>
                        </motion.div>
                    </AnimatePresence>
                )}
            </div>
        </div>
    );
};

export default LoraTrainingView;
