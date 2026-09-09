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
  private company: typeof Company;

  constructor(dbPath: string) {
    // In a real implementation, detect if we are in browser or server.
    // Server mode uses napi, Browser mode hits the server's API.
    this.company = new Company();
  }

  async setPacing(config: RateLimitConfig) {
    await this.company.set_pacing(
      config.requestsPerMinute,
      config.workingHours?.start,
      config.workingHours?.end
    );
  }

  async getAgents(): Promise<Agent[]> {
    const agentsJson = await this.company.get_agents();
    return JSON.parse(agentsJson);
  }

  async addAgent(agent: Agent): Promise<void> {
    await this.company.add_agent(JSON.stringify(agent));
  }

  async queueMessage(agentId: string, message: string): Promise<void> {
    await this.company.queue_message(agentId, message);
  }

  // Other methods ...
}
