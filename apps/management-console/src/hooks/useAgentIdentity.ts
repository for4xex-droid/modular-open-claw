import { useState, useEffect } from 'react';
import { getAuthToken } from '../lib/auth';

interface AgentIdentity {
    agentId: string | null;
    isEkycVerified: boolean;
}

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
                agentId: decoded.sub || decoded.agent_id || null,
                isEkycVerified: !!decoded.ekyc_verified,
            });
        } catch (e) {
            console.error('Failed to parse agent identity from token:', e);
            setIdentity({ agentId: null, isEkycVerified: false });
        }
    }, []);

    return identity;
};
