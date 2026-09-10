/* tslint:disable */
/* eslint-disable */

export class OrchestratorWasm {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
    add_agent(agent_json: any): Promise<void>;
    add_project(project_json: any): Promise<void>;
    add_shared_file(file_json: any): Promise<void>;
    add_tool(tool_json: any): Promise<void>;
    configure_inference(config_json: any): void;
    configure_rate_limits(config_json: any): void;
    get_agent_context(agent_id: string): any;
    get_agent_memories(agent_id: string): any;
    get_agent_memory_tree(agent_id: string): any;
    get_agents(): any;
    get_projects(): any;
    get_shared_files(): any;
    get_shared_tree(): any;
    get_tools(): any;
    static init(): Promise<OrchestratorWasm>;
    remove_agent(id: string): Promise<void>;
    remove_shared_file(id: string): Promise<void>;
    remove_tool(name: string): Promise<void>;
    run_orchestrator(): Promise<void>;
    send_agent_message(agent_id: string, message: string): Promise<void>;
    update_agent(id: string, agent_json: any): Promise<void>;
    update_tool(name: string, tool_json: any): Promise<void>;
}

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly __wbg_orchestratorwasm_free: (a: number, b: number) => void;
    readonly orchestratorwasm_add_agent: (a: number, b: any) => any;
    readonly orchestratorwasm_add_project: (a: number, b: any) => any;
    readonly orchestratorwasm_add_shared_file: (a: number, b: any) => any;
    readonly orchestratorwasm_add_tool: (a: number, b: any) => any;
    readonly orchestratorwasm_configure_inference: (a: number, b: any) => [number, number];
    readonly orchestratorwasm_configure_rate_limits: (a: number, b: any) => [number, number];
    readonly orchestratorwasm_get_agent_context: (a: number, b: number, c: number) => [number, number, number];
    readonly orchestratorwasm_get_agent_memories: (a: number, b: number, c: number) => [number, number, number];
    readonly orchestratorwasm_get_agent_memory_tree: (a: number, b: number, c: number) => [number, number, number];
    readonly orchestratorwasm_get_agents: (a: number) => [number, number, number];
    readonly orchestratorwasm_get_projects: (a: number) => [number, number, number];
    readonly orchestratorwasm_get_shared_files: (a: number) => [number, number, number];
    readonly orchestratorwasm_get_shared_tree: (a: number) => [number, number, number];
    readonly orchestratorwasm_get_tools: (a: number) => [number, number, number];
    readonly orchestratorwasm_init: () => any;
    readonly orchestratorwasm_remove_agent: (a: number, b: number, c: number) => any;
    readonly orchestratorwasm_remove_shared_file: (a: number, b: number, c: number) => any;
    readonly orchestratorwasm_remove_tool: (a: number, b: number, c: number) => any;
    readonly orchestratorwasm_run_orchestrator: (a: number) => any;
    readonly orchestratorwasm_send_agent_message: (a: number, b: number, c: number, d: number, e: number) => any;
    readonly orchestratorwasm_update_agent: (a: number, b: number, c: number, d: any) => any;
    readonly orchestratorwasm_update_tool: (a: number, b: number, c: number, d: any) => any;
    readonly wasm_bindgen_d05406a4d4dc2481___convert__closures_____invoke___js_sys_5b72c82c83b45550___Function_fn_wasm_bindgen_d05406a4d4dc2481___JsValue_____wasm_bindgen_d05406a4d4dc2481___sys__Undefined___js_sys_5b72c82c83b45550___Function_fn_wasm_bindgen_d05406a4d4dc2481___JsValue_____wasm_bindgen_d05406a4d4dc2481___sys__Undefined_______true_: (a: number, b: number, c: any, d: any) => void;
    readonly wasm_bindgen_d05406a4d4dc2481___convert__closures_____invoke___wasm_bindgen_d05406a4d4dc2481___JsValue__core_ed718c3d60ebd546___result__Result_____wasm_bindgen_d05406a4d4dc2481___JsError___true_: (a: number, b: number, c: any) => [number, number];
    readonly wasm_bindgen_d05406a4d4dc2481___convert__closures_____invoke_______true_: (a: number, b: number) => void;
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
    readonly __wbindgen_exn_store: (a: number) => void;
    readonly __externref_table_alloc: () => number;
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __wbindgen_free: (a: number, b: number, c: number) => void;
    readonly __wbindgen_destroy_closure: (a: number, b: number) => void;
    readonly __externref_table_dealloc: (a: number) => void;
    readonly __wbindgen_start: () => void;
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
