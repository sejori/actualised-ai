import re
with open("packages/web-ui/src/company-runtime.cloud.ts", "r", encoding="utf-8") as f:
    c = f.read()

exports_patch = """
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
"""

if "export async function signup(" not in c:
    c += exports_patch

with open("packages/web-ui/src/company-runtime.cloud.ts", "w", encoding="utf-8") as f:
    f.write(c)
