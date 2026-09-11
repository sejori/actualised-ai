import { Hono } from 'hono';
import { serve } from '@hono/node-server';
import { serveStatic } from '@hono/node-server/serve-static';
import { setCookie, getCookie } from 'hono/cookie';
import { ActualisedClient } from '../src/index';

const app = new Hono();
const port = Number(process.env.PORT) || 8080;

const streamClients = new Set<ReadableStreamDefaultController<Uint8Array>>();
const encoder = new TextEncoder();

const clientMap = new Map<string, ActualisedClient>();

const requireClient = async (c: any) => {
  const token = getCookie(c, 'session_token');
  if (!token) throw new Error('Unauthorized');
  
  if (clientMap.has(token)) return clientMap.get(token)!;
  
  const client = await ActualisedClient.createWithToken(process.env.SURREALDB_URL || 'mem://', token);
  clientMap.set(token, client);
  return client;
};

const snapshot = async (client: ActualisedClient) => {
  const [companyName, agents, projects, tools] = await Promise.all([
    client.getCompanyName(),
    client.getAgents(),
    client.getProjects(),
    client.getTools(),
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
  return c.json({ status: 'ready' });
});

app.post('/api/auth/signup', async (c) => {
  const { email, password } = await c.req.json();
  const token = await ActualisedClient.signup(process.env.SURREALDB_URL || 'mem://', email, password);
  setCookie(c, 'session_token', token, { path: '/' });
  return c.json({ success: true, token });
});

app.post('/api/auth/signin', async (c) => {
  const { email, password } = await c.req.json();
  const token = await ActualisedClient.signin(process.env.SURREALDB_URL || 'mem://', email, password);
  setCookie(c, 'session_token', token, { path: '/' });
  return c.json({ success: true, token });
});

app.post('/api/auth/logout', async (c) => {
  setCookie(c, 'session_token', '', { path: '/', maxAge: 0 });
  return c.json({ success: true });
});

app.get('/api/bootstrap', async (c) => c.json(await snapshot(await requireClient(c))));
app.get('/api/company', async (c) => c.json({ name: await (await requireClient(c)).getCompanyName() }));
app.post('/api/company', async (c) => {
  const body = await c.req.json<{ name: string }>();
  await (await requireClient(c)).foundCompany(body.name);
  publishStateChanged();
  return c.json(await snapshot(await requireClient(c)), 201);
});
app.get('/api/agents/:id/context', async (c) => c.json(await (await requireClient(c)).getAgentContext(c.req.param('id'))));
app.get('/api/agents/:id/memory-tree', async (c) => c.json(await (await requireClient(c)).getAgentMemoryTree(c.req.param('id'))));
app.get('/api/shared-tree', async (c) => c.json(await (await requireClient(c)).getSharedTree()));

app.post('/api/agents', async (c) => {
  await (await requireClient(c)).addAgent(await c.req.json());
  publishStateChanged();
  return c.json(await snapshot(await requireClient(c)));
});
app.put('/api/agents/:id', async (c) => {
  await (await requireClient(c)).updateAgent(c.req.param('id'), await c.req.json());
  publishStateChanged();
  return c.json(await snapshot(await requireClient(c)));
});
app.delete('/api/agents/:id', async (c) => {
  await (await requireClient(c)).removeAgent(c.req.param('id'));
  publishStateChanged();
  return c.json(await snapshot(await requireClient(c)));
});
app.post('/api/agents/:id/messages', async (c) => {
  const body = await c.req.json<{ message: string }>();
  await (await requireClient(c)).queueMessage(c.req.param('id'), body.message);
  publishStateChanged();
  return c.json({ accepted: true });
});
app.post('/api/config/inference', async (c) => {
  await (await requireClient(c)).configureInference(await c.req.json());
  return c.json({ configured: true });
});
app.post('/api/config/rate-limits', async (c) => {
  const body = await c.req.json<{ requests_per_minute: number; working_hours?: [string, string] | null }>();
  await (await requireClient(c)).setPacing({
    requestsPerMinute: body.requests_per_minute,
    workingHours: body.working_hours ? { start: body.working_hours[0], end: body.working_hours[1] } : undefined,
  });
  return c.json({ configured: true });
});
app.post('/api/orchestrator/run', async (c) => {
  await (await requireClient(c)).start();
  publishStateChanged();
  return c.json(await snapshot(await requireClient(c)));
});
app.post('/api/shared-files', async (c) => {
  await (await requireClient(c)).addSharedFile(await c.req.json());
  publishStateChanged();
  return c.json({ created: true });
});
app.delete('/api/tools/:name', async (c) => {
  await (await requireClient(c)).removeTool(c.req.param('name'));
  publishStateChanged();
  return c.json(await snapshot(await requireClient(c)));
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

// Serve the Web UI built files
app.use('/*', serveStatic({ root: '../web-ui/dist' }));
// Fallback to index.html for SPA routing
app.get('/*', serveStatic({ root: '../web-ui/dist', path: 'index.html' }));

console.log(`Self-developing orchestrator listening on port ${port}`);
serve({
  fetch: app.fetch,
  port
});
