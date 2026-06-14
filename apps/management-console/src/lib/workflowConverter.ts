import { Node, Edge } from '@xyflow/react';

export type NodeType =
  | { Start: { trigger: 'Manual' | { Cron: { expression: string } } | 'Webhook' | { Event: { event_name: string } } } }
  | { LlmPrompt: { model?: string; temperature?: number } }
  | { McpToolCall: { server_name: string; tool_name: string } }
  | { HttpRequest: { method: string; url_template: string } }
  | { Transform: { expression: string } }
  | { Condition: { expression: string; mode: 'Expression' | 'LlmJudge' } }
  | { HumanApproval: { prompt_message: string; timeout_seconds?: number } }
  | { Loop: { iterator_expression: string; max_iterations?: number } }
  | { Parallel: { wait_mode: 'All' | 'Any' | { N: number } } }
  | { SubWorkflow: { workflow_id: string; version?: number } }
  | { Timer: { delay_seconds: number } }
  | { WasmCode: { code: string; language: string } };

export interface WorkflowNode {
  id: string;
  node_type: NodeType;
  label: string;
  config: any;
  position: { x: number; y: number };
}

export interface WorkflowEdge {
  source: string;
  target: string;
  source_handle?: string;
  target_handle?: string;
}

export interface WorkflowDefinition {
  id: string;
  name: string;
  description: string;
  version: number;
  nodes: WorkflowNode[];
  edges: WorkflowEdge[];
  variables: Record<string, any>;
  created_at: string;
  updated_at: string;
}

export function toWorkflowDefinition(params: {
  id: string;
  name: string;
  description: string;
  version: number;
  nodes: Node[];
  edges: Edge[];
}): WorkflowDefinition {
  const nodes: WorkflowNode[] = params.nodes.map((n) => {
    const nodeType = (n.data?.node_type as NodeType) || { Start: { trigger: 'Manual' } };
    return {
      id: n.id,
      node_type: nodeType,
      label: (n.data?.label as string) || n.id,
      config: n.data?.config || {},
      position: {
        x: n.position.x,
        y: n.position.y,
      },
    };
  });

  const edges: WorkflowEdge[] = params.edges.map((e) => {
    return {
      source: e.source,
      target: e.target,
      source_handle: e.sourceHandle || undefined,
      target_handle: e.targetHandle || undefined,
    };
  });

  return {
    id: params.id,
    name: params.name,
    description: params.description,
    version: params.version,
    nodes,
    edges,
    variables: {},
    created_at: new Date().toISOString(),
    updated_at: new Date().toISOString(),
  };
}

export function fromWorkflowDefinition(def: WorkflowDefinition): {
  nodes: Node[];
  edges: Edge[];
} {
  const nodes: Node[] = def.nodes.map((n) => {
    return {
      id: n.id,
      type: 'default',
      position: {
        x: n.position.x,
        y: n.position.y,
      },
      data: {
        label: n.label,
        node_type: n.node_type,
        config: n.config,
      },
    };
  });

  const edges: Edge[] = def.edges.map((e, idx) => {
    return {
      id: `edge-${e.source}-${e.target}-${idx}`,
      source: e.source,
      target: e.target,
      sourceHandle: e.source_handle || undefined,
      targetHandle: e.target_handle || undefined,
    };
  });

  return { nodes, edges };
}

export interface CostEstimate {
  estimatedUsd: number;
  nodes: number;
}

export function estimateCost(nodes: Node[]): CostEstimate {
  let total = 0.0;
  for (const node of nodes) {
    const nodeType = node.data?.node_type as NodeType;
    if (!nodeType) continue;

    const typeName = Object.keys(nodeType)[0];
    const details = (nodeType as any)[typeName] || {};

    switch (typeName) {
      case 'LlmPrompt':
        total += 0.003;
        break;
      case 'McpToolCall':
        total += 0.001;
        break;
      case 'HttpRequest':
        total += 0.0005;
        break;
      case 'Loop':
        const maxIterations = typeof details.max_iterations === 'number' ? details.max_iterations : 10;
        total += 0.003 * maxIterations;
        break;
      case 'Timer':
        total += 0.0001;
        break;
      case 'WasmCode':
        total += 0.002;
        break;
      default:
        break;
    }
  }

  return {
    estimatedUsd: total,
    nodes: nodes.length,
  };
}
