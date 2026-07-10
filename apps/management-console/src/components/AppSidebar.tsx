/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import React from "react";
import { motion, AnimatePresence } from "framer-motion";
import { PanelLeftClose, PanelLeftOpen } from "lucide-react";
import { NAV_GROUPS } from "../navConfig";
import { NavItem } from "./NavItem";
import { AgentStats } from "../types";
import type { WorkspacePersona } from "../hooks/useWorkspacePersona";
import type { Language } from "../i18n";

interface AppSidebarProps {
  viewMode: string;
  isMobileNav: boolean;
  isSidebarOpen: boolean;
  setIsSidebarOpen: (open: boolean) => void;
  workspacePersona: WorkspacePersona;
  isVisible: (tab: string) => boolean;
  activeTab: string;
  setActiveTab: (tab: string) => void;
  t: (key: string, options?: any) => string | any;
  navContainerRef: React.RefObject<HTMLDivElement | null>;
  stats: AgentStats;
  lang: Language;
  setLang: (lang: Language) => void;
  APP_VERSION: string;
}

export function AppSidebar({
  viewMode,
  isMobileNav,
  isSidebarOpen,
  setIsSidebarOpen,
  workspacePersona,
  isVisible,
  activeTab,
  setActiveTab,
  t,
  navContainerRef,
  stats,
  lang,
  setLang,
  APP_VERSION
}: AppSidebarProps) {
  if (viewMode !== 'cockpit') return null;

  return (
    <>
      {isMobileNav && isSidebarOpen && (
        <div
          className="sidebar-backdrop"
          onClick={() => setIsSidebarOpen(false)}
          aria-hidden="true"
        />
      )}
      <aside className={`sidebar ${isSidebarOpen ? '' : 'closed'}`}>
        <div className="brand-row">
          <img
            src={isSidebarOpen ? '/aiome-horizontal-white.png' : '/aiome-graphic-white.png'}
            alt="Aiome"
            className="brand-logo"
          />
          <button
            type="button"
            className="sidebar-toggle-btn"
            onClick={() => setIsSidebarOpen(!isSidebarOpen)}
            aria-label={t('sidebar.toggleSidebar')}
            data-tooltip={t('sidebar.toggleSidebar')}
          >
            {isSidebarOpen ? <PanelLeftClose size={20} /> : <PanelLeftOpen size={20} />}
          </button>
        </div>

        <div className="sidebar-nav-container" ref={navContainerRef}>
          {NAV_GROUPS.map((group) => {
            const visibleItems = group.items.filter((item) =>
              item.tab === 'agency'
                ? workspacePersona.mode === 'agency'
                : isVisible(item.tab)
            );
            if (visibleItems.length === 0) return null;
            return (
              <nav className="nav-group" key={group.sectionKey}>
                <h4>{t(`nav.section.${group.sectionKey}`)}</h4>
                {visibleItems.map((item) => (
                  <NavItem
                     key={item.tab}
                     tab={item.tab}
                     icon={item.icon}
                     label={t(item.labelKey)}
                     description={t(`nav.desc.${item.tab}`)}
                     active={activeTab === item.tab}
                     onClick={() => setActiveTab(item.tab)}
                  />
                ))}
              </nav>
            );
          })}
        </div>

        <AnimatePresence>
          {isSidebarOpen && (
            <motion.div
              className="sidebar-footer"
              initial={{ opacity: 0, y: 10 }}
              animate={{ opacity: 1, y: 0 }}
              exit={{ opacity: 0, y: 10 }}
              transition={{ duration: 0.2 }}
            >
              <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: '0.5rem' }}>
                <span style={{ color: 'var(--text-secondary)' }}>{t('sidebar.samsaraTier')}</span>
                <span style={{ color: 'var(--accent-purple)' }}>{t('sidebar.level')} {stats.level}</span>
              </div>
              <div style={{ height: '4px', background: 'var(--white-10)', borderRadius: '2px', overflow: 'hidden' }}>
                <motion.div
                  initial={{ width: 0 }}
                  animate={{ width: `${(stats.exp % 1000) / 10}%` }}
                  style={{ height: '100%', background: 'var(--accent-purple)' }}
                />
              </div>
              <div style={{ marginTop: '0.5rem', textAlign: 'center', fontSize: '0.65rem', color: 'var(--text-muted)' }}>
                AIOME {APP_VERSION}
              </div>
              <div style={{ display: 'flex', justifyContent: 'center', gap: '0.25rem', marginTop: '0.75rem' }}>
                <button className={`lang-btn ${lang === 'en' ? 'active' : ''}`} onClick={() => setLang('en')}>
                  🇺🇸 {t('language.en')}
                </button>
                <button className={`lang-btn ${lang === 'ja' ? 'active' : ''}`} onClick={() => setLang('ja')}>
                  🇯🇵 {t('language.ja')}
                </button>
              </div>
            </motion.div>
          )}
        </AnimatePresence>
      </aside>
    </>
  );
}
