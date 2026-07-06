/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

import React, { useState, useEffect } from 'react';
import { Sparkles, Check, X, CreditCard } from 'lucide-react';
import { useCheckoutSession } from '../../hooks/useCheckoutSession';
import { cssVar } from '../../utils/cssVar';
import { useTranslation } from '../../i18n';
import { Modal } from '../ui/Modal';

interface ProUpgradeModalProps {
    priceId: string;
    agentId?: string;
    /** Optional feature name i18n key when opened from LockedOverlay */
    triggerFeatureKey?: string;
}

const FEATURE_KEYS = [
    { title: 'pro.featureBuzz', desc: 'pro.featureBuzzDesc' },
    { title: 'pro.featureAgentSense', desc: 'pro.featureAgentSenseDesc' },
    { title: 'pro.featureLora', desc: 'pro.featureLoraDesc' },
    { title: 'pro.featureGift', desc: 'pro.featureGiftDesc' },
    { title: 'pro.featureTts', desc: 'pro.featureTtsDesc' },
] as const;

export const ProUpgradeModal: React.FC<ProUpgradeModalProps> = ({ priceId, agentId }) => {
    const { t } = useTranslation();
    const [isOpen, setIsOpen] = useState(false);
    const [triggerFeatureKey, setTriggerFeatureKey] = useState<string | undefined>();
    const { handleCheckout, isLoading, error } = useCheckoutSession(priceId, agentId);

    useEffect(() => {
        const handle402Event = () => {
            setIsOpen(true);
        };
        const handleOpenEvent = (e: Event) => {
            const detail = (e as CustomEvent<{ featureKey?: string }>).detail;
            setTriggerFeatureKey(detail?.featureKey);
            setIsOpen(true);
        };

        window.addEventListener('stripe-402-payment-required', handle402Event);
        window.addEventListener('pro-upgrade-modal-open', handleOpenEvent);
        return () => {
            window.removeEventListener('stripe-402-payment-required', handle402Event);
            window.removeEventListener('pro-upgrade-modal-open', handleOpenEvent);
        };
    }, []);

    return (
        <Modal
            isOpen={isOpen}
            onClose={() => setIsOpen(false)}
            labelledBy="pro-upgrade-modal-title"
            overlayStyle={styles.overlay}
            dialogStyle={styles.modal}
            zIndex={9999}
        >
                <div style={styles.header}>
                    <div style={styles.sparkleContainer}>
                        <Sparkles size={32} color={cssVar('--accent-purple', '#bc8cff')} style={styles.iconGlow} />
                    </div>
                    <button type="button" onClick={() => setIsOpen(false)} style={styles.closeButton} aria-label={t('pro.closeModal')}>
                        <X size={18} color={cssVar('--text-secondary', '#94a3b8')} />
                    </button>
                </div>

                <div style={styles.content}>
                    <h2 id="pro-upgrade-modal-title" style={styles.title}>{t('pro.modalTitle')}</h2>
                    {triggerFeatureKey && (
                        <p style={{ ...styles.subtitle, color: 'var(--accent-purple)', fontWeight: 600 }}>
                            {t('pro.triggeredBy', { feature: t(triggerFeatureKey) })}
                        </p>
                    )}
                    <p style={styles.subtitle}>{t('pro.modalSubtitle')}</p>

                    <div style={styles.featuresList}>
                        {FEATURE_KEYS.map(({ title, desc }) => (
                            <div key={title} style={styles.featureItem}>
                                <div style={styles.checkIconWrapper}>
                                    <Check size={14} color={cssVar('--accent-emerald', '#10b981')} />
                                </div>
                                <div>
                                    <h4 style={styles.featureTitle}>{t(title)}</h4>
                                    <p style={styles.featureDesc}>{t(desc)}</p>
                                </div>
                            </div>
                        ))}
                    </div>

                    <div style={styles.priceContainer}>
                        <span style={styles.priceAmount}>$19.99</span>
                        <span style={styles.pricePeriod}>{t('pro.pricePeriod')}</span>
                        <div style={styles.trialBadge}>{t('pro.trialBadge')}</div>
                    </div>

                    {error && <div style={styles.errorMessage}>{error}</div>}
                </div>

                <div style={styles.footer}>
                    <button type="button" onClick={() => setIsOpen(false)} style={styles.cancelButton} disabled={isLoading}>
                        {t('pro.cancel')}
                    </button>
                    <button type="button" onClick={handleCheckout} style={styles.upgradeButton} disabled={isLoading}>
                        {isLoading ? (
                            <span style={styles.spinner}></span>
                        ) : (
                            <>
                                <CreditCard size={16} style={{ marginRight: '0.5rem' }} />
                                {t('pro.upgrade')}
                            </>
                        )}
                    </button>
                </div>
        </Modal>
    );
};

const styles: { [key: string]: React.CSSProperties } = {
    overlay: {
        position: 'fixed',
        top: 0,
        left: 0,
        right: 0,
        bottom: 0,
        backgroundColor: 'var(--black-85)',
        backdropFilter: 'blur(8px)',
        display: 'flex',
        justifyContent: 'center',
        alignItems: 'center',
        zIndex: 9999,
    },
    modal: {
        width: '460px',
        backgroundColor: 'var(--bg-primary)',
        border: '1px solid var(--accent-purple-20)',
        borderRadius: 'var(--radius-lg)',
        boxShadow: 'var(--shadow-deep)',
        overflow: 'hidden',
        position: 'relative',
        display: 'flex',
        flexDirection: 'column',
    },
    header: {
        display: 'flex',
        justifyContent: 'space-between',
        alignItems: 'center',
        padding: '1.25rem 1.5rem 0.5rem',
    },
    sparkleContainer: {
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        width: '48px',
        height: '48px',
        borderRadius: 'var(--radius-md)',
        background: 'linear-gradient(135deg, var(--accent-purple-15), var(--accent-rose-15))',
        border: '1px solid var(--accent-purple-20)',
    },
    iconGlow: {
        filter: 'drop-shadow(0 0 8px var(--accent-purple-50))',
    },
    closeButton: {
        background: 'none',
        border: 'none',
        cursor: 'pointer',
        padding: '0.25rem',
        borderRadius: 'var(--radius-sm)',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
    },
    content: {
        padding: '0.5rem 1.5rem 1.5rem',
        display: 'flex',
        flexDirection: 'column',
    },
    title: {
        fontSize: '1.6rem',
        fontWeight: 700,
        color: 'var(--text-primary)',
        margin: '0.75rem 0 0.5rem',
    },
    subtitle: {
        fontSize: '0.925rem',
        color: 'var(--text-secondary)',
        lineHeight: 1.5,
        margin: '0 0 1.5rem 0',
    },
    featuresList: {
        display: 'flex',
        flexDirection: 'column',
        gap: '1rem',
        backgroundColor: 'var(--white-02)',
        border: '1px solid var(--white-05)',
        borderRadius: 'var(--radius-md)',
        padding: '1.25rem',
        marginBottom: '1.5rem',
    },
    featureItem: {
        display: 'flex',
        gap: '0.75rem',
        alignItems: 'flex-start',
    },
    checkIconWrapper: {
        width: '20px',
        height: '20px',
        borderRadius: '50%',
        backgroundColor: 'var(--accent-emerald-10)',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        marginTop: '2px',
        flexShrink: 0,
    },
    featureTitle: {
        fontSize: '0.9rem',
        fontWeight: 600,
        color: 'var(--text-primary)',
        margin: 0,
    },
    featureDesc: {
        fontSize: '0.8rem',
        color: 'var(--text-muted)',
        margin: '0.2rem 0 0 0',
        lineHeight: 1.4,
    },
    priceContainer: {
        display: 'flex',
        alignItems: 'baseline',
        gap: '0.25rem',
        justifyContent: 'center',
        padding: '0.5rem 0',
    },
    priceAmount: {
        fontSize: '2rem',
        fontWeight: 800,
        color: 'var(--text-primary)',
    },
    pricePeriod: {
        fontSize: '0.9rem',
        color: 'var(--text-muted)',
    },
    trialBadge: {
        marginLeft: '0.75rem',
        fontSize: '0.75rem',
        fontWeight: 600,
        color: 'var(--accent-purple)',
        backgroundColor: 'var(--accent-purple-10)',
        border: '1px solid var(--accent-purple-20)',
        borderRadius: '20px',
        padding: '0.2rem 0.6rem',
    },
    errorMessage: {
        color: 'var(--accent-rose)',
        fontSize: '0.825rem',
        backgroundColor: 'var(--accent-rose-10)',
        border: '1px solid var(--accent-rose-20)',
        borderRadius: 'var(--radius-sm)',
        padding: '0.6rem 0.8rem',
        marginTop: '1rem',
        textAlign: 'center',
    },
    footer: {
        display: 'flex',
        justifyContent: 'flex-end',
        gap: '0.75rem',
        padding: '1.25rem 1.5rem',
        borderTop: '1px solid var(--white-05)',
        backgroundColor: 'var(--bg-primary)',
    },
    cancelButton: {
        padding: '0.625rem 1.25rem',
        borderRadius: 'var(--radius-sm)',
        backgroundColor: 'transparent',
        border: '1px solid var(--white-10)',
        color: 'var(--text-secondary)',
        fontSize: '0.9rem',
        fontWeight: 600,
        cursor: 'pointer',
    },
    upgradeButton: {
        padding: '0.625rem 1.5rem',
        borderRadius: 'var(--radius-sm)',
        background: 'linear-gradient(135deg, var(--accent-purple), var(--accent-rose))',
        border: 'none',
        color: 'var(--white-100)',
        fontSize: '0.9rem',
        fontWeight: 600,
        cursor: 'pointer',
        boxShadow: 'var(--glow-purple)',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
    },
    spinner: {
        display: 'inline-block',
        width: '18px',
        height: '18px',
        border: '2px solid var(--white-30)',
        borderRadius: '50%',
        borderTopColor: 'var(--white-100)',
    },
};
