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
export async function signup(email: string, pass: string) {
  await RemoteOrchestrator.signup(email, pass);
}
export async function signin(email: string, pass: string) {
  await RemoteOrchestrator.signin(email, pass);
}

export async function getCompanies() {
  return RemoteOrchestrator.getCompanies();
}
export async function deleteCompany(id: string) {
  return RemoteOrchestrator.deleteCompany(id);
}
export function setActiveCompanyId(id?: string) {
  RemoteOrchestrator.activeCompanyId = id;
}
