/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import { useState } from 'react';
import { Node, Edge } from '@xyflow/react';
import { toWorkflowDefinition, WorkflowDefinition } from '../lib/workflowConverter';
import { authenticatedFetch } from '../lib/auth';
import { API_BASE } from '../config';

export interface WorkflowListItem {
  id: string;
  name: string;
  description: string;
  current_version: number;
  updated_at: string;
}

export function useWorkflowApi() {
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const saveWorkflow = async (params: {
    id: string;
    name: string;
    description: string;
    version: number;
    nodes: Node[];
    edges: Edge[];
  }): Promise<boolean> => {
    setLoading(true);
    setError(null);
    try {
      const def = toWorkflowDefinition(params);
      const res = await authenticatedFetch(`${API_BASE}/api/v1/workflows`, {
        method: 'POST',
        body: JSON.stringify(def),
      });
      if (!res.ok) {
        throw new Error(`Failed to save workflow: ${res.statusText}`);
      }
      return true;
    } catch (e: unknown) {
      const message = e instanceof Error ? e.message : 'Error saving workflow';
      setError(message);
      return false;
    } finally {
      setLoading(false);
    }
  };

  const updateWorkflow = async (params: {
    id: string;
    name: string;
    description: string;
    version: number;
    nodes: Node[];
    edges: Edge[];
  }): Promise<boolean> => {
    setLoading(true);
    setError(null);
    try {
      const def = toWorkflowDefinition(params);
      const res = await authenticatedFetch(`${API_BASE}/api/v1/workflows/${params.id}`, {
        method: 'PUT',
        body: JSON.stringify(def),
      });
      if (!res.ok) {
        throw new Error(`Failed to update workflow: ${res.statusText}`);
      }
      return true;
    } catch (e: unknown) {
      const message = e instanceof Error ? e.message : 'Error updating workflow';
      setError(message);
      return false;
    } finally {
      setLoading(false);
    }
  };

  const listWorkflows = async (): Promise<WorkflowListItem[]> => {
    setLoading(true);
    setError(null);
    try {
      const res = await authenticatedFetch(`${API_BASE}/api/v1/workflows`);
      if (!res.ok) {
        throw new Error(`Failed to list workflows: ${res.statusText}`);
      }
      const data = await res.json();
      return Array.isArray(data) ? data : [];
    } catch (e: unknown) {
      const message = e instanceof Error ? e.message : 'Error listing workflows';
      setError(message);
      return [];
    } finally {
      setLoading(false);
    }
  };

  const loadWorkflow = async (id: string): Promise<WorkflowDefinition | null> => {
    setLoading(true);
    setError(null);
    try {
      const res = await authenticatedFetch(`${API_BASE}/api/v1/workflows/${id}`);
      if (!res.ok) {
        throw new Error(`Failed to load workflow: ${res.statusText}`);
      }
      return await res.json();
    } catch (e: unknown) {
      const message = e instanceof Error ? e.message : 'Error loading workflow';
      setError(message);
      return null;
    } finally {
      setLoading(false);
    }
  };

  const validateWorkflow = async (params: {
    id: string;
    name: string;
    description: string;
    version: number;
    nodes: Node[];
    edges: Edge[];
  }): Promise<{ valid: boolean; errors?: string[] }> => {
    setLoading(true);
    setError(null);
    try {
      const def = toWorkflowDefinition(params);
      const res = await authenticatedFetch(`${API_BASE}/api/v1/workflows/${params.id}/validate`, {
        method: 'POST',
        body: JSON.stringify(def),
      });
      if (!res.ok) {
        const errText = await res.text();
        let detail = res.statusText;
        if (errText.trim()) {
          try {
            const parsed = JSON.parse(errText) as { message?: string; error?: string };
            detail = parsed.message || parsed.error || errText;
          } catch {
            detail = errText;
          }
        }
        throw new Error(`Failed to validate workflow: ${detail}`);
      }
      const text = await res.text();
      if (!text.trim()) {
        return { valid: true };
      }
      return JSON.parse(text) as { valid: boolean; errors?: string[] };
    } catch (e: unknown) {
      const message = e instanceof Error ? e.message : 'Error validating workflow';
      setError(message);
      return { valid: false, errors: [message] };
    } finally {
      setLoading(false);
    }
  };

  const executeWorkflow = async (
    id: string
  ): Promise<{ execution_id: string; job_ids: string[] } | null> => {
    setLoading(true);
    setError(null);
    try {
      const res = await authenticatedFetch(`${API_BASE}/api/v1/workflows/${id}/execute`, {
        method: 'POST',
      });
      if (!res.ok) {
        throw new Error(`Failed to execute workflow: ${res.statusText}`);
      }
      const data = await res.json();
      return {
        execution_id: data.execution_id,
        job_ids: Array.isArray(data.job_ids) ? data.job_ids : [],
      };
    } catch (e: unknown) {
      const message = e instanceof Error ? e.message : 'Error executing workflow';
      setError(message);
      return null;
    } finally {
      setLoading(false);
    }
  };

  return {
    loading,
    error,
    saveWorkflow,
    updateWorkflow,
    listWorkflows,
    loadWorkflow,
    validateWorkflow,
    executeWorkflow,
  };
}
