import { renderHook } from '@testing-library/react';
import { useAgentIdentity } from './useAgentIdentity';
import * as auth from '../lib/auth';

jest.mock('../lib/auth');

describe('useAgentIdentity', () => {
    beforeEach(() => {
        jest.resetAllMocks();
    });

    // JWT Helper
    const createMockToken = (payload: any) => {
        const header = btoa(JSON.stringify({ alg: 'HS256', typ: 'JWT' }));
        // Base64URL format (no padding =, replace + with -, replace / with _)
        const body = btoa(JSON.stringify(payload)).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '');
        const signature = 'mock-signature';
        return `${header}.${body}.${signature}`;
    };

    it('should return nulls when no token exists', () => {
        (auth.getAuthToken as jest.Mock).mockReturnValue(null);
        const { result } = renderHook(() => useAgentIdentity());
        expect(result.current.agentId).toBeNull();
        expect(result.current.isEkycVerified).toBe(false);
    });

    it('should decode base64url payload and return agentId from sub', () => {
        (auth.getAuthToken as jest.Mock).mockReturnValue(
            createMockToken({ sub: '123e4567-e89b-12d3-a456-426614174000', ekyc_verified: true })
        );
        const { result } = renderHook(() => useAgentIdentity());
        expect(result.current.agentId).toBe('123e4567-e89b-12d3-a456-426614174000');
        expect(result.current.isEkycVerified).toBe(true);
    });

    it('should return agentId from agent_id if sub is missing', () => {
        (auth.getAuthToken as jest.Mock).mockReturnValue(
            createMockToken({ agent_id: '123e4567-e89b-12d3-a456-426614174001', ekyc_verified: false })
        );
        const { result } = renderHook(() => useAgentIdentity());
        expect(result.current.agentId).toBe('123e4567-e89b-12d3-a456-426614174001');
        expect(result.current.isEkycVerified).toBe(false);
    });

    it('should handle malformed token gracefully', () => {
        (auth.getAuthToken as jest.Mock).mockReturnValue('invalid.token.here');
        const { result } = renderHook(() => useAgentIdentity());
        expect(result.current.agentId).toBeNull();
        expect(result.current.isEkycVerified).toBe(false);
    });
});
