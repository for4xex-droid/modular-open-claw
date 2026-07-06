/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import { useState, useEffect, useCallback } from "react";
import { motion, AnimatePresence } from "framer-motion";
import { API_BASE, STRIPE_PRICE_ID } from "../../config";
import { authenticatedFetch, getAuthToken } from "../../lib/auth";
import { useCheckoutSession } from "../../hooks/useCheckoutSession";
import { openProUpgradeModal } from "../../hooks/useSubscriptionStatus";
import { useTranslation, useLanguage } from "../../i18n";
import { useCoinBalance } from "../../hooks/useCoinBalance";

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
  memo?: string | null;
}

export default function NurtureDashboard({ onNavigateToStore }: { onNavigateToStore?: () => void }) {
  const { t } = useTranslation();
  const { lang } = useLanguage();
  const {
    balance: coinBalance,
    isLoading: coinBalanceLoading,
    refetch: refetchCoinBalance,
  } = useCoinBalance();
  const [balance, setBalance] = useState<PointsBalance | null>(null);
  const [history, setHistory] = useState<TransactionRecord[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

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

  const fetchData = useCallback(async (signal?: AbortSignal) => {
    setIsLoading(true);
    setError(null);
    try {
      const [ptsRes, histRes] = await Promise.all([
        authenticatedFetch(`${API_BASE}/api/v1/commerce/points/${agentId}`, { signal }),
        authenticatedFetch(`${API_BASE}/api/v1/commerce/history/${agentId}`, { signal }),
      ]);

      if (ptsRes.ok) {
        setBalance(await ptsRes.json());
      } else if (ptsRes.status === 403) {
        throw new Error(t('nurture.errorUnauthorized'));
      } else {
        throw new Error(t('nurture.errorPoints'));
      }

      if (histRes.ok) {
        setHistory(await histRes.json());
      } else if (histRes.status !== 403) {
        throw new Error(t('nurture.errorHistory'));
      }
    } catch (e: unknown) {
      if (e instanceof Error) {
        if (e.name === 'AbortError') return;
        setError(e.message || t('nurture.errorConnect'));
      } else {
        setError(t('nurture.errorConnect'));
      }
    } finally {
      setIsLoading(false);
    }
  }, [agentId, t]);

  useEffect(() => {
    const controller = new AbortController();
    void fetchData(controller.signal);
    return () => controller.abort();
  }, [fetchData]);

  const formatDate = (dateString: string) => {
    const d = new Date(dateString);
    const locale = lang === 'ja' ? 'ja-JP' : 'en-US';
    return new Intl.DateTimeFormat(locale, {
      month: "short",
      day: "2-digit",
      hour: "2-digit",
      minute: "2-digit",
    }).format(d);
  };

  const handleRetry = () => {
    void fetchData();
    void refetchCoinBalance();
  };

  return (
    <div className="system-panel" style={{ padding: "2rem", height: "100%", overflowY: "auto" }}>
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: "2rem", flexWrap: "wrap", gap: "1rem" }}>
        <div>
          <h3 style={{ margin: 0, color: "var(--text-primary)", display: "flex", alignItems: "center", gap: "0.5rem" }}>
            <Wallet size={24} color="var(--accent-purple)" />
            {t('nurture.title')}
          </h3>
          <p style={{ margin: "0.5rem 0 0", color: "var(--text-secondary)", fontSize: "0.9rem" }}>
            {t('nurture.subtitle')}
          </p>
        </div>
        <div style={{ display: "flex", gap: "1rem", flexWrap: "wrap" }}>
          <div className="config-card" style={{
            display: "flex",
            flexDirection: "column",
            gap: "0.35rem",
            padding: "0.75rem 1rem",
            borderColor: "var(--accent-emerald-30)",
          }}>
            <span style={{ fontSize: "0.65rem", fontWeight: 700, color: "var(--accent-emerald)", textTransform: "uppercase" }}>
              {t('nurture.kcPointsSection')}
            </span>
            <button
              className="primary-button"
              onClick={handleCheckout}
              disabled={isLoading || isCheckoutLoading}
              style={{ display: "flex", alignItems: "center", gap: "0.5rem", background: "var(--accent-emerald)", color: "var(--black-100)" }}
            >
              {isCheckoutLoading ? t('nurture.loading') : t('nurture.buyPoints')}
            </button>
            <span style={{ fontSize: "0.7rem", color: "var(--text-muted)", maxWidth: "200px" }}>
              {t('nurture.buyPointsHint')}
            </span>
          </div>
          <div className="config-card" style={{
            display: "flex",
            flexDirection: "column",
            gap: "0.35rem",
            padding: "0.75rem 1rem",
            borderColor: "var(--accent-purple-30)",
          }}>
            <span style={{ fontSize: "0.65rem", fontWeight: 700, color: "var(--accent-purple)", textTransform: "uppercase" }}>
              {t('nurture.proSection')}
            </span>
            <button
              className="primary-button"
              onClick={() => openProUpgradeModal()}
              style={{ display: "flex", alignItems: "center", gap: "0.5rem" }}
            >
              {t('nurture.upgradePro')}
            </button>
          </div>
          <button
            className="secondary-button"
            onClick={() => onNavigateToStore?.()}
            style={{ display: "flex", alignItems: "center", gap: "0.5rem", borderColor: "var(--accent-purple)", color: "var(--accent-purple)", alignSelf: "flex-end" }}
          >
            <Wallet size={16} />
            {t('nurture.viewStore')}
          </button>
          <button
            className="secondary-button"
            onClick={handleRetry}
            disabled={isLoading}
            style={{ display: "flex", alignItems: "center", gap: "0.5rem", alignSelf: "flex-end" }}
          >
            <RefreshCcw size={16} className={isLoading ? "ani-spin" : ""} />
            {t('nurture.refresh')}
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
              justifyContent: "space-between",
              gap: "1rem",
              flexWrap: "wrap",
            }}
          >
            <div style={{ display: "flex", alignItems: "center", gap: "0.5rem" }}>
              <ShieldCheck size={20} />
              <span style={{ fontWeight: 600 }}>{error}</span>
            </div>
            <button
              type="button"
              className="secondary-button"
              onClick={handleRetry}
              disabled={isLoading}
              style={{ borderColor: "var(--accent-rose-30)", color: "var(--accent-rose)" }}
            >
              {t('error.retry')}
            </button>
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
            <span style={{ fontSize: "0.9rem", fontWeight: 600, textTransform: "uppercase" }}>{t('nurture.aiomeCoin')}</span>
          </div>
          <div style={{ fontSize: "2.5rem", fontWeight: 800, color: "var(--accent-emerald)" }}>
            {coinBalanceLoading ? "..." : coinBalance.toLocaleString()} <span style={{ fontSize: "1.2rem" }}>KC</span>
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
            <span style={{ fontSize: "0.9rem", fontWeight: 600, textTransform: "uppercase" }}>{t('nurture.pointsBalance')}</span>
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
            <span style={{ fontSize: "0.9rem", fontWeight: 600, textTransform: "uppercase" }}>{t('nurture.lifetimeEarned')}</span>
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
            <span style={{ fontSize: "0.9rem", fontWeight: 600, textTransform: "uppercase" }}>{t('nurture.convertedToCoin')}</span>
          </div>
          <div style={{ fontSize: "2.5rem", fontWeight: 800, color: "var(--accent-rose)" }}>
            {isLoading ? "..." : (balance?.lifetime_withdrawn || 0).toLocaleString()} <span style={{ fontSize: "1.2rem" }}>KP</span>
          </div>
        </motion.div>
      </div>

      <div className="config-card" style={{ padding: 0, overflow: "hidden" }}>
        <div style={{ padding: "1.5rem", borderBottom: "1px solid var(--white-10)", display: "flex", alignItems: "center", gap: "0.5rem" }}>
          <History size={20} color="var(--accent-cyan)" />
          <h4 style={{ margin: 0, color: "var(--text-primary)", fontSize: "1.1rem" }}>{t('nurture.ledgerHistory')}</h4>
        </div>
        
        {isLoading && history.length === 0 ? (
          <div style={{ padding: "3rem", textAlign: "center", color: "var(--text-muted)" }}>
            <RefreshCcw size={24} className="ani-spin" style={{ margin: "0 auto 1rem" }} />
            {t('nurture.loadingTransactions')}
          </div>
        ) : history.length === 0 ? (
          <div style={{ padding: "3rem", textAlign: "center", color: "var(--text-muted)" }}>
            {t('nurture.noTransactions')}
          </div>
        ) : (
          <div style={{ overflowX: "auto" }}>
            <table style={{ width: "100%", borderCollapse: "collapse", fontSize: "0.9rem" }}>
              <thead>
                <tr style={{ background: "var(--black-20)", textAlign: "left" }}>
                  <th style={{ padding: "1rem 1.5rem", color: "var(--text-muted)", fontWeight: 600 }}>{t('nurture.tableType')}</th>
                  <th style={{ padding: "1rem 1.5rem", color: "var(--text-muted)", fontWeight: 600 }}>{t('nurture.tablePoints')}</th>
                  <th style={{ padding: "1rem 1.5rem", color: "var(--text-muted)", fontWeight: 600 }}>{t('nurture.tableCoins')}</th>
                  <th style={{ padding: "1rem 1.5rem", color: "var(--text-muted)", fontWeight: 600 }}>{t('nurture.tableTxId')}</th>
                  <th style={{ padding: "1rem 1.5rem", color: "var(--text-muted)", fontWeight: 600 }}>{t('nurture.tableMemo')}</th>
                  <th style={{ padding: "1rem 1.5rem", color: "var(--text-muted)", fontWeight: 600 }}>{t('nurture.tableDate')}</th>
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
                            <ArrowUpRight size={14} /> {t('nurture.received')}
                          </span>
                        ) : (
                          <span style={{ color: "var(--accent-rose)", display: "flex", alignItems: "center", gap: "4px", background: "var(--accent-rose-10)", padding: "4px 8px", borderRadius: "4px", fontSize: "0.8rem" }}>
                            <ArrowDownRight size={14} /> {t('nurture.sent')}
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
                      <td style={{ padding: "1rem 1.5rem", color: "var(--text-secondary)", fontSize: "0.85rem" }}>
                        {record.memo ?? "—"}
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
