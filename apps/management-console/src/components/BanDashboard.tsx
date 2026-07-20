/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

import { useState, useEffect } from "react";
import { motion, AnimatePresence } from "framer-motion";
import { ShieldAlert, UserX, ShieldCheck, UserCheck, AlertTriangle, Search } from "lucide-react";
import { API_BASE } from "../config";
import { authenticatedFetch } from "../lib/auth";
import { useTranslation } from "../i18n";
import { useToast } from "./common/Toast";
import ConfirmModal from "./common/ConfirmModal";

/** API body default — must stay English regardless of UI locale (OP-021 §4.1). */
const DEFAULT_BAN_REASON = "Policy violation";

async function readErrorMessage(res: Response): Promise<string | undefined> {
  try {
    const data = await res.json();
    return typeof data?.message === "string" ? data.message : undefined;
  } catch {
    return undefined;
  }
}

interface BanRecord {
  actor_id: string;
  reason: string;
  severity: string;
  banned_by: string;
  banned_at: string;
  expires_at?: string | null;
  unbanned_at?: string | null;
}

export default function BanDashboard() {
  const { t } = useTranslation();
  const { showToast } = useToast();
  const [bans, setBans] = useState<BanRecord[]>([]);
  const [loading, setLoading] = useState(true);
  const [targetId, setTargetId] = useState("");
  const [reason, setReason] = useState("");
  const [severity, setSeverity] = useState("HIGH");
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [searchQuery, setSearchQuery] = useState("");
  const [unbanTargetId, setUnbanTargetId] = useState<string | null>(null);

  useEffect(() => {
    fetchBans();
  }, []);

  const fetchBans = async () => {
    setLoading(true);
    try {
      const res = await authenticatedFetch(`${API_BASE}/api/v1/admin/bans`);
      if (res.ok) {
        const data = await res.json();
        if (Array.isArray(data)) {
          setBans(data);
        }
      } else {
        showToast("error", t("ban.errorFetch"));
      }
    } catch (e) {
      showToast("error", t("common.networkError"));
    } finally {
      setLoading(false);
    }
  };

  const handleBan = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!targetId.trim()) {
      showToast("error", t("ban.errorTargetRequired"));
      return;
    }

    setIsSubmitting(true);
    try {
      const res = await authenticatedFetch(`${API_BASE}/api/v1/admin/ban`, {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
        },
        body: JSON.stringify({
          agent_id: targetId.trim(),
          reason: reason.trim() || DEFAULT_BAN_REASON,
          severity: severity,
        }),
      });

      if (res.ok) {
        showToast("success", t("ban.successBan"));
        setTargetId("");
        setReason("");
        fetchBans();
      } else {
        const message = await readErrorMessage(res);
        showToast("error", message || t("ban.errorBan"));
      }
    } catch (err) {
      showToast("error", t("common.networkError"));
    } finally {
      setIsSubmitting(false);
    }
  };

  const handleUnban = async (actorId: string) => {
    try {
      const res = await authenticatedFetch(`${API_BASE}/api/v1/admin/unban`, {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
        },
        body: JSON.stringify({
          agent_id: actorId,
        }),
      });

      if (res.ok) {
        showToast("success", t("ban.successUnban"));
        fetchBans();
      } else {
        const message = await readErrorMessage(res);
        showToast("error", message || t("ban.errorUnban"));
      }
    } catch (err) {
      showToast("error", t("common.networkError"));
    } finally {
      setUnbanTargetId(null);
    }
  };

  const filteredBans = bans.filter(
    (b) =>
      b.actor_id.toLowerCase().includes(searchQuery.toLowerCase()) ||
      b.reason.toLowerCase().includes(searchQuery.toLowerCase())
  );

  return (
    <div className="system-panel" style={{ padding: "2rem", height: "100%", overflowY: "auto" }}>
      <ConfirmModal
        isOpen={!!unbanTargetId}
        type="warning"
        title={t("ban.confirmTitle")}
        message={t("ban.confirmMessage")}
        confirmText={t("ban.unban")}
        cancelText={t("common.cancel")}
        onConfirm={() => unbanTargetId && handleUnban(unbanTargetId)}
        onCancel={() => setUnbanTargetId(null)}
      />
      {/* Header */}
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: "2rem" }}>
        <div>
          <h3 style={{ margin: 0, color: "var(--text-primary)", display: "flex", alignItems: "center", gap: "0.5rem" }}>
            <ShieldAlert size={24} color="var(--accent-rose)" />
            {t("ban.title")}
          </h3>
          <p style={{ margin: "0.5rem 0 0", color: "var(--text-secondary)", fontSize: "0.9rem" }}>
            {t("ban.subtitle")}
          </p>
        </div>
      </div>

      <div style={{ display: "grid", gridTemplateColumns: "1fr 2fr", gap: "2rem", alignItems: "start" }}>
        {/* Ban Form */}
        <motion.div
          initial={{ opacity: 0, x: -20 }}
          animate={{ opacity: 1, x: 0 }}
          className="config-card"
          style={{ padding: "1.5rem", border: "1px solid var(--accent-rose-30)", background: "var(--accent-rose-05)" }}
        >
          <h4 style={{ margin: "0 0 1.5rem", color: "var(--accent-rose)", display: "flex", alignItems: "center", gap: "0.5rem" }}>
            <UserX size={20} />
            {t("ban.formTitle")}
          </h4>
          <form onSubmit={handleBan} style={{ display: "flex", flexDirection: "column", gap: "1rem" }}>
            <div>
              <label style={{ display: "block", fontSize: "0.8rem", color: "var(--text-muted)", marginBottom: "4px" }}>
                {t("ban.targetLabel")}
              </label>
              <input
                type="text"
                className="input-field"
                placeholder="00000000-0000-0000-0000-000000000000"
                value={targetId}
                onChange={(e) => setTargetId(e.target.value)}
                style={{ width: "100%", fontFamily: "monospace" }}
              />
            </div>

            <div>
              <label style={{ display: "block", fontSize: "0.8rem", color: "var(--text-muted)", marginBottom: "4px" }}>
                {t("ban.reasonLabel")}
              </label>
              <textarea
                className="input-field"
                placeholder={t("ban.reasonPlaceholder")}
                value={reason}
                onChange={(e) => setReason(e.target.value)}
                style={{ width: "100%", height: "80px", resize: "none" }}
              />
            </div>

            <div>
              <label style={{ display: "block", fontSize: "0.8rem", color: "var(--text-muted)", marginBottom: "4px" }}>
                {t("ban.severityLabel")}
              </label>
              <select
                className="input-field"
                value={severity}
                onChange={(e) => setSeverity(e.target.value)}
                style={{ width: "100%" }}
              >
                <option value="LOW">{t("ban.severityLow")}</option>
                <option value="MEDIUM">{t("ban.severityMedium")}</option>
                <option value="HIGH">{t("ban.severityHigh")}</option>
                <option value="CRITICAL">{t("ban.severityCritical")}</option>
              </select>
            </div>

            <button
              type="submit"
              className="primary-button"
              style={{
                background: "var(--accent-rose)",
                borderColor: "var(--accent-rose-50)",
                color: "var(--bg-primary)",
                marginTop: "0.5rem",
                width: "100%",
                fontWeight: "bold",
              }}
              disabled={isSubmitting}
            >
              {isSubmitting ? t("ban.submitting") : t("ban.submit")}
            </button>
          </form>
        </motion.div>

        {/* Ban List */}
        <motion.div
          initial={{ opacity: 0, x: 20 }}
          animate={{ opacity: 1, x: 0 }}
          className="config-card"
          style={{ display: "flex", flexDirection: "column", height: "550px", overflow: "hidden" }}
        >
          {/* List Search */}
          <div style={{ display: "flex", gap: "1rem", padding: "1.2rem", borderBottom: "1px solid var(--white-05)" }}>
            <div style={{ position: "relative", flex: 1 }}>
              <Search
                size={16}
                color="var(--text-muted)"
                style={{ position: "absolute", left: "10px", top: "50%", transform: "translateY(-50%)" }}
              />
              <input
                type="text"
                className="input-field"
                placeholder={t("ban.searchPlaceholder")}
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                style={{ width: "100%", paddingLeft: "2.2rem" }}
              />
            </div>
          </div>

          {/* Table Container */}
          <div style={{ flex: 1, overflowY: "auto", padding: "1rem" }}>
            {loading ? (
              <div style={{ display: "flex", justifyContent: "center", padding: "3rem", color: "var(--text-secondary)" }}>
                {t("ban.loading")}
              </div>
            ) : filteredBans.length === 0 ? (
              <div style={{ textAlign: "center", padding: "4rem", color: "var(--text-muted)", fontSize: "0.9rem" }}>
                <ShieldCheck size={48} color="var(--accent-emerald)" style={{ marginBottom: "1rem", opacity: 0.5 }} />
                {t("ban.empty")}
              </div>
            ) : (
              <div style={{ display: "flex", flexDirection: "column", gap: "1rem" }}>
                <AnimatePresence>
                  {filteredBans.map((ban) => {
                    const isActive = !ban.unbanned_at;
                    return (
                      <motion.div
                        key={ban.actor_id}
                        layout
                        initial={{ opacity: 0, y: 10 }}
                        animate={{ opacity: 1, y: 0 }}
                        exit={{ opacity: 0, scale: 0.95 }}
                        style={{
                          background: "var(--black-20)",
                          borderRadius: "8px",
                          border: isActive ? "1px solid var(--accent-rose-20)" : "1px solid var(--white-05)",
                          padding: "1rem",
                          display: "flex",
                          justifyContent: "space-between",
                          alignItems: "center",
                          opacity: isActive ? 1 : 0.6,
                        }}
                      >
                        <div style={{ display: "flex", gap: "1rem", alignItems: "flex-start", flex: 1 }}>
                          <div
                            style={{
                              background: isActive ? "var(--accent-rose-10)" : "var(--white-05)",
                              padding: "0.5rem",
                              borderRadius: "6px",
                              display: "flex",
                              alignItems: "center",
                              justifyContent: "center",
                            }}
                          >
                            {isActive ? (
                              <AlertTriangle size={20} color="var(--accent-rose)" />
                            ) : (
                              <UserCheck size={20} color="var(--accent-emerald)" />
                            )}
                          </div>
                          <div style={{ display: "flex", flexDirection: "column", gap: "0.25rem", width: "70%" }}>
                            <div style={{ fontSize: "0.85rem", color: "var(--text-primary)", fontFamily: "monospace", overflow: "hidden", textOverflow: "ellipsis" }}>
                              {ban.actor_id}
                            </div>
                            <div style={{ fontSize: "0.85rem", color: "var(--text-secondary)", wordBreak: "break-all" }}>
                              {ban.reason}
                            </div>
                            <div style={{ display: "flex", gap: "0.5rem", fontSize: "0.7rem", color: "var(--text-muted)" }}>
                              <span style={{ color: ban.severity === "CRITICAL" ? "var(--accent-rose)" : "var(--text-muted)", fontWeight: "bold" }}>
                                {ban.severity}
                              </span>
                              <span>•</span>
                              <span>{t("ban.issued")} {new Date(ban.banned_at).toLocaleString()}</span>
                              {ban.unbanned_at && (
                                <>
                                  <span>•</span>
                                  <span style={{ color: "var(--accent-emerald)" }}>
                                    {t("ban.lifted")} {new Date(ban.unbanned_at).toLocaleString()}
                                  </span>
                                </>
                              )}
                            </div>
                          </div>
                        </div>

                        {isActive && (
                          <button
                            className="secondary-button"
                            onClick={() => setUnbanTargetId(ban.actor_id)}
                            style={{
                              padding: "0.4rem 0.8rem",
                              fontSize: "0.8rem",
                              borderColor: "var(--accent-emerald-30)",
                              color: "var(--accent-emerald)",
                              background: "transparent",
                            }}
                          >
                            {t("ban.unban")}
                          </button>
                        )}
                      </motion.div>
                    );
                  })}
                </AnimatePresence>
              </div>
            )}
          </div>
        </motion.div>
      </div>
    </div>
  );
}
