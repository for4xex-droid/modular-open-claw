/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import { useState, useEffect, useCallback } from "react";
import { motion } from "framer-motion";
import { ShoppingCart, Volume2, ShieldCheck, Crown } from "lucide-react";
import { API_BASE, STRIPE_PRICE_ID } from "../config";
import { authenticatedFetch } from "../lib/auth";
import { useCheckoutSession } from '../hooks/useCheckoutSession';
import { useTranslation } from '../i18n';
import { useAgentIdentity } from '../hooks/useAgentIdentity';
import { useToast } from './common/Toast';

interface VoiceAsset {
  id: string;
  name: string;
  description: string;
  price_coins: number;
  author: string;
  tags: string[];
}

const mockAssets: VoiceAsset[] = [
  {
    id: "v-001",
    name: "Ethereal Whisper",
    description: "A soft, calming voice perfect for late-night assistance.",
    price_coins: 500,
    author: "SoundForge",
    tags: ["calm", "assistant", "soft"],
  },
  {
    id: "v-002",
    name: "Cybernetic Commander",
    description: "Authoritative and precise. Ideal for system operations.",
    price_coins: 1200,
    author: "NexusCorp",
    tags: ["authoritative", "system", "loud"],
  },
  {
    id: "v-003",
    name: "Neural Muse",
    description: "Expressive and dynamic, great for storytelling.",
    price_coins: 850,
    author: "ArtisansAI",
    tags: ["expressive", "story", "dynamic"],
  }
];

export default function VoiceStore() {
    const { t } = useTranslation();
  const { agentId, isEkycVerified } = useAgentIdentity();
  const { showToast } = useToast();
  const [assets, setAssets] = useState<VoiceAsset[]>(mockAssets);
  const [balance, setBalance] = useState<number>(0);
  const [balanceError, setBalanceError] = useState(false);
  const [purchasing, setPurchasing] = useState<string | null>(null);

  const {
    handleCheckout,
    handlePortal,
    isLoading: isRecharging,
    isPortalLoading: isManagingPortal,
    error: checkoutError
  } = useCheckoutSession(STRIPE_PRICE_ID, agentId || undefined);

  useEffect(() => {
    if (checkoutError) {
      showToast('error', checkoutError);
    }
  }, [checkoutError, showToast]);

  const fetchBalance = useCallback(async () => {
    if (!agentId) return;
    try {
      setBalanceError(false);
      const res = await authenticatedFetch(`${API_BASE}/api/v1/commerce/balance/${agentId}`);
      if (res.ok) {
        const data = await res.json();
        setBalance(typeof data?.balance === 'number' ? data.balance : 0);
      } else {
        setBalanceError(true);
      }
    } catch (e) {
      console.error("Failed to fetch balance", e);
      setBalanceError(true);
    }
  }, [agentId]);

  const fetchVoiceAssets = useCallback(async () => {
    try {
      const res = await authenticatedFetch(`${API_BASE}/api/v1/voice/list?scope=public`);
      if (res.ok) {
        const data = await res.json();
        if (Array.isArray(data)) {
          const mappedAssets: VoiceAsset[] = data.map((item: Record<string, unknown>) => ({
            id: String(item.id ?? ''),
            name: String(item.name ?? ''),
            description: String(item.description ?? ''),
            price_coins: typeof item.price_coins === 'number' ? item.price_coins : 0,
            author: String(item.creator_id ?? ''),
            tags: ["voice", "api"],
          }));
          setAssets(mappedAssets.length > 0 ? mappedAssets : []);
        }
      }
    } catch (e) {
      console.error("Failed to fetch voice assets", e);
    }
  }, []);

  useEffect(() => {
    fetchBalance();
    fetchVoiceAssets();
  }, [fetchBalance, fetchVoiceAssets]);

  const handlePurchase = async (asset: VoiceAsset) => {
    if (!agentId) return;
    setPurchasing(asset.id);
    
    try {
      const res = await authenticatedFetch(`${API_BASE}/api/v1/commerce/purchase/${agentId}`, {
        method: "POST",
        body: JSON.stringify({
          item_id: asset.id,
          metadata: {
            amount_coins: asset.price_coins,
            context_layer: "voice_registry_buy"
          }
        })
      });

      if (res.ok) {
        await fetchBalance();
        showToast('success', t('voice.purchaseSuccess', { name: asset.name }));
      } else {
        let message = t('voice.insufficientFunds') || 'Insufficient funds';
        try {
          const data = await res.json();
          if (data?.message) message = data.message;
        } catch {
          message = `${t('common.error') || 'Error'}: ${res.status} ${res.statusText}`;
        }
        showToast('error', message);
      }
    } catch (e) {
      showToast('error', t('common.networkError'));
    } finally {
      setPurchasing(null);
    }
  };

  return (
    <div className="system-panel" style={{ padding: "2rem", height: "100%", overflowY: "auto", position: 'relative' }}>
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: "2rem" }}>
        <div>
          <h3 style={{ margin: 0, color: "var(--text-primary)", display: "flex", alignItems: "center", gap: "0.5rem" }}>
            <Crown size={24} color="var(--accent-purple)" />
            {t('voice.title') || 'Creator Registry & Voice Store'}
          </h3>
          <p style={{ margin: "0.5rem 0 0", color: "var(--text-secondary)", fontSize: "0.9rem" }}>
            {t('voice.subtitle') || 'Acquire premium XTTS voices with mathematically enforced DRM.'}
          </p>
        </div>
        <div style={{ 
          background: "var(--black-30)", 
          padding: "0.75rem 1.5rem", 
          borderRadius: "8px",
          border: "1px solid var(--white-05)",
          display: "flex",
          alignItems: "center",
          gap: "1rem"
        }}>
          <div>
            <span style={{ fontSize: "0.75rem", color: "var(--text-muted)", textTransform: "uppercase" }}>{t('voice.walletBalance')}</span>
            <div style={{ fontWeight: "bold", color: balanceError ? "var(--accent-rose)" : "var(--accent-cyan)", fontSize: "1.2rem" }}>
              {balanceError ? (t('common.error') || '—') : `${balance.toLocaleString()} KC`}
            </div>
          </div>
          <button 
            className="primary-button" 
            style={{ padding: "0.5rem 1rem", fontSize: "0.85rem" }}
            onClick={handleCheckout}
            disabled={isRecharging || !agentId}
          >
            {isRecharging ? (t('common.processing') || 'Processing...') : (t('voice.recharge') || 'Recharge')}
          </button>
          <button 
            className="secondary-button" 
            style={{ padding: "0.5rem 1rem", fontSize: "0.85rem" }}
            onClick={handlePortal}
            disabled={isManagingPortal || !agentId}
          >
            {isManagingPortal ? (t('common.processing') || 'Processing...') : (t('voice.manageSubscription') || 'Manage')}
          </button>
        </div>
      </div>

      <div style={{ 
        display: "grid", 
        gridTemplateColumns: "repeat(auto-fill, minmax(300px, 1fr))", 
        gap: "1.5rem" 
      }}>
        {assets.map(asset => (
          <motion.div 
            key={asset.id}
            initial={{ opacity: 0, scale: 0.95 }}
            animate={{ opacity: 1, scale: 1 }}
            className="config-card"
            style={{ display: "flex", flexDirection: "column" }}
          >
            <div style={{ display: "flex", justifyContent: "space-between", alignItems: "flex-start", marginBottom: "1rem" }}>
              <div>
                <h4 style={{ margin: "0 0 0.25rem", color: "var(--text-primary)" }}>{asset.name}</h4>
                <div style={{ fontSize: "0.8rem", color: "var(--text-muted)" }}>{t('voice.authorPrefix') || 'by '} {asset.author}</div>
              </div>
              <div style={{ background: "var(--accent-cyan-10)", color: "var(--accent-cyan)", padding: "4px 8px", borderRadius: "4px", fontSize: "0.8rem", fontWeight: "bold", display: "flex", alignItems: "center", gap: "4px" }}>
                <span>{asset.price_coins}</span> KC
              </div>
            </div>

            <p style={{ fontSize: "0.9rem", color: "var(--text-secondary)", flex: 1, marginBottom: "1.5rem" }}>
              {asset.description}
            </p>

            <div style={{ display: "flex", gap: "0.5rem", marginBottom: "1.5rem", flexWrap: "wrap" }}>
              {asset.tags.map(tag => (
                <span key={tag} style={{ 
                  fontSize: "0.7rem", 
                  padding: "2px 8px", 
                  borderRadius: "12px", 
                  background: "var(--white-05)",
                  color: "var(--text-muted)"
                }}>
                  #{tag}
                </span>
              ))}
            </div>

            <div style={{ display: "flex", gap: "0.5rem" }}>
              <button 
                className="secondary-button" 
                style={{ flex: 1, display: "flex", justifyContent: "center", alignItems: "center", gap: "0.5rem" }}
              >
                <Volume2 size={16} /> {t('common.preview') || 'Preview'}
              </button>
              <button 
                className="primary-button" 
                style={{ flex: 1, display: "flex", justifyContent: "center", alignItems: "center", gap: "0.5rem" }}
                onClick={() => handlePurchase(asset)}
                disabled={purchasing === asset.id || balance < asset.price_coins || !isEkycVerified}
              >
                {purchasing === asset.id ? (
                  <span className="ani-pulse">{t('voice.securing')}</span>
                ) : (
                  <>
                    <ShoppingCart size={16} /> {t('voice.purchase') || 'Purchase'}
                  </>
                )}
              </button>
            </div>
            {(!isEkycVerified || balance < asset.price_coins) && (
              <div style={{ marginTop: "0.75rem", fontSize: "0.75rem", color: "var(--accent-rose)", textAlign: "center" }}>
                {!isEkycVerified ? (t('voice.ekycRequired') || 'eKYC Required') : (t('voice.insufficientFunds') || 'Insufficient funds')}
              </div>
            )}
            <div style={{ marginTop: "1rem", fontSize: "0.7rem", color: "var(--text-muted)", display: "flex", alignItems: "center", justifyContent: "center", gap: "0.25rem" }}>
              <ShieldCheck size={12} color="var(--accent-emerald)" />
              {t('voice.drmPowered') || 'Powered by Abyss Security Proxy DRM'}
            </div>
          </motion.div>
        ))}
      </div>
    </div>
  );
}
