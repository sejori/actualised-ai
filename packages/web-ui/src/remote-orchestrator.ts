type Bootstrap = { company_name: string | null; agents: unknown[]; projects: unknown[]; tools: unknown[] };

async function request<T>(path: string, init?: RequestInit): Promise<T> {
  const response = await fetch(path, {
    ...init,
    headers: { 'Content-Type': 'application/json', ...init?.headers },
  });
  if (!response.ok) {
    const body = await response.json().catch(() => ({})) as { error?: string };
    throw new Error(body.error ?? `Request failed with status ${response.status}`);
  }
  return response.json() as Promise<T>;
}

export class RemoteOrchestrator {
  private constructor(private state: Bootstrap) {}

  static async init() {
    return new RemoteOrchestrator(await request<Bootstrap>('/api/bootstrap'));
  }

  static async getCompanyName() {
    return (await request<{ name: string | null }>('/api/company')).name;
  }

  static async foundCompany(name: string) {
    return new RemoteOrchestrator(await request<Bootstrap>('/api/company', {
      method: 'POST',
      body: JSON.stringify({ name }),
    }));
  }

  async refresh() {
    this.update(await request<Bootstrap>('/api/bootstrap'));
  }

  private update(state: Bootstrap) {
    this.state = state;
  }

  get_agents() { return this.state.agents; }
  get_company_name() { return this.state.company_name; }
  get_projects() { return this.state.projects; }
  get_tools() { return this.state.tools; }

  async found_company(name: string) {
    this.state = (await RemoteOrchestrator.foundCompany(name)).state;
  }
  get_agent_context(agentId: string) { return request<unknown>(`/api/agents/${encodeURIComponent(agentId)}/context`); }
  get_agent_memory_tree(agentId: string) { return request<unknown[]>(`/api/agents/${encodeURIComponent(agentId)}/memory-tree`); }
  get_shared_tree() { return request<unknown[]>('/api/shared-tree'); }

  async add_agent(agent: unknown) {
    this.update(await request<Bootstrap>('/api/agents', { method: 'POST', body: JSON.stringify(agent) }));
  }

  async update_agent(id: string, agent: unknown) {
    this.update(await request<Bootstrap>(`/api/agents/${encodeURIComponent(id)}`, { method: 'PUT', body: JSON.stringify(agent) }));
  }

  async remove_agent(id: string) {
    this.update(await request<Bootstrap>(`/api/agents/${encodeURIComponent(id)}`, { method: 'DELETE' }));
  }

  send_agent_message(id: string, message: string) {
    return request(`/api/agents/${encodeURIComponent(id)}/messages`, { method: 'POST', body: JSON.stringify({ message }) });
  }

  async run_orchestrator() {
    this.update(await request<Bootstrap>('/api/orchestrator/run', { method: 'POST' }));
  }

  configure_inference(config: unknown) {
    return request('/api/config/inference', { method: 'POST', body: JSON.stringify(config) });
  }

  configure_rate_limits(config: unknown) {
    return request('/api/config/rate-limits', { method: 'POST', body: JSON.stringify(config) });
  }

  add_shared_file(file: unknown) {
    return request('/api/shared-files', { method: 'POST', body: JSON.stringify(file) });
  }

  async remove_tool(name: string) {
    this.update(await request<Bootstrap>(`/api/tools/${encodeURIComponent(name)}`, { method: 'DELETE' }));
  }
}