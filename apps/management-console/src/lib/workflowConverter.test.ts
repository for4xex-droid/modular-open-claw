/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import { toWorkflowDefinition, fromWorkflowDefinition, estimateCost } from './workflowConverter';
import { Node, Edge } from '@xyflow/react';

describe('Workflow Data Converter', () => {
  const mockNodes: Node[] = [
    {
      id: 'node-1',
      type: 'default',
      position: { x: 100, y: 200 },
      data: {
        label: 'Start Node',
        node_type: {
          Start: {
            trigger: 'Manual',
          },
        },
      },
    },
    {
      id: 'node-2',
      type: 'default',
      position: { x: 300, y: 400 },
      data: {
        label: 'LLM Prompt',
        node_type: {
          LlmPrompt: {
            model: 'gemini-1.5-pro',
            temperature: 0.7,
          },
        },
      },
    },
  ];

  const mockEdges: Edge[] = [
    {
      id: 'edge-1-2',
      source: 'node-1',
      target: 'node-2',
      sourceHandle: 'handle-out',
      targetHandle: 'handle-in',
    },
  ];

  it('transforms React Flow data to Rust WorkflowDefinition JSON structure', () => {
    const result = toWorkflowDefinition({
      id: '8437dfb3-c4e2-4da6-bb4a-262de6e1099c',
      name: 'Test Workflow',
      description: 'A test workflow definition',
      version: 1,
      nodes: mockNodes,
      edges: mockEdges,
    });

    expect(result.id).toBe('8437dfb3-c4e2-4da6-bb4a-262de6e1099c');
    expect(result.name).toBe('Test Workflow');
    expect(result.nodes).toHaveLength(2);

    // Node 1 checks
    expect(result.nodes[0].id).toBe('node-1');
    expect(result.nodes[0].label).toBe('Start Node');
    expect(result.nodes[0].node_type).toEqual({ Start: { trigger: 'Manual' } });
    expect(result.nodes[0].position).toEqual({ x: 100, y: 200 });

    // Edge checks (camelCase to snake_case conversions)
    expect(result.edges).toHaveLength(1);
    expect(result.edges[0].source).toBe('node-1');
    expect(result.edges[0].target).toBe('node-2');
    expect(result.edges[0].source_handle).toBe('handle-out');
    expect(result.edges[0].target_handle).toBe('handle-in');
  });

  it('transforms Rust WorkflowDefinition JSON back to React Flow nodes and edges', () => {
    const rustDef = {
      id: '8437dfb3-c4e2-4da6-bb4a-262de6e1099c',
      name: 'Test Workflow',
      description: 'A test workflow definition',
      version: 1,
      nodes: [
        {
          id: 'node-1',
          node_type: { Start: { trigger: 'Manual' } },
          label: 'Start Node',
          config: {},
          position: { x: 100, y: 200 },
        },
        {
          id: 'node-2',
          node_type: { LlmPrompt: { model: 'gemini-1.5-pro', temperature: 0.7 } },
          label: 'LLM Prompt',
          config: {},
          position: { x: 300, y: 400 },
        },
      ],
      edges: [
        {
          source: 'node-1',
          target: 'node-2',
          source_handle: 'handle-out',
          target_handle: 'handle-in',
        },
      ],
      variables: {},
      created_at: '',
      updated_at: '',
    };

    const { nodes, edges } = fromWorkflowDefinition(rustDef);

    expect(nodes).toHaveLength(2);
    expect(nodes[0].id).toBe('node-1');
    expect(nodes[0].data.label).toBe('Start Node');
    expect(nodes[0].data.node_type).toEqual({ Start: { trigger: 'Manual' } });
    expect(nodes[0].position).toEqual({ x: 100, y: 200 });

    expect(edges).toHaveLength(1);
    expect(edges[0].source).toBe('node-1');
    expect(edges[0].target).toBe('node-2');
    expect(edges[0].sourceHandle).toBe('handle-out');
    expect(edges[0].targetHandle).toBe('handle-in');
  });

  it('estimates execution cost accurately based on nodes', () => {
    const nodes: Node[] = [
      { id: '1', position: { x: 0, y: 0 }, data: { node_type: { Start: { trigger: 'Manual' } } } },
      { id: '2', position: { x: 0, y: 0 }, data: { node_type: { LlmPrompt: {} } } }, // 0.003
      { id: '3', position: { x: 0, y: 0 }, data: { node_type: { McpToolCall: { server_name: 's', tool_name: 't' } } } }, // 0.001
      { id: '4', position: { x: 0, y: 0 }, data: { node_type: { HttpRequest: { method: 'GET', url_template: 'u' } } } }, // 0.0005
      { id: '5', position: { x: 0, y: 0 }, data: { node_type: { Loop: { iterator_expression: 'i', max_iterations: 5 } } } }, // 0.003 * 5 = 0.015
      { id: '6', position: { x: 0, y: 0 }, data: { node_type: { Timer: { delay_seconds: 60 } } } }, // 0.0001
      { id: '7', position: { x: 0, y: 0 }, data: { node_type: { WasmCode: { code: 'c', language: 'javascript' } } } }, // 0.002
    ];

    const estimate = estimateCost(nodes);
    // 0.003 + 0.001 + 0.0005 + 0.015 + 0.0001 + 0.002 = 0.0216
    expect(estimate.estimatedUsd).toBeCloseTo(0.0216, 5);
    expect(estimate.nodes).toBe(7);
  });
});
