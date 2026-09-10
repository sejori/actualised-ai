import initWasm, { OrchestratorWasm } from './wasm/actualised_core_wasm.js';

export type SetupOrchestrator = OrchestratorWasm;

export async function createOrchestrator(existing?: SetupOrchestrator) {
  if (existing) return existing;
  await initWasm();
  return OrchestratorWasm.init();
}

export async function inspectCompany() {
  const orchestrator = await createOrchestrator();
  return { name: orchestrator.get_company_name() ?? null, orchestrator };
}

export async function foundCompany(name: string, orchestrator?: SetupOrchestrator) {
  if (!orchestrator) throw new Error('Orchestrator not ready');
  await orchestrator.found_company(name);
  return orchestrator;
}