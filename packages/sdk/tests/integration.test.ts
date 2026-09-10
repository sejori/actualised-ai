import { describe, expect, it } from 'vitest';
import { ActualisedClient } from '../src/index';

describe('Actualised SDK Integration', () => {
  it('should found a company and get seeded agents', async () => {
    const client = await ActualisedClient.create('mem://');
    expect(await client.getCompanyName()).toBeNull();
    await client.foundCompany('Test Venture');
    const agents = await client.getAgents();
    expect(await client.getCompanyName()).toBe('Test Venture');
    expect(agents).toHaveLength(8);
  });

  it('should register and execute a custom tool', async () => {
    const client = await ActualisedClient.create('mem://');

    const customTool = {
      name: 'calculate_tax',
      description: 'Calculates tax',
      parameters: {
        type: 'object',
        properties: {
          amount: { type: 'number' }
        }
      }
    };
    await client.addTool(customTool);

    const agent = {
      id: 'agent_1',
      name: 'Test Agent',
      role: 'Tester',
      system_prompt: 'You are a test agent',
      tools: ['calculate_tax'],
      telemetry: null,
      scheduled_tasks: null,
      pending_messages: null,
    };
    await client.addAgent(agent);

    let executedTool = false;

    client.registerToolExecutor((_agentId, toolName, _argsJson) => {
      if (toolName === 'calculate_tax') {
        executedTool = true;
        return 'Tax calculated successfully';
      }
      return 'Unknown tool';
    });

    await client.start();
    expect(executedTool).toBe(true);
  });
});
