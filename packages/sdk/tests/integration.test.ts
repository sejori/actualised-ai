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
    await client.queueMessage('agent_1', 'Please calculate the tax.');

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

  it('should register and execute an asynchronous custom tool', async () => {
    const client = await ActualisedClient.create('mem://');

    const customTool = {
      name: 'async_op',
      description: 'Async operation',
      parameters: {
        type: 'object',
        properties: {
          task: { type: 'string' }
        }
      }
    };
    await client.addTool(customTool);

    const agent = {
      id: 'agent_async',
      name: 'Async Tester',
      role: 'Tester',
      system_prompt: 'You are an async tester',
      tools: ['async_op'],
      telemetry: null,
      scheduled_tasks: null,
      pending_messages: null,
    };
    await client.addAgent(agent);
    await client.queueMessage('agent_async', 'Execute async operation.');

    let asyncDone = false;
    client.registerToolExecutor(async (_agentId, toolName, _argsJson) => {
      if (toolName === 'async_op') {
        await new Promise((r) => setTimeout(r, 20));
        asyncDone = true;
        return 'Async finished';
      }
      return 'Unknown';
    });

    await client.start();
    expect(asyncDone).toBe(true);
  });
});
