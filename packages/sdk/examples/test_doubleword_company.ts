import { ActualisedClient } from '../src/index';

async function runTestCompany() {
  console.log('=== Running Test Company with Doubleword DeepSeek-V4.1-Flash ===');

  const apiKey = process.env.DOUBLEWORD_API_KEY || 'sk-U-5Q1NFbZaI7nsuSSmCqzqcn_AOQLe9wirAJUOQTZH0';
  const client = await ActualisedClient.create('mem://');

  console.log('Configuring company inference to Doubleword...');
  await client.configureInference({
    provider: 'doubleword',
    model: 'deepseek-ai/DeepSeek-V4.1-Flash',
    apiKey: apiKey,
    baseUrl: 'https://api.doubleword.ai/v1',
    temperature: 0.2
  });

  console.log('Adding custom async tool: publish_architecture_spec...');
  await client.addTool({
    name: 'publish_architecture_spec',
    description: 'Publishes an architecture specification asynchronously to the shared system catalog.',
    parameters: {
      type: 'object',
      properties: {
        title: { type: 'string' },
        summary: { type: 'string' }
      },
      required: ['title', 'summary']
    }
  });

  let asyncToolExecuted = false;
  client.registerToolExecutor(async (agentId, toolName, argsJson) => {
    console.log(`[Async Tool Executor] Received call: ${toolName} from ${agentId}`);
    if (toolName === 'publish_architecture_spec') {
      await new Promise(resolve => setTimeout(resolve, 100));
      asyncToolExecuted = true;
      const args = JSON.parse(argsJson);
      console.log(`[Async Tool Executor] Published spec: "${args.title}"`);
      return JSON.stringify({ status: 'published', timestamp: new Date().toISOString() });
    }
    return JSON.stringify({ status: 'unknown_tool' });
  });

  console.log('Adding Lead Architect with temperature 0.1 (low temp for precision)...');
  await client.addAgent({
    id: 'lead_architect',
    name: 'Lead Architect',
    role: 'System Architect',
    system_prompt: 'You are the Lead Architect. When receiving a directive, synthesize the architecture and call publish_architecture_spec to record your design.',
    tools: ['publish_architecture_spec'],
    provider: 'doubleword',
    model: 'deepseek-ai/DeepSeek-V4.1-Flash',
    temperature: 0.1
  });

  console.log('Adding Systems Developer with temperature 0.7 (creative exploration)...');
  await client.addAgent({
    id: 'systems_developer',
    name: 'Systems Developer',
    role: 'Systems Engineer',
    parent_id: 'lead_architect',
    system_prompt: 'You are the Systems Developer. Implement high-performance data structures and report back.',
    tools: ['write_memory', 'read_memory'],
    provider: 'doubleword',
    model: 'deepseek-ai/DeepSeek-V4.1-Flash',
    temperature: 0.7
  });

  const agents = await client.getAgents();
  console.log(`Successfully added ${agents.length} agents:`);
  for (const a of agents) {
    console.log(` - [${a.id}] ${a.name} (${a.role}) | Provider: ${a.provider || 'default'} | Model: ${a.model || 'default'} | Temp: ${a.temperature ?? 'default'}`);
  }

  console.log('\nQueueing directive for Lead Architect...');
  await client.queueMessage(
    'lead_architect',
    'Directive: Design a scalable JSONL batch processing queue that can poll Doubleword /v1/batches and wake up agents upon completion. Call publish_architecture_spec to record the design.'
  );

  console.log('Starting company orchestration turn...');
  const start = Date.now();
  await client.start();
  const elapsed = ((Date.now() - start) / 1000).toFixed(2);
  console.log(`Orchestration turn completed in ${elapsed}s.\n`);

  console.log('=== Checking Agent Context & Telemetry ===');
  const leadContext: any = await client.getAgentContext('lead_architect');
  console.log('Lead Architect History:');
  if (leadContext && leadContext.history) {
    for (const turn of leadContext.history) {
      console.log(` [${turn.role.toUpperCase()}]: ${turn.content}`);
    }
  }

  const updatedAgents = await client.getAgents();
  const leadAgent = updatedAgents.find(a => a.id === 'lead_architect');
  console.log('\nTelemetry for Lead Architect:', leadAgent?.telemetry);
  console.log('Async tool executed:', asyncToolExecuted);

  if (leadAgent?.telemetry && leadAgent.telemetry.total_tokens > 0) {
    console.log('\n✅ SUCCESS: Doubleword test company with async tool completed successfully!');
  } else {
    throw new Error('Telemetry reported 0 tokens or turn failed.');
  }
}

runTestCompany().catch(err => {
  console.error('Test company failed:', err);
  process.exit(1);
});
