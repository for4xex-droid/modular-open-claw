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
    } catch (e: any) {
      setError(e.message || 'Error saving workflow');
      return false;
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
    } catch (e: any) {
      setError(e.message || 'Error loading workflow');
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
        throw new Error(`Failed to validate workflow: ${res.statusText}`);
      }
      return await res.json();
    } catch (e: any) {
      setError(e.message || 'Error validating workflow');
      return { valid: false, errors: [e.message] };
    } finally {
      setLoading(false);
    }
  };

  const executeWorkflow = async (id: string): Promise<{ execution_id: string } | null> => {
    setLoading(true);
    setError(null);
    try {
      const res = await authenticatedFetch(`${API_BASE}/api/v1/workflows/${id}/execute`, {
        method: 'POST',
      });
      if (!res.ok) {
        throw new Error(`Failed to execute workflow: ${res.statusText}`);
      }
      return await res.json();
    } catch (e: any) {
      setError(e.message || 'Error executing workflow');
      return null;
    } finally {
      setLoading(false);
    }
  };

  return {
    loading,
    error,
    saveWorkflow,
    loadWorkflow,
    validateWorkflow,
    executeWorkflow,
  };
}
