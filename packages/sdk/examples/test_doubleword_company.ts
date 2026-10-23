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

  console.log('Adding Lead Architect with temperature 0.1 (low temp for precision)...');
  await client.addAgent({
    id: 'lead_architect',
    name: 'Lead Architect',
    role: 'System Architect',
    system_prompt: 'You are the Lead Architect. When receiving a directive, synthesize the architecture and use send_message or create_sub_project to assign tasks.',
    tools: ['create_sub_project', 'send_message'],
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
    'Directive: Design a scalable JSONL batch processing queue that can poll Doubleword /v1/batches and wake up agents upon completion. Briefly list the 3 core components.'
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

  if (leadAgent?.telemetry && leadAgent.telemetry.total_tokens > 0) {
    console.log('\n SUCCESS: Phase 1 Doubleword test company completed successfully!');
  } else {
    throw new Error('Telemetry reported 0 tokens or turn failed.');
  }
}

runTestCompany().catch(err => {
  console.error('Test company failed:', err);
  process.exit(1);
});
