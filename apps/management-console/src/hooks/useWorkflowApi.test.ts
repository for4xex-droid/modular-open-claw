/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import { renderHook, act } from '@testing-library/react';
import { useWorkflowApi } from './useWorkflowApi';

jest.mock('../config', () => ({
  API_BASE: 'http://localhost:3000'
}));

// グローバル fetch のモック
const mockFetch = jest.fn();
global.fetch = mockFetch;

describe('useWorkflowApi Hook', () => {
  beforeEach(() => {
    mockFetch.mockClear();
  });

  it('saves workflow successfully', async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true,
      status: 200,
    });

    const { result } = renderHook(() => useWorkflowApi());

    let success;
    await act(async () => {
      success = await result.current.saveWorkflow({
        id: 'w-1',
        name: 'Test',
        description: 'Desc',
        version: 1,
        nodes: [],
        edges: [],
      });
    });

    expect(success).toBe(true);
    expect(mockFetch).toHaveBeenCalledWith('http://localhost:3000/api/v1/workflows', expect.any(Object));
  });

  it('loads workflow successfully', async () => {
    const mockDef = {
      id: 'w-1',
      name: 'Loaded Test',
      description: 'Desc',
      version: 1,
      nodes: [],
      edges: [],
      variables: {},
      created_at: '',
      updated_at: '',
    };

    mockFetch.mockResolvedValueOnce({
      ok: true,
      status: 200,
      json: async () => mockDef,
    });

    const { result } = renderHook(() => useWorkflowApi());

    let data;
    await act(async () => {
      data = await result.current.loadWorkflow('w-1');
    });

    expect(data).toEqual(mockDef);
    expect(mockFetch).toHaveBeenCalledWith('http://localhost:3000/api/v1/workflows/w-1', expect.any(Object));
  });

  it('validates workflow successfully', async () => {
    const mockResponse = { valid: true };

    mockFetch.mockResolvedValueOnce({
      ok: true,
      status: 200,
      json: async () => mockResponse,
    });

    const { result } = renderHook(() => useWorkflowApi());

    let validation;
    await act(async () => {
      validation = await result.current.validateWorkflow({
        id: 'w-1',
        name: 'Test',
        description: 'Desc',
        version: 1,
        nodes: [],
        edges: [],
      });
    });

    expect(validation).toEqual(mockResponse);
  });

  it('handles execution and returns execution_id (FIND-1)', async () => {
    const mockResponse = { execution_id: 'exec-123' };

    mockFetch.mockResolvedValueOnce({
      ok: true,
      status: 200,
      json: async () => mockResponse,
    });

    const { result } = renderHook(() => useWorkflowApi());

    let execResult;
    await act(async () => {
      execResult = await result.current.executeWorkflow('w-1');
    });

    expect(execResult).toEqual(mockResponse);
    expect(mockFetch).toHaveBeenCalledWith('http://localhost:3000/api/v1/workflows/w-1/execute', expect.any(Object));
  });
});
