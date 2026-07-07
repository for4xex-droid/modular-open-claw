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
      text: async () => JSON.stringify(mockResponse),
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

  it('treats empty 200 validate response as valid (API contract)', async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true,
      status: 200,
      text: async () => '',
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

    expect(validation).toEqual({ valid: true });
  });

  it('surfaces API error body on validate failure', async () => {
    mockFetch.mockResolvedValueOnce({
      ok: false,
      status: 400,
      statusText: 'Bad Request',
      text: async () => JSON.stringify({ message: 'Workflow is invalid: missing start node' }),
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

    expect(validation.valid).toBe(false);
    expect(validation.errors?.[0]).toContain('missing start node');
  });

  it('handles execution and returns execution_id with job_ids (FIND-1)', async () => {
    const mockResponse = { execution_id: 'exec-123', job_ids: ['job-a', 'job-b'] };

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

  it('defaults job_ids to empty array when API omits the field', async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true,
      status: 200,
      json: async () => ({ execution_id: 'exec-legacy' }),
    });

    const { result } = renderHook(() => useWorkflowApi());

    let execResult;
    await act(async () => {
      execResult = await result.current.executeWorkflow('w-1');
    });

    expect(execResult).toEqual({ execution_id: 'exec-legacy', job_ids: [] });
  });

  it('lists workflow executions successfully', async () => {
    const mockExecutions = [
      {
        id: 'exec-1',
        workflow_id: 'w-1',
        version: 1,
        status: 'Completed',
        input_variables: '{}',
        output_result: null,
        root_job_id: null,
        started_at: '2026-07-08T00:00:00Z',
        completed_at: '2026-07-08T00:01:00Z',
      },
    ];

    mockFetch.mockResolvedValueOnce({
      ok: true,
      status: 200,
      json: async () => mockExecutions,
    });

    const { result } = renderHook(() => useWorkflowApi());

    let executions;
    await act(async () => {
      executions = await result.current.listExecutions('w-1');
    });

    expect(executions).toEqual(mockExecutions);
    expect(mockFetch).toHaveBeenCalledWith(
      'http://localhost:3000/api/v1/workflows/w-1/executions',
      expect.any(Object)
    );
  });

  it('returns empty array when listExecutions fails', async () => {
    mockFetch.mockResolvedValueOnce({
      ok: false,
      status: 500,
      statusText: 'Internal Server Error',
    });

    const { result } = renderHook(() => useWorkflowApi());

    let executions;
    await act(async () => {
      executions = await result.current.listExecutions('w-1');
    });

    expect(executions).toEqual([]);
  });
});
