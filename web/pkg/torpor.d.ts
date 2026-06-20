/* tslint:disable */
/* eslint-disable */

export class Solver {
    free(): void;
    [Symbol.dispose](): void;
    constructor(modulus_hex: string, base_hex: string, difficulty: bigint);
    /**
     * Square up to `steps` more times. Returns `true` once the chain is done
     */
    step(steps: bigint): boolean;
    readonly answerHex: string | undefined;
    readonly done: boolean;
    readonly progress: number;
}

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly __wbg_solver_free: (a: number, b: number) => void;
    readonly solver_answerHex: (a: number, b: number) => void;
    readonly solver_done: (a: number) => number;
    readonly solver_new: (a: number, b: number, c: number, d: number, e: number, f: bigint) => void;
    readonly solver_progress: (a: number) => number;
    readonly solver_step: (a: number, b: bigint) => number;
    readonly __wbindgen_add_to_stack_pointer: (a: number) => number;
    readonly __wbindgen_export: (a: number, b: number, c: number) => void;
    readonly __wbindgen_export2: (a: number, b: number) => number;
    readonly __wbindgen_export3: (a: number, b: number, c: number, d: number) => number;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;

/**
 * Instantiates the given `module`, which can either be bytes or
 * a precompiled `WebAssembly.Module`.
 *
 * @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
 *
 * @returns {InitOutput}
 */
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
 * If `module_or_path` is {RequestInfo} or {URL}, makes a request and
 * for everything else, calls `WebAssembly.instantiate` directly.
 *
 * @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
 *
 * @returns {Promise<InitOutput>}
 */
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
