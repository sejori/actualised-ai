import { ActualisedClient } from '../src/index';

async function runBatchTestCompany() {
  console.log('=== Running Phase 2 Test Company with Doubleword Batch Processing ===');

  const apiKey = process.env.DOUBLEWORD_API_KEY || 'sk-U-5Q1NFbZaI7nsuSSmCqzqcn_AOQLe9wirAJUOQTZH0';
  process.env.DOUBLEWORD_API_KEY = apiKey;
  const client = await ActualisedClient.create('mem://');

  console.log('Configuring company inference to Doubleword...');
  await client.configureInference({
    provider: 'doubleword',
    model: 'deepseek-ai/DeepSeek-V4.1-Flash',
    apiKey: apiKey,
    baseUrl: 'https://api.doubleword.ai/v1',
    temperature: 0.2
  });

  console.log('Adding Batch Operations Lead Agent...');
  await client.addAgent({
    id: 'batch_lead',
    name: 'Batch Operations Lead',
    role: 'Batch Workload Manager',
    system_prompt: 'You are the Batch Operations Lead. When asked to submit a batch job, you MUST call the submit_batch_job tool with the requested items.',
    tools: ['submit_batch_job', 'check_batch_status', 'write_memory', 'read_memory'],
    provider: 'doubleword',
    model: 'deepseek-ai/DeepSeek-V4.1-Flash',
    temperature: 0.1
  });

  const agents = await client.getAgents();
  console.log(`Registered agent: ${agents[0].name} with tools: ${agents[0].tools.join(', ')}`);

  console.log('\nQueueing directive to submit a 2-item asynchronous batch job...');
  await client.queueMessage(
    'batch_lead',
    'Directive: Submit a batch job using the submit_batch_job tool containing 2 items:\n' +
    'Item 1 (custom_id: "sword-desc"): "Generate a 1-sentence description of an ancient iron longsword."\n' +
    'Item 2 (custom_id: "potion-desc"): "Generate a 1-sentence description of a glowing crimson healing potion."'
  );

  console.log('Starting orchestration cycle (Agent turn + Batch submission)...');
  const start = Date.now();
  await client.start();
  const elapsed = ((Date.now() - start) / 1000).toFixed(2);
  console.log(`First orchestration turn completed in ${elapsed}s.\n`);

  console.log('=== Checking Batch Jobs in Company State ===');
  const batchJobs = await client.getBatchJobs();
  console.log(`Found ${batchJobs.length} batch job(s) in state:`);
  for (const job of batchJobs as any[]) {
    console.log(` - ID: ${job.id} | Status: ${job.status} | Total: ${job.total_requests} | Req Agent: ${job.requesting_agent_id}`);
  }

  if (batchJobs.length === 0) {
    const context: any = await client.getAgentContext('batch_lead');
    console.log('Agent context history:');
    for (const h of context.history || []) {
      console.log(` [${h.role}]:`, h.content || h.tool_calls);
    }
    throw new Error('No batch jobs were submitted in the first orchestration turn.');
  }

  const activeBatch: any = batchJobs[0];
  console.log(`\nSuccessfully submitted batch job with ID: ${activeBatch.id}`);

  // Test polling status
  console.log('Polling batch status over the next few cycles...');
  let completed = false;
  for (let attempt = 1; attempt <= 10; attempt++) {
    console.log(`Poll cycle ${attempt}/10...`);
    await client.start();

    const currentJobs: any = await client.getBatchJobs();
    const updated = currentJobs.find((j: any) => j.id === activeBatch.id);
    if (updated) {
      console.log(` -> Status: ${updated.status} (Completed: ${updated.completed_requests}/${updated.total_requests})`);
      if (updated.status === 'completed' || updated.status === 'failed') {
        completed = true;
        if (updated.status === 'completed') {
          console.log('\n Batch job completed successfully!');
          console.log('Output file ID:', updated.output_file_id);
        }
        break;
      }
    }
    await new Promise(r => setTimeout(r, 4000));
  }

  console.log('\n=== Verifying Agent Turn Context ===');
  const leadContext: any = await client.getAgentContext('batch_lead');
  for (const turn of leadContext.history || []) {
    if (turn.role === 'assistant') {
      console.log(`[ASSISTANT]: ${turn.content || JSON.stringify(turn.tool_calls)}`);
    } else if (turn.role === 'tool') {
      console.log(`[TOOL RESPONSE]: ${turn.content}`);
    } else {
      console.log(`[${turn.role.toUpperCase()}]: ${turn.content}`);
    }
  }

  console.log('\n SUCCESS: Phase 2 Doubleword batch engine test company verified!');
}

runBatchTestCompany().catch(err => {
  console.error('Phase 2 test company failed:', err);
  process.exit(1);
});
