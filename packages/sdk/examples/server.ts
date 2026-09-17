import { Hono } from 'hono';
import { serve } from '@hono/node-server';
import { serveStatic } from '@hono/node-server/serve-static';
import { setCookie, getCookie } from 'hono/cookie';
import { ActualisedClient } from '../src/index';
import { CompanyRunner } from './company-runner';

const app = new Hono();
const port = Number(process.env.PORT) || 8080;

const streamClients = new Set<ReadableStreamDefaultController<Uint8Array>>();
const encoder = new TextEncoder();

const clientMap = new Map<string, ActualisedClient>();
const runnerMap = new Map<string, CompanyRunner>();
const transientRunnerKeys = new WeakMap<ActualisedClient, string>();
let transientRunnerSequence = 0;

const getRunner = async (client: ActualisedClient) => {
  const companyId = await client.getCompanyId();
  let key = companyId ?? transientRunnerKeys.get(client);
  if (!key) {
    key = `transient:${++transientRunnerSequence}`;
    transientRunnerKeys.set(client, key);
  }
  let runner = runnerMap.get(key);
  if (!runner) {
    runner = new CompanyRunner(client, 500, publishStateChanged);
    runnerMap.set(key, runner);
  }
  return runner;
};

const restoreCompanyRunners = async () => {
  const dbPath = process.env.SURREALDB_URL || 'mem://';
  await ActualisedClient.create(dbPath);
  const companyIds = await ActualisedClient.getRunningCompanyIds(dbPath);
  await Promise.all(companyIds.map(async companyId => {
    const client = await ActualisedClient.createSystemClient(dbPath, companyId);
    clientMap.set(`system:${companyId.replace('company:', '')}`, client);
    await (await getRunner(client)).restore();
  }));
  console.log(`Restored ${companyIds.length} enabled company runner(s)`);
};

if (process.env.NODE_ENV !== 'test') {
  void restoreCompanyRunners().catch(error => console.error('Failed to restore company runners:', error));
}

const requireClient = async (c: any) => {
  const token = getCookie(c, 'session_token') || c.req.query('token');
  if (!token) throw new Error('Unauthorized');
  
  let companyId = c.req.header('X-Company-ID');
  
  // For webhooks, if no explicit company header is provided, fallback to the query parameter or the user's first available company.
  if (!companyId) {
    companyId = c.req.query('companyId');
    if (!companyId) {
      const companies = await ActualisedClient.getCompanies(process.env.SURREALDB_URL || 'mem://', token);
      if (companies && companies.length > 0) {
        companyId = (companies[0] as any).id;
      }
    }
  }
  
  const cacheKey = `${token}:${companyId || 'default'}`;
  if (clientMap.has(cacheKey)) return clientMap.get(cacheKey)!;
  
  const client = await ActualisedClient.createWithToken(process.env.SURREALDB_URL || 'mem://', token, companyId);
  clientMap.set(cacheKey, client);
  await (await getRunner(client)).restore();
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

type TelegramClient = Pick<ActualisedClient, 'queueMessage' | 'start' | 'getAgentContext'>;

const sendTelegramMessage = async (token: string, chatId: string | number, text: string) => {
  const response = await fetch(`https://api.telegram.org/bot${token}/sendMessage`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ chat_id: chatId, text }),
    signal: AbortSignal.timeout(15000),
  });
  if (!response.ok) throw new Error(`Telegram sendMessage failed with status ${response.status}`);
};

export const processTelegramMessage = async (
  client: TelegramClient,
  rootAgentId: string,
  text: string,
  telegramToken: string,
  chatId: string | number,
  sendMessage = sendTelegramMessage,
) => {
  await client.queueMessage(rootAgentId, `[TELEGRAM MESSAGE FROM USER]: ${text}\n\nNOTE: You MUST reply to the user using the 'telegram_notify' tool immediately.`);
  
  let loopRunning = true;
  let maxTurns = 15;
  let currentTurn = 0;

  while (loopRunning && currentTurn < maxTurns) {
    currentTurn++;
    const contextBefore = await client.getAgentContext(rootAgentId) as { history?: Array<{ role: string; content: string }> };
    const historyLengthBefore = contextBefore.history?.length ?? 0;

    await client.start();

    const contextAfter = await client.getAgentContext(rootAgentId) as { history?: Array<{ role: string; content: string }> };
    const historyAfter = contextAfter.history ?? [];
    
    // Find turns added during this start() cycle
    const newTurns = historyAfter.slice(historyLengthBefore);
    
    if (newTurns.length === 0) {
      break;
    }

    const latestTurn = newTurns[newTurns.length - 1];

    if (latestTurn.role === 'operator' && latestTurn.content.startsWith('System Error')) {
      // The agent errored out and the system queued an error message for it. Retry.
      console.warn('Agent encountered a system error, retrying...');
      loopRunning = true;
      continue;
    }

    // Process new agent replies
    const newAgentReplies = newTurns.filter(turn => turn.role === 'agent');
    if (newAgentReplies.length === 0) {
      break;
    }

    const latestAgentReply = newAgentReplies[newAgentReplies.length - 1].content;

    const usedTelegramTool = latestAgentReply.startsWith('Called tools:') && latestAgentReply.includes('telegram_notify');
    
    if (!usedTelegramTool) {
      await sendMessage(telegramToken, chatId, latestAgentReply);
    }

    if (latestAgentReply.startsWith('Called tools:')) {
      // The agent used tools, so we should run another inference cycle to resolve them.
      loopRunning = true;
    } else {
      // The agent replied with a final message. Loop ends.
      loopRunning = false;
    }
  }

  if (loopRunning && currentTurn >= maxTurns) {
    await sendMessage(telegramToken, chatId, "I'm sorry, I hit my maximum loop limit while processing your request. I've stopped to save resources. Please try rephrasing your request.");
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


app.get('/api/companies', async (c) => {
  const token = getCookie(c, 'session_token') || c.req.query('token');
  if (!token) return c.json({ error: 'Unauthorized' }, 401);
  return c.json(await ActualisedClient.getCompanies(process.env.SURREALDB_URL || 'mem://', token));
});

app.delete('/api/companies/:id', async (c) => {
  const id = c.req.param('id');
  const client = await requireClient(c);
  await client.deleteCompany(id);
  // Optional: clear cache keys for this company
  const token = getCookie(c, 'session_token') || c.req.query('token');
  clientMap.delete(`${token}:${id}`);
  return c.json({ deleted: true });
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
app.get('/api/orchestrator/status', async (c) => {
  const client = await requireClient(c);
  return c.json((await getRunner(client)).status());
});
app.post('/api/orchestrator/start', async (c) => {
  const client = await requireClient(c);
  return c.json(await (await getRunner(client)).start());
});
app.post('/api/orchestrator/pause', async (c) => {
  const client = await requireClient(c);
  return c.json(await (await getRunner(client)).pause());
});
app.post('/api/orchestrator/run', async (c) => {
  const client = await requireClient(c);
  await client.start();
  await (await getRunner(client)).restore();
  publishStateChanged();
  return c.json(await snapshot(client));
});
app.put('/api/settings', async (c) => {
  const settings = await c.req.json();
  const client = await requireClient(c);
  
  // Save settings to database
  await client.updateCompanySettings(settings);

  // If a Telegram Bot Token is provided, register the webhook automatically
  if (settings.telegramBotToken) {
    const companyId = c.req.header('X-Company-ID') || c.req.query('companyId') || await client.getCompanyName().then(async () => {
      const token = getCookie(c, 'session_token') || c.req.query('token');
      const companies = await ActualisedClient.getCompanies(process.env.SURREALDB_URL || 'mem://', token as string);
      return (companies[0] as any).id;
    });

    if (companyId) {
      // Extract the raw ID part if it contains 'company:'
      const rawId = companyId.replace('company:', '');
      const host = process.env.BASE_URL || 'https://actualised-orchestrator-658050940120.europe-west2.run.app';
      const webhookUrl = `${host}/api/webhooks/telegram/${rawId}`;
      
      try {
        const response = await fetch(`https://api.telegram.org/bot${settings.telegramBotToken}/setWebhook?url=${encodeURIComponent(webhookUrl)}`);
        if (!response.ok) throw new Error(`Telegram setWebhook failed with status ${response.status}`);
        console.log(`Registered Telegram webhook for company ${rawId} to ${webhookUrl}`);
      } catch (err) {
        console.error('Failed to register telegram webhook', err);
      }
    }
  }

  return c.json({ success: true });
});

app.post('/api/webhooks/telegram/:companyId', async (c) => {
  const companyId = c.req.param('companyId');
  if (!companyId) return c.json({ ok: true });

  const payload = await c.req.json();
  if (payload && payload.message && payload.message.text) {
    const text = payload.message.text;
    const chatId = payload.message.chat?.id;
    if (!chatId) return c.json({ ok: true });

    try {
      // Create or retrieve a system client bound to this company's scope
      const cacheKey = `system:${companyId}`;
      let client = clientMap.get(cacheKey);
      if (!client) {
        client = await ActualisedClient.createSystemClient(process.env.SURREALDB_URL || 'mem://', `company:${companyId}`);
        clientMap.set(cacheKey, client);
      }
      
      const settings = await client.getCompanySettings();
      const telegramToken = settings?.telegramBotToken;
      console.log(`Telegram webhook received: company=${companyId} token=${telegramToken ? 'SET' : 'MISSING'}`);
      
      // Automatically capture and save the chat ID so agents can push notifications back to the user later
      if (settings && !settings.telegramChatId) {
        settings.telegramChatId = chatId;
        await client.updateCompanySettings(settings);
      }

      if (telegramToken) {
        fetch(`https://api.telegram.org/bot${telegramToken}/sendChatAction`, {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ chat_id: chatId, action: 'typing' }),
        }).catch(() => {});
      } else {
        console.warn(`Telegram webhook received for company ${companyId} but no telegramBotToken is configured.`);
      }

      // Find the root agent dynamically
      const agents = await client.getAgents();
      const rootAgent = agents.find((a: any) => !a.parent_id);
      if (!rootAgent) {
        console.error('Telegram webhook: no root agent found for company', companyId);
        return c.json({ ok: true });
      }

      if (!telegramToken) return c.json({ ok: true });

      (async () => {
        try {
          await processTelegramMessage(client, rootAgent.id, text, telegramToken, chatId);
          await (await getRunner(client)).restore();
          publishStateChanged();
        } catch (error) {
          console.error('Telegram processing error:', error);
          const errorMessage = error instanceof Error ? error.message : String(error);
          await sendTelegramMessage(telegramToken, chatId, `System error while processing your message: ${errorMessage}`)
            .catch(sendError => console.error('Failed to send Telegram error response:', sendError));
        }
      })();
    } catch (e) {
      console.error('Telegram webhook error processing message:', e);
    }
  }
  return c.json({ ok: true });
});

app.post('/api/webhooks/github', async (c) => {
  // Extract standard GitHub webhook payload
  const payload = await c.req.json();
  const event = c.req.header('x-github-event');
  
  if (event === 'issues' || event === 'issue_comment') {
    const issueId = payload.issue.number.toString();
    const title = payload.issue.title;
    const body = payload.issue.body || '';
    const state = payload.issue.state;
    // For simplicity we just use login name.
    const assignee = payload.issue.assignee ? payload.issue.assignee.login : null; 
    
    // Convert to our internal Issue format
    const internalIssue = {
      id: issueId,
      title,
      body,
      state,
      author: payload.issue.user.login,
      assignee,
      labels: payload.issue.labels.map((l: any) => l.name),
      comments: [] // We could parse comments here if needed, or rely on fetching them
    };
    
    // For simplicity in a multi-tenant setup, this needs a valid auth context or a dedicated webhook token mapped to a company.
    // Assuming requireClient will authenticate via query param `?token=` for webhook
    const client = await requireClient(c);
    await client.syncIssue(JSON.stringify(internalIssue));
    
    // Auto-trigger the inference loop so the assigned agent responds
    await client.start();
    
    publishStateChanged();
    return c.json({ processed: true });
  }
  
  return c.json({ ignored: true });
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

export { app };

if (process.env.NODE_ENV !== 'test') {
  console.log(`Self-developing orchestrator listening on port ${port}`);
  serve({
    fetch: app.fetch,
    port
  });
}
