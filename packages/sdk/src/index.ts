import { Company } from './actualised_sdk.node';

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

  // Other methods ...
}
