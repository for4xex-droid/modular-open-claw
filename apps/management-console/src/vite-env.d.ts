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
