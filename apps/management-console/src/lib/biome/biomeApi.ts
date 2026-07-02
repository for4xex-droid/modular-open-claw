/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

import { authenticatedFetch } from '../../lib/auth';
import { API_BASE } from '../../config';
import type { Specimen } from './BiomeDendou';

export async function fetchBiomeSpecimens(): Promise<Specimen[]> {
  const res = await authenticatedFetch(`${API_BASE}/api/v1/biome/specimens`);
  if (!res.ok) {
    throw new Error(`Failed to fetch specimens: ${res.status}`);
  }
  const data = await res.json();
  return data.map((item: Record<string, string>) => ({
    id: item.id,
    name: item.specimen_name,
    generation: 200,
    rarity: item.rarity,
    date: new Date(item.created_at).toLocaleDateString(),
  }));
}

export async function saveBiomeRun(payload: Record<string, unknown>): Promise<void> {
  const res = await authenticatedFetch(`${API_BASE}/api/v1/biome/runs`, {
    method: 'POST',
    body: JSON.stringify(payload),
  });
  if (!res.ok) {
    throw new Error(`Failed to save run: ${res.status}`);
  }
}

export async function saveBiomeSpecimen(payload: Record<string, unknown>): Promise<void> {
  const res = await authenticatedFetch(`${API_BASE}/api/v1/biome/specimens`, {
    method: 'POST',
    body: JSON.stringify(payload),
  });
  if (!res.ok) {
    throw new Error(`Failed to save specimen: ${res.status}`);
  }
}
