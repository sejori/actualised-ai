import { Company } from '../index.js';

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

/**
 * A wrapper to interact with the underlying Rust napi bindings.
 * This class serves as the shared API surface for both in-browser and server execution.
 * When in browser, it would hit the HTTPS API. On server, it invokes the node addon.
 */
export class ActualisedClient {
  private company: Company;

  private constructor(company: Company) {
    this.company = company;
  }

  static async create(dbPath: string): Promise<ActualisedClient> {
    const company = await Company.init('Test Co', 'Testing', 'state', dbPath);
    return new ActualisedClient(company);
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

  async addAgent(agent: Agent): Promise<void> {
    await this.company.addAgent(JSON.stringify(agent));
  }

  async queueMessage(agentId: string, message: string): Promise<void> {
    await this.company.queueMessage(agentId, message);
  }

  registerToolExecutor(executor: (agentId: string, toolName: string, argsJson: string) => Promise<string>): void {
    this.company.registerToolExecutor(executor);
  }

  async start(): Promise<void> {
    await this.company.start();
  }
}
