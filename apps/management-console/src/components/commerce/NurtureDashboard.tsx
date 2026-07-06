/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import { useState, useEffect } from "react";
import { motion, AnimatePresence } from "framer-motion";
import { API_BASE, STRIPE_PRICE_ID } from "../../config";
import { authenticatedFetch, getAuthToken } from "../../lib/auth";
import { useCheckoutSession } from "../../hooks/useCheckoutSession";
import { openProUpgradeModal } from "../../hooks/useSubscriptionStatus";

import {
  Wallet,
  ArrowUpRight,
  ArrowDownRight,
  History,
  TrendingUp,
  Award,
  Clock,
  ShieldCheck,
  RefreshCcw
} from "lucide-react";

interface PointsBalance {
  balance: number;
  lifetime_earned: number;
  lifetime_withdrawn: number;
  conversion_rate_bps: number;
}

interface TransactionRecord {
  id: string;
  transaction_id: string;
  debit_account: string;
  credit_account: string;
  coin_amount: number;
  points_amount: number;
  entry_type: string;
  created_at: string;
}

export default function NurtureDashboard({ onNavigateToStore }: { onNavigateToStore?: () => void }) {
  // useTranslation is available if needed
  // const { t } = useTranslation();
  const [balance, setBalance] = useState<PointsBalance | null>(null);
  const [coinBalance, setCoinBalance] = useState<number | null>(null);
  const [history, setHistory] = useState<TransactionRecord[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  // Extract agent_id from JWT token, or fallback to mock
  const token = getAuthToken();
  let agentId = "agent-001";
  if (token) {
    try {
      const payload = JSON.parse(atob(token.split('.')[1]));
      agentId = payload.sub || payload.agent_id || "agent-001";
    } catch {
      // ignore
    }
  }

  const { handleCheckout, isLoading: isCheckoutLoading, error: checkoutError } = useCheckoutSession(STRIPE_PRICE_ID, agentId);

  useEffect(() => {
    if (checkoutError) {
      setError(checkoutError);
    }
  }, [checkoutError]);

  const fetchData = async (signal?: AbortSignal) => {
    setIsLoading(true);
    setError(null);
    try {
      const [ptsRes, histRes, kcRes] = await Promise.all([
        authenticatedFetch(`${API_BASE}/api/v1/commerce/points/${agentId}`, { signal }),
        authenticatedFetch(`${API_BASE}/api/v1/commerce/history/${agentId}`, { signal }),
        authenticatedFetch(`${API_BASE}/api/v1/commerce/balance/${agentId}`, { signal }),
      ]);

      if (ptsRes.ok) {
        setBalance(await ptsRes.json());
      } else if (ptsRes.status === 403) {
        throw new Error("Unauthorized: Access denied to this agent's ledger.");
      } else {
        throw new Error("Failed to load points balance.");
      }

      if (histRes.ok) {
        setHistory(await histRes.json());
      } else if (histRes.status !== 403) {
        throw new Error("Failed to load transaction history.");
      }

      if (kcRes.ok) {
        const kcData = await kcRes.json();
        setCoinBalance(typeof kcData.balance === "number" ? kcData.balance : 0);
      } else if (kcRes.status !== 403) {
        setCoinBalance(null);
      }
    } catch (e: unknown) {
      if (e instanceof Error) {
        if (e.name === 'AbortError') return;
        setError(e.message || "Failed to connect to Nurture Engine.");
      } else {
        setError("Failed to connect to Nurture Engine.");
      }
    } finally {
      setIsLoading(false);
    }
  };

  useEffect(() => {
    const controller = new AbortController();
    fetchData(controller.signal);
    return () => controller.abort();
  }, []);

  const formatDate = (dateString: string) => {
    const d = new Date(dateString);
    return new Intl.DateTimeFormat("en-US", {
      month: "short",
      day: "2-digit",
      hour: "2-digit",
      minute: "2-digit",
    }).format(d);
  };

  return (
    <div className="system-panel" style={{ padding: "2rem", height: "100%", overflowY: "auto" }}>
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: "2rem" }}>
        <div>
          <h3 style={{ margin: 0, color: "var(--text-primary)", display: "flex", alignItems: "center", gap: "0.5rem" }}>
            <Wallet size={24} color="var(--accent-purple)" />
            Nurture Economy Engine
          </h3>
          <p style={{ margin: "0.5rem 0 0", color: "var(--text-secondary)", fontSize: "0.9rem" }}>
            Real-time tracking of Experience points and transaction ledgers.
          </p>
        </div>
        <div style={{ display: "flex", gap: "1rem", flexWrap: "wrap" }}>
          <div style={{
            display: "flex",
            flexDirection: "column",
            gap: "0.35rem",
            padding: "0.75rem 1rem",
            borderRadius: "var(--radius-md)",
            border: "1px solid var(--accent-emerald-30)",
            background: "var(--black-20)",
          }}>
            <span style={{ fontSize: "0.65rem", fontWeight: 700, color: "var(--accent-emerald)", textTransform: "uppercase" }}>
              KC / Points
            </span>
            <button
              className="primary-button"
              onClick={handleCheckout}
              disabled={isLoading || isCheckoutLoading}
              style={{ display: "flex", alignItems: "center", gap: "0.5rem", background: "var(--accent-emerald)", color: "var(--black-100)" }}
            >
              {isCheckoutLoading ? "Loading..." : "Buy Points (KC)"}
            </button>
            <span style={{ fontSize: "0.7rem", color: "var(--text-muted)", maxWidth: "200px" }}>
              Karma Coin / experience points — separate from Pro subscription.
            </span>
          </div>
          <div style={{
            display: "flex",
            flexDirection: "column",
            gap: "0.35rem",
            padding: "0.75rem 1rem",
            borderRadius: "var(--radius-md)",
            border: "1px solid var(--accent-purple-30)",
            background: "var(--black-20)",
          }}>
            <span style={{ fontSize: "0.65rem", fontWeight: 700, color: "var(--accent-purple)", textTransform: "uppercase" }}>
              Aiome Pro
            </span>
            <button
              className="primary-button"
              onClick={() => openProUpgradeModal()}
              style={{ display: "flex", alignItems: "center", gap: "0.5rem" }}
            >
              Upgrade to Pro
            </button>
          </div>
          <button
            className="secondary-button"
            onClick={() => onNavigateToStore?.()}
            style={{ display: "flex", alignItems: "center", gap: "0.5rem", borderColor: "var(--accent-purple)", color: "var(--accent-purple)", alignSelf: "flex-end" }}
          >
            <Wallet size={16} />
            View Store
          </button>
          <button
            className="secondary-button"
            onClick={() => fetchData()}
            disabled={isLoading}
            style={{ display: "flex", alignItems: "center", gap: "0.5rem", alignSelf: "flex-end" }}
          >
            <RefreshCcw size={16} className={isLoading ? "ani-spin" : ""} />
            Refresh
          </button>
        </div>
      </div>

      <AnimatePresence>
        {error && (
          <motion.div
            initial={{ opacity: 0, y: -10 }}
            animate={{ opacity: 1, y: 0 }}
            exit={{ opacity: 0 }}
            style={{
              background: "var(--accent-rose-10)",
              border: "1px solid var(--accent-rose-30)",
              padding: "1rem",
              borderRadius: "var(--radius-md)",
              color: "var(--accent-rose)",
              marginBottom: "2rem",
              display: "flex",
              alignItems: "center",
              gap: "0.5rem",
            }}
          >
            <ShieldCheck size={20} />
            <span style={{ fontWeight: 600 }}>{error}</span>
          </motion.div>
        )}
      </AnimatePresence>

      <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(240px, 1fr))", gap: "1.5rem", marginBottom: "2rem" }}>
        <motion.div
          initial={{ opacity: 0, scale: 0.95 }}
          animate={{ opacity: 1, scale: 1 }}
          transition={{ delay: 0.05 }}
          className="config-card"
          style={{ background: "linear-gradient(145deg, var(--black-30), var(--black-50))", border: "1px solid var(--accent-emerald-30)" }}
        >
          <div style={{ display: "flex", alignItems: "center", gap: "0.75rem", marginBottom: "1rem", color: "var(--text-secondary)" }}>
            <Wallet size={20} color="var(--accent-emerald)" />
            <span style={{ fontSize: "0.9rem", fontWeight: 600, textTransform: "uppercase" }}>AiomeCoin (KC)</span>
          </div>
          <div style={{ fontSize: "2.5rem", fontWeight: 800, color: "var(--accent-emerald)" }}>
            {isLoading ? "..." : (coinBalance ?? 0).toLocaleString()} <span style={{ fontSize: "1.2rem" }}>KC</span>
          </div>
        </motion.div>
        <motion.div
          initial={{ opacity: 0, scale: 0.95 }}
          animate={{ opacity: 1, scale: 1 }}
          transition={{ delay: 0.1 }}
          className="config-card"
          style={{ background: "linear-gradient(145deg, var(--black-30), var(--black-50))", border: "1px solid var(--accent-purple-30)" }}
        >
          <div style={{ display: "flex", alignItems: "center", gap: "0.75rem", marginBottom: "1rem", color: "var(--text-secondary)" }}>
            <Award size={20} color="var(--accent-purple)" />
            <span style={{ fontSize: "0.9rem", fontWeight: 600, textTransform: "uppercase" }}>ポイント残高</span>
          </div>
          <div style={{ fontSize: "2.5rem", fontWeight: 800, color: "var(--accent-purple)" }}>
            {isLoading ? "..." : (balance?.balance || 0).toLocaleString()} <span style={{ fontSize: "1.2rem" }}>KP</span>
          </div>
        </motion.div>

        <motion.div
          initial={{ opacity: 0, scale: 0.95 }}
          animate={{ opacity: 1, scale: 1 }}
          transition={{ delay: 0.2 }}
          className="config-card"
        >
          <div style={{ display: "flex", alignItems: "center", gap: "0.75rem", marginBottom: "1rem", color: "var(--text-secondary)" }}>
            <TrendingUp size={20} color="var(--accent-cyan)" />
            <span style={{ fontSize: "0.9rem", fontWeight: 600, textTransform: "uppercase" }}>Lifetime Earned</span>
          </div>
          <div style={{ fontSize: "2.5rem", fontWeight: 800, color: "var(--accent-cyan)" }}>
            {isLoading ? "..." : (balance?.lifetime_earned || 0).toLocaleString()} <span style={{ fontSize: "1.2rem" }}>KP</span>
          </div>
        </motion.div>

        <motion.div
          initial={{ opacity: 0, scale: 0.95 }}
          animate={{ opacity: 1, scale: 1 }}
          transition={{ delay: 0.3 }}
          className="config-card"
        >
          <div style={{ display: "flex", alignItems: "center", gap: "0.75rem", marginBottom: "1rem", color: "var(--text-secondary)" }}>
            <ArrowDownRight size={20} color="var(--accent-rose)" />
            <span style={{ fontSize: "0.9rem", fontWeight: 600, textTransform: "uppercase" }}>Lifetime Withdrawn</span>
          </div>
          <div style={{ fontSize: "2.5rem", fontWeight: 800, color: "var(--accent-rose)" }}>
            {isLoading ? "..." : (balance?.lifetime_withdrawn || 0).toLocaleString()} <span style={{ fontSize: "1.2rem" }}>KP</span>
          </div>
        </motion.div>
      </div>

      <div className="config-card" style={{ padding: 0, overflow: "hidden" }}>
        <div style={{ padding: "1.5rem", borderBottom: "1px solid var(--white-10)", display: "flex", alignItems: "center", gap: "0.5rem" }}>
          <History size={20} color="var(--accent-cyan)" />
          <h4 style={{ margin: 0, color: "var(--text-primary)", fontSize: "1.1rem" }}>Ledger History</h4>
        </div>
        
        {isLoading && history.length === 0 ? (
          <div style={{ padding: "3rem", textAlign: "center", color: "var(--text-muted)" }}>
            <RefreshCcw size={24} className="ani-spin" style={{ margin: "0 auto 1rem" }} />
            Loading transactions...
          </div>
        ) : history.length === 0 ? (
          <div style={{ padding: "3rem", textAlign: "center", color: "var(--text-muted)" }}>
            No transactions found.
          </div>
        ) : (
          <div style={{ overflowX: "auto" }}>
            <table style={{ width: "100%", borderCollapse: "collapse", fontSize: "0.9rem" }}>
              <thead>
                <tr style={{ background: "var(--black-20)", textAlign: "left" }}>
                  <th style={{ padding: "1rem 1.5rem", color: "var(--text-muted)", fontWeight: 600 }}>Type</th>
                  <th style={{ padding: "1rem 1.5rem", color: "var(--text-muted)", fontWeight: 600 }}>Amount (Points)</th>
                  <th style={{ padding: "1rem 1.5rem", color: "var(--text-muted)", fontWeight: 600 }}>Amount (Coins)</th>
                  <th style={{ padding: "1rem 1.5rem", color: "var(--text-muted)", fontWeight: 600 }}>Transaction ID</th>
                  <th style={{ padding: "1rem 1.5rem", color: "var(--text-muted)", fontWeight: 600 }}>Date</th>
                </tr>
              </thead>
              <tbody>
                {history.map((record, i) => {
                  const isCredit = record.credit_account.includes(agentId);
                  return (
                    <motion.tr
                      key={record.id}
                      initial={{ opacity: 0, y: 10 }}
                      animate={{ opacity: 1, y: 0 }}
                      transition={{ delay: i * 0.05 }}
                      style={{ borderBottom: "1px solid var(--white-05)" }}
                    >
                      <td style={{ padding: "1rem 1.5rem", display: "flex", alignItems: "center", gap: "0.5rem" }}>
                        {isCredit ? (
                          <span style={{ color: "var(--accent-emerald)", display: "flex", alignItems: "center", gap: "4px", background: "var(--accent-emerald-10)", padding: "4px 8px", borderRadius: "4px", fontSize: "0.8rem" }}>
                            <ArrowUpRight size={14} /> Received
                          </span>
                        ) : (
                          <span style={{ color: "var(--accent-rose)", display: "flex", alignItems: "center", gap: "4px", background: "var(--accent-rose-10)", padding: "4px 8px", borderRadius: "4px", fontSize: "0.8rem" }}>
                            <ArrowDownRight size={14} /> Sent
                          </span>
                        )}
                        <span style={{ color: "var(--text-secondary)", fontSize: "0.85rem", textTransform: "capitalize" }}>
                          {record.entry_type.replace(/\"/g, "").replace("_", " ")}
                        </span>
                      </td>
                      <td style={{ padding: "1rem 1.5rem", fontWeight: 700, color: isCredit ? "var(--accent-emerald)" : "var(--text-primary)" }}>
                        {isCredit ? "+" : "-"}{record.points_amount}
                      </td>
                      <td style={{ padding: "1rem 1.5rem", color: "var(--text-secondary)" }}>
                        {record.coin_amount > 0 ? `${isCredit ? "+" : "-"}${record.coin_amount}` : "-"}
                      </td>
                      <td style={{ padding: "1rem 1.5rem", color: "var(--text-muted)", fontFamily: "monospace" }}>
                        {record.transaction_id.substring(0, 8)}...
                      </td>
                      <td style={{ padding: "1rem 1.5rem", color: "var(--text-secondary)", display: "flex", alignItems: "center", gap: "0.5rem" }}>
                        <Clock size={14} />
                        {formatDate(record.created_at)}
                      </td>
                    </motion.tr>
                  );
                })}
              </tbody>
            </table>
          </div>
        )}
      </div>
    </div>
  );
}
