import { useState, useEffect } from "react";
import { motion } from "framer-motion";
import { ShoppingCart, Volume2, ShieldCheck, Crown } from "lucide-react";
import { API_BASE } from "../config";
import { authenticatedFetch } from "../lib/auth";

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
  const [assets, setAssets] = useState<VoiceAsset[]>(mockAssets);
  const [balance, setBalance] = useState<number>(0);
  const [purchasing, setPurchasing] = useState<string | null>(null);

  useEffect(() => {
    fetchBalance();
    fetchVoiceAssets();
  }, []);

  const fetchBalance = async () => {
    try {
      const res = await authenticatedFetch(`${API_BASE}/api/v1/commerce/balance/agent-001`);
      if (res.ok) {
        const data = await res.json();
        setBalance(data.coins);
      } else {
        setBalance(0);
      }
    } catch (e) {
      setBalance(0);
    }
  };

  const fetchVoiceAssets = async () => {
    try {
      const res = await authenticatedFetch(`${API_BASE}/api/v1/voice/list?scope=public`);
      if (res.ok) {
        const data = await res.json();
        const mappedAssets = data.map((item: any) => ({
          id: item.id,
          name: item.name,
          description: item.description,
          price_coins: item.price_coins,
          author: item.creator_id,
          tags: ["voice", "api"],
        }));
        if (mappedAssets.length > 0) {
            setAssets(mappedAssets);
        }
      }
    } catch (e) {
      console.error("Failed to fetch voice assets", e);
    }
  };

  const handlePurchase = async (asset: VoiceAsset) => {
    setPurchasing(asset.id);
    
    try {
      const res = await authenticatedFetch(`${API_BASE}/api/v1/commerce/purchase/agent-001`, {
        method: "POST",
        body: JSON.stringify({
          asset_id: asset.id,
          amount_coins: asset.price_coins,
          context_layer: "voice_registry_buy"
        })
      });

      if (res.ok) {
        setBalance(prev => prev - asset.price_coins);
        alert(`Success! Purchased ${asset.name}. DRM key has been securely deposited to your Abyss Vault.`);
      } else {
        const data = await res.json();
        alert(`Purchase failed: ${data.message || 'Insufficient funds'}`);
      }
    } catch (e) {
      alert("Purchase request failed.");
    } finally {
      setPurchasing(null);
    }
  };

  return (
    <div className="system-panel" style={{ padding: "2rem", height: "100%", overflowY: "auto" }}>
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: "2rem" }}>
        <div>
          <h3 style={{ margin: 0, color: "var(--text-primary)", display: "flex", alignItems: "center", gap: "0.5rem" }}>
            <Crown size={24} color="var(--accent-purple)" />
            Creator Registry & Voice Store
          </h3>
          <p style={{ margin: "0.5rem 0 0", color: "var(--text-secondary)", fontSize: "0.9rem" }}>
            Acquire premium XTTS voices with mathematically enforced DRM.
          </p>
        </div>
        <div style={{ 
          background: "rgba(0,0,0,0.3)", 
          padding: "0.75rem 1.5rem", 
          borderRadius: "8px",
          border: "1px solid rgba(255,255,255,0.05)",
          display: "flex",
          alignItems: "center",
          gap: "1rem"
        }}>
          <div>
            <span style={{ fontSize: "0.75rem", color: "var(--text-muted)", textTransform: "uppercase" }}>Wallet Balance</span>
            <div style={{ fontWeight: "bold", color: "var(--accent-cyan)", fontSize: "1.2rem" }}>
              {balance.toLocaleString()} KC
            </div>
          </div>
          <button className="primary-button" style={{ padding: "0.5rem 1rem", fontSize: "0.85rem" }}>
            Recharge
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
                <div style={{ fontSize: "0.8rem", color: "var(--text-muted)" }}>by {asset.author}</div>
              </div>
              <div style={{ background: "rgba(0, 242, 255, 0.1)", color: "var(--accent-cyan)", padding: "4px 8px", borderRadius: "4px", fontSize: "0.8rem", fontWeight: "bold", display: "flex", alignItems: "center", gap: "4px" }}>
                <span>{asset.price_coins}</span> KC
              </div>
            </div>

            <p style={{ fontSize: "0.9rem", color: "var(--text-secondary)", flex: 1, marginBottom: "1.5rem" }}>
              {asset.description}
            </p>

            <div style={{ display: "flex", gap: "0.5rem", marginBottom: "1.5rem", flexWrap: "wrap" }}>
              {asset.tags.map(t => (
                <span key={t} style={{ 
                  fontSize: "0.7rem", 
                  padding: "2px 8px", 
                  borderRadius: "12px", 
                  background: "rgba(255,255,255,0.05)",
                  color: "var(--text-muted)"
                }}>
                  #{t}
                </span>
              ))}
            </div>

            <div style={{ display: "flex", gap: "0.5rem" }}>
              <button 
                className="secondary-button" 
                style={{ flex: 1, display: "flex", justifyContent: "center", alignItems: "center", gap: "0.5rem" }}
              >
                <Volume2 size={16} /> Preview
              </button>
              <button 
                className="primary-button" 
                style={{ flex: 1, display: "flex", justifyContent: "center", alignItems: "center", gap: "0.5rem" }}
                onClick={() => handlePurchase(asset)}
                disabled={purchasing === asset.id || balance < asset.price_coins}
              >
                {purchasing === asset.id ? (
                  <span className="ani-pulse">Securing...</span>
                ) : (
                  <>
                    <ShoppingCart size={16} /> Purchase
                  </>
                )}
              </button>
            </div>
            {balance < asset.price_coins && (
              <div style={{ marginTop: "0.75rem", fontSize: "0.75rem", color: "var(--accent-rose)", textAlign: "center" }}>
                Insufficient funds
              </div>
            )}
            <div style={{ marginTop: "1rem", fontSize: "0.7rem", color: "var(--text-muted)", display: "flex", alignItems: "center", justifyContent: "center", gap: "0.25rem" }}>
              <ShieldCheck size={12} color="var(--accent-emerald)" />
              Powered by Abyss Security Proxy DRM
            </div>
          </motion.div>
        ))}
      </div>
    </div>
  );
}
