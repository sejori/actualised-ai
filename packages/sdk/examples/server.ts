import { Hono } from 'hono';
import { serve } from '@hono/node-server';
import { serveStatic } from '@hono/node-server/serve-static';
import { ActualisedClient } from '../src/index';

const app = new Hono();
const port = Number(process.env.PORT) || 8080;

let client: ActualisedClient;
let initializationError: Error | undefined;
const streamClients = new Set<ReadableStreamDefaultController<Uint8Array>>();
const encoder = new TextEncoder();

const clientReady = ActualisedClient.create(process.env.SURREALDB_URL || 'mem://')
  .then(c => {
    client = c;
    console.log('Client initialized');
    return c;
  })
  .catch(err => {
    initializationError = err instanceof Error ? err : new Error(String(err));
    console.error('Failed to init client', initializationError.message);
    throw initializationError;
  });

const requireClient = async () => {
  if (client) return client;
  return clientReady;
};

const snapshot = async () => {
  const readyClient = await requireClient();
  const [companyName, agents, projects, tools] = await Promise.all([
    readyClient.getCompanyName(),
    readyClient.getAgents(),
    readyClient.getProjects(),
    readyClient.getTools(),
  ]);
  return { company_name: companyName, agents, projects, tools };
};

const publishStateChanged = () => {
  const event = encoder.encode(`data: ${JSON.stringify({ type: 'state_changed' })}\n\n`);
  for (const controller of streamClients) {
    try {
      controller.enqueue(event);
    } catch {
      streamClients.delete(controller);
    }
  }
};

app.onError((error, c) => {
  console.error('Request failed:', error.message);
  return c.json({ error: error.message }, 500);
});

app.get('/health', (c) => {
  if (client) return c.json({ status: 'ready' });
  return c.json({ status: 'not_ready' }, 503);
});

app.get('/api/bootstrap', async (c) => c.json(await snapshot()));
app.get('/api/company', async (c) => c.json({ name: await (await requireClient()).getCompanyName() }));
app.post('/api/company', async (c) => {
  const body = await c.req.json<{ name: string }>();
  await (await requireClient()).foundCompany(body.name);
  publishStateChanged();
  return c.json(await snapshot(), 201);
});
app.get('/api/agents/:id/context', async (c) => c.json(await (await requireClient()).getAgentContext(c.req.param('id'))));
app.get('/api/agents/:id/memory-tree', async (c) => c.json(await (await requireClient()).getAgentMemoryTree(c.req.param('id'))));
app.get('/api/shared-tree', async (c) => c.json(await (await requireClient()).getSharedTree()));

app.post('/api/agents', async (c) => {
  await (await requireClient()).addAgent(await c.req.json());
  publishStateChanged();
  return c.json(await snapshot());
});
app.put('/api/agents/:id', async (c) => {
  await (await requireClient()).updateAgent(c.req.param('id'), await c.req.json());
  publishStateChanged();
  return c.json(await snapshot());
});
app.delete('/api/agents/:id', async (c) => {
  await (await requireClient()).removeAgent(c.req.param('id'));
  publishStateChanged();
  return c.json(await snapshot());
});
app.post('/api/agents/:id/messages', async (c) => {
  const body = await c.req.json<{ message: string }>();
  await (await requireClient()).queueMessage(c.req.param('id'), body.message);
  publishStateChanged();
  return c.json({ accepted: true });
});
app.post('/api/config/inference', async (c) => {
  await (await requireClient()).configureInference(await c.req.json());
  return c.json({ configured: true });
});
app.post('/api/config/rate-limits', async (c) => {
  const body = await c.req.json<{ requests_per_minute: number; working_hours?: [string, string] | null }>();
  await (await requireClient()).setPacing({
    requestsPerMinute: body.requests_per_minute,
    workingHours: body.working_hours ? { start: body.working_hours[0], end: body.working_hours[1] } : undefined,
  });
  return c.json({ configured: true });
});
app.post('/api/orchestrator/run', async (c) => {
  await (await requireClient()).start();
  publishStateChanged();
  return c.json(await snapshot());
});
app.post('/api/shared-files', async (c) => {
  await (await requireClient()).addSharedFile(await c.req.json());
  publishStateChanged();
  return c.json({ created: true });
});
app.delete('/api/tools/:name', async (c) => {
  await (await requireClient()).removeTool(c.req.param('name'));
  publishStateChanged();
  return c.json(await snapshot());
});

app.get('/api/orchestrator/stream', async (c) => {
  let streamController: ReadableStreamDefaultController<Uint8Array>;
  let keepalive: ReturnType<typeof setInterval>;
  return new Response(new ReadableStream<Uint8Array>({
    start(controller) {
      streamController = controller;
      streamClients.add(controller);
      controller.enqueue(encoder.encode(`data: ${JSON.stringify({ type: 'status', message: 'Orchestrator connected' })}\n\n`));
      keepalive = setInterval(() => controller.enqueue(encoder.encode(': keepalive\n\n')), 30000);
    },
    cancel() {
      clearInterval(keepalive);
      streamClients.delete(streamController);
    }
  }), {
    headers: {
      'Content-Type': 'text/event-stream',
      'Cache-Control': 'no-cache',
      'Connection': 'keep-alive'
    }
  });
});

const telegramToken = process.env.TELEGRAM_TOKEN;
if (telegramToken) {
  clientReady.then(async (readyClient) => {
    console.log('Starting Telegram bot polling...');
    let lastUpdateId = 0;
    while (true) {
      try {
        const res = await fetch(`https://api.telegram.org/bot${telegramToken}/getUpdates?offset=${lastUpdateId + 1}&timeout=30`);
        if (!res.ok) {
          console.error(`Telegram polling returned HTTP ${res.status}`);
          await new Promise(r => setTimeout(r, 5000));
          continue;
        }
        const data = await res.json();
        if (data.ok && data.result.length > 0) {
            for (const update of data.result) {
              lastUpdateId = update.update_id;
              if (update.message && update.message.text) {
                console.log(`Received Telegram message from chat ${update.message.chat.id}`);
                
                const agents = await readyClient.getAgents();
                const topAgent = agents.find(a => !a.parent_id);
                if (topAgent) {
                  const msg = `Message from User: ${update.message.text}`;
                  await readyClient.queueMessage(topAgent.id, msg);
                  publishStateChanged();
                  console.log(`Queued message to top-level agent: ${topAgent.name}`);
                } else {
                  console.log('No top-level agent found to route message to.');
                }
              }
            }
        }
      } catch (err) {
        console.error('Telegram polling error:', err);
        await new Promise(r => setTimeout(r, 5000));
      }
    }
  }).catch((error) => console.error('Telegram polling unavailable:', error.message));
}

// Serve the Web UI built files
app.use('/*', serveStatic({ root: '../web-ui/dist' }));
// Fallback to index.html for SPA routing
app.get('/*', serveStatic({ root: '../web-ui/dist', path: 'index.html' }));

console.log(`Self-developing orchestrator listening on port ${port}`);
serve({
  fetch: app.fetch,
  port
});
