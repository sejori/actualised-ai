import { createRequire } from 'node:module';
import type { Company as NativeCompany } from '../native.js';

const { Company } = createRequire(__filename)('../index.js') as { Company: typeof NativeCompany };

export { Company };

export interface RateLimitConfig {
  requestsPerMinute: number;
  workingHours?: {
    start: string;
    end: string;
  };
}

export interface Agent {
  id: string;
  name: string;
  role: string;
  parent_id?: string;
  system_prompt: string;
  tools: string[];
}

export interface Project {
  id: string;
  title: string;
  description: string;
}

/**
 * A wrapper to interact with the underlying Rust napi bindings.
 * This class serves as the shared API surface for both in-browser and server execution.
 * When in browser, it would hit the HTTPS API. On server, it invokes the node addon.
 */
export class ActualisedClient {
  private company: NativeCompany;

  private constructor(company: NativeCompany) {
    this.company = company;
  }

  static async create(dbPath: string): Promise<ActualisedClient> {
    const company = await Company.init('Test Co', 'Testing', 'state', dbPath);
    return new ActualisedClient(company);
  }

  getCompanyName(): Promise<string | null> {
    return this.company.getCompanyName();
  }

  foundCompany(name: string): Promise<void> {
    return this.company.foundCompany(name);
  }

  async setPacing(config: RateLimitConfig) {
    await this.company.setPacing(
      config.requestsPerMinute,
      config.workingHours?.start,
      config.workingHours?.end
    );
  }

  async getAgents(): Promise<Agent[]> {
    const agentsJson = await this.company.getAgents();
    return JSON.parse(agentsJson);
  }

  async getProjects(): Promise<Project[]> {
    return JSON.parse(await this.company.getProjects());
  }

  async getTools(): Promise<unknown[]> {
    return JSON.parse(await this.company.getTools());
  }

  async addTool(tool: unknown): Promise<void> {
    await this.company.addTool(JSON.stringify(tool));
  }

  async getAgentContext(agentId: string): Promise<unknown> {
    return JSON.parse(await this.company.getAgentContext(agentId));
  }

  async getAgentMemoryTree(agentId: string): Promise<unknown[]> {
    return JSON.parse(await this.company.getAgentMemoryTree(agentId));
  }

  async getSharedTree(): Promise<unknown[]> {
    return JSON.parse(await this.company.getSharedTree());
  }

  async addAgent(agent: Agent): Promise<void> {
    await this.company.addAgent(JSON.stringify(agent));
  }

  async updateAgent(id: string, agent: Agent): Promise<void> {
    await this.company.updateAgent(id, JSON.stringify(agent));
  }

  async removeAgent(id: string): Promise<void> {
    await this.company.removeAgent(id);
  }

  async addSharedFile(file: unknown): Promise<void> {
    await this.company.addSharedFile(JSON.stringify(file));
  }

  async removeTool(name: string): Promise<void> {
    await this.company.removeTool(name);
  }

  async configureInference(config: unknown): Promise<void> {
    await this.company.configureInference(JSON.stringify(config));
  }

  async queueMessage(agentId: string, message: string): Promise<void> {
    await this.company.queueMessage(agentId, message);
  }

  registerToolExecutor(executor: (agentId: string, toolName: string, argsJson: string) => string): void {
    this.company.registerToolExecutor(executor);
  }

  async start(): Promise<void> {
    await this.company.start();
  }
}
