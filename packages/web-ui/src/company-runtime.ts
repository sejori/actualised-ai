import initWasm, { OrchestratorWasm } from './wasm/actualised_core_wasm.js';
import { RemoteOrchestrator } from './remote-orchestrator.js';

export type SetupOrchestrator = OrchestratorWasm | RemoteOrchestrator;
const USE_REMOTE = import.meta.env.VITE_ORCHESTRATOR_MODE === 'remote';

export async function createOrchestrator(existing?: SetupOrchestrator) {
  if (existing) return existing;
  if (USE_REMOTE) {
    return RemoteOrchestrator.init();
  }
  await initWasm();
  return OrchestratorWasm.init();
}

export async function signup(email: string, pass: string) {
  if (USE_REMOTE) {
    await RemoteOrchestrator.signup(email, pass);
  }
}
export async function signin(email: string, pass: string) {
  if (USE_REMOTE) {
    await RemoteOrchestrator.signin(email, pass);
  }
}

export async function inspectCompany() {
  if (USE_REMOTE) {
    try {
      const orchestrator = await RemoteOrchestrator.init();
      return { name: orchestrator.get_company_name() ?? null, orchestrator };
    } catch {
      return { name: null, orchestrator: undefined };
    }
  }
  const orchestrator = await createOrchestrator() as OrchestratorWasm;
  return { name: orchestrator.get_company_name() ?? null, orchestrator };
}

export async function foundCompany(name: string, orchestrator?: SetupOrchestrator) {
  if (USE_REMOTE && !orchestrator) {
    orchestrator = await RemoteOrchestrator.foundCompany(name);
    return orchestrator;
  }
  if (!orchestrator) throw new Error('Orchestrator not ready');
  await orchestrator.found_company(name);
  return orchestrator;
}

export async function getCompanies() {
  if (USE_REMOTE) return RemoteOrchestrator.getCompanies();
  return []; // local mock
}
export async function deleteCompany(id: string) {
  if (USE_REMOTE) return RemoteOrchestrator.deleteCompany(id);
}
export function setActiveCompanyId(id?: string) {
  if (USE_REMOTE) RemoteOrchestrator.activeCompanyId = id;
}
