/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import { useState, useEffect } from 'react';
import { getAuthToken } from '../lib/auth';

interface AgentIdentity {
    agentId: string | null;
    isEkycVerified: boolean;
}

const UUID_RE =
    /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;

/** Prefer JWT `agent_id`; fall back to UUID-shaped `sub` only (never email). */
export const resolveAgentIdFromClaims = (decoded: {
    agent_id?: unknown;
    sub?: unknown;
}): string | null => {
    if (typeof decoded.agent_id === 'string' && UUID_RE.test(decoded.agent_id)) {
        return decoded.agent_id;
    }
    if (typeof decoded.sub === 'string' && UUID_RE.test(decoded.sub)) {
        return decoded.sub;
    }
    return null;
};

export const useAgentIdentity = (): AgentIdentity => {
    const [identity, setIdentity] = useState<AgentIdentity>({
        agentId: null,
        isEkycVerified: false,
    });

    useEffect(() => {
        const token = getAuthToken();
        if (!token) {
            setIdentity({ agentId: null, isEkycVerified: false });
            return;
        }

        try {
            const parts = token.split('.');
            if (parts.length !== 3) {
                throw new Error('Invalid JWT format');
            }

            const payloadStr = parts[1];
            // Base64Url decode logic
            const base64 = payloadStr.replace(/-/g, '+').replace(/_/g, '/');
            const pad = base64.length % 4;
            const paddedBase64 = pad ? base64 + '='.repeat(4 - pad) : base64;

            const decoded = JSON.parse(atob(paddedBase64));

            setIdentity({
                agentId: resolveAgentIdFromClaims(decoded),
                isEkycVerified: !!decoded.ekyc_verified,
            });
        } catch (e) {
            console.error('Failed to parse agent identity from token:', e);
            setIdentity({ agentId: null, isEkycVerified: false });
        }
    }, []);

    return identity;
};
