/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import { A2uiEnvelope, A2uiSurface } from '../types';

type Listener = () => void;

/** Shared surface registry for chat-embedded A2UI (create / update / delete). */
class A2uiSurfaceStore {
    private surfaces = new Map<string, A2uiSurface>();
    private deletedIds = new Set<string>();
    private listeners = new Set<Listener>();

    subscribe(listener: Listener): () => void {
        this.listeners.add(listener);
        return () => this.listeners.delete(listener);
    }

    getSnapshot(): Map<string, A2uiSurface> {
        return this.surfaces;
    }

    applyEnvelope(envelope: A2uiEnvelope): void {
        switch (envelope.type) {
            case 'createSurface':
                this.deletedIds.delete(envelope.surface.id);
                this.surfaces.set(envelope.surface.id, envelope.surface);
                break;
            case 'updateComponents': {
                const existing = this.surfaces.get(envelope.surfaceId);
                if (existing) {
                    this.surfaces.set(envelope.surfaceId, {
                        ...existing,
                        components: envelope.components,
                    });
                }
                break;
            }
            case 'deleteSurface':
                this.surfaces.delete(envelope.surfaceId);
                this.deletedIds.add(envelope.surfaceId);
                break;
        }
        this.listeners.forEach((l) => l());
    }

    getSurface(id: string): A2uiSurface | undefined {
        if (this.deletedIds.has(id)) {
            return undefined;
        }
        return this.surfaces.get(id);
    }

    isDeleted(id: string): boolean {
        return this.deletedIds.has(id);
    }

    /** Test helper */
    clear(): void {
        this.surfaces.clear();
        this.deletedIds.clear();
        this.listeners.forEach((l) => l());
    }
}

export const a2uiSurfaceStore = new A2uiSurfaceStore();
