import test from 'node:test';
import assert from 'node:assert';
import { ActualisedClient } from '../src/index';

test('Actualised SDK Integration', async (t) => {
  await t.test('should initialize and get agents', async () => {
    const client = await ActualisedClient.create('mem://');
    const agents = await client.getAgents();
    assert.strictEqual(Array.isArray(agents), true);
  });

  await t.test('should register and execute a custom tool', async () => {
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
    await (client as any).company.addTool(JSON.stringify(customTool));

    const agent = {
      id: 'agent_1',
      name: 'Test Agent',
      role: 'Tester',
      system_prompt: 'You are a test agent',
      tools: ['calculate_tax']
    };
    await client.addAgent(agent);

    let executedTool = false;

    client.registerToolExecutor(async (agentId, toolName, argsJson) => {
      if (toolName === 'calculate_tax') {
        executedTool = true;
        return 'Tax calculated successfully';
      }
      return 'Unknown tool';
    });

    await client.start();
    assert.strictEqual(executedTool, true);
  });
});
