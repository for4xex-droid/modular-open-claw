/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import React from "react";
import { motion } from "framer-motion";

interface NavItemProps {
  tab: string;
  icon: React.ReactNode;
  label: string;
  description?: string;
  active: boolean;
  onClick: () => void;
}

export function NavItem({ tab, icon, label, description, active, onClick }: NavItemProps) {
  return (
    <button
      type="button"
      className={`nav-item ${active ? 'active' : ''}`}
      data-testid={`nav-${tab}`}
      onClick={onClick}
      title={description}
    >
      {icon}
      <span className="nav-item-text">
        <span className="nav-item-label">{label}</span>
        {description && <span className="nav-item-desc">{description}</span>}
      </span>
      {active && <motion.div layoutId="active-pill" className="nav-active-bar" />}
    </button>
  );
}
