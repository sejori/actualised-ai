import { RemoteOrchestrator } from './remote-orchestrator';

export type SetupOrchestrator = RemoteOrchestrator;

export function createOrchestrator(existing?: SetupOrchestrator) {
  return existing ?? RemoteOrchestrator.init();
}

export async function inspectCompany() {
  return { name: await RemoteOrchestrator.getCompanyName(), orchestrator: undefined };
}

export async function foundCompany(name: string) {
  return RemoteOrchestrator.foundCompany(name);
}