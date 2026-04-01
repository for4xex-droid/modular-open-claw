/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
/// <reference types="vite/client" />

declare module '@nicebyte/inochi2d-es' {
    export class Inochi2D {
        static init(): Promise<void>;
        loadModel(buffer: ArrayBuffer): void;
        update(): void;
        draw(gl: WebGLRenderingContext | WebGL2RenderingContext): void;
        destroy(): void;
        setParameter(name: string, value: number): void;
    }
}
