/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import { Component, ErrorInfo, ReactNode } from "react";
import { ShieldAlert, RefreshCw, Home } from "lucide-react";
import { motion } from "framer-motion";

interface Props {
  /**
   * Optional custom title displayed when an error occurs.
   * Falls back to the default "Neural Sync Interrupted".
   */
  errorTitle?: string;
  children?: ReactNode;
}

interface State {
  hasError: boolean;
  error?: Error;
}

class ErrorBoundary extends Component<Props, State> {
  public state: State = {
    hasError: false
  };

  public static getDerivedStateFromError(error: Error): State {
    return { hasError: true, error };
  }

  public componentDidCatch(error: Error, errorInfo: ErrorInfo) {
    console.error("Uncaught error:", error, errorInfo);
  }

  public render() {
    if (this.state.hasError) {
      return (
        <div style={{
          height: '100vh',
          width: '100vw',
          background: 'linear-gradient(135deg, var(--bg-dark-sidebar) 0%, var(--bg-dark) 100%)',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          color: 'var(--text-primary)',
          fontFamily: "var(--font-main, 'Inter', system-ui, -apple-system, sans-serif)"
        }}>
          <motion.div
            initial={{ opacity: 0, scale: 0.9 }}
            animate={{ opacity: 1, scale: 1 }}
            style={{
              maxWidth: '500px',
              padding: '3rem',
              background: 'var(--white-03)',
              borderRadius: '24px',
              border: '1px solid var(--accent-rose-20)',
              textAlign: 'center',
              backdropFilter: 'blur(20px)',
              boxShadow: 'var(--shadow-deep)'
            }}
          >
            <div style={{ display: 'flex', justifyContent: 'center', marginBottom: '1.5rem' }}>
              <div style={{ 
                width: '64px', height: '64px', borderRadius: '50%', background: 'var(--accent-rose-10)',
                display: 'flex', alignItems: 'center', justifyContent: 'center', border: '1px solid var(--accent-rose-30)'
              }}>
                <ShieldAlert color="var(--accent-rose)" size={32} />
              </div>
            </div>

            <h1 style={{ fontSize: '1.8rem', fontWeight: 800, fontFamily: "var(--font-display, 'Outfit', sans-serif)", marginBottom: '1rem', letterSpacing: '-0.02em' }}>
              {this.props.errorTitle ?? 'Neural Sync Interrupted'}
            </h1>
            <p style={{ color: 'var(--white-60)', marginBottom: '2rem', fontSize: '0.95rem', lineHeight: 1.6 }}>
              A fatal exception occurred in the neural interface. The system has initiated protective isolation to preserve data integrity.
            </p>

            <div style={{ display: 'flex', gap: '1rem', justifyContent: 'center' }}>
              <button
                onClick={() => window.location.reload()}
                style={{
                  padding: '0.8rem 1.5rem', borderRadius: '12px', background: 'var(--accent-cyan, var(--accent-cyan))',
                  border: 'none', color: 'var(--bg-primary)', fontWeight: 700, display: 'flex', alignItems: 'center', gap: '0.5rem',
                  cursor: 'pointer'
                }}
              >
                <RefreshCw size={18} />
                Re-initialize
              </button>
              <button
                onClick={() => window.location.href = '/'}
                style={{
                  padding: '0.8rem 1.5rem', borderRadius: '12px', background: 'var(--white-05)',
                  border: '1px solid var(--white-10)', color: 'var(--text-primary)', fontWeight: 600, display: 'flex', alignItems: 'center', gap: '0.5rem',
                  cursor: 'pointer'
                }}
              >
                <Home size={18} />
                Home
              </button>
            </div>
            
            {this.state.error && (
              <div style={{ marginTop: '2rem', padding: '1rem', background: 'var(--black-30)', borderRadius: '12px', textAlign: 'left', fontSize: '0.75rem', overflow: 'auto', maxHeight: '100px', color: 'var(--white-40)', border: '1px solid var(--white-05)' }}>
                <code>{this.state.error.toString()}</code>
              </div>
            )}
          </motion.div>
        </div>
      );
    }

    return this.props.children;
  }
}

export default ErrorBoundary;
