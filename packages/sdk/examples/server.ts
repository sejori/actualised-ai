import { Hono } from 'hono';
import { serve } from '@hono/node-server';
import { serveStatic } from '@hono/node-server/serve-static';
import { ActualisedClient } from '../src/index';

const app = new Hono();
const port = Number(process.env.PORT) || 8080;

const client = new ActualisedClient(process.env.SURREALDB_URL || 'surreal-cloud-url');

app.get('/health', (c) => c.text('OK'));

app.get('/api/orchestrator/stream', async (c) => {
  return new Response(new ReadableStream({
    async start(controller) {
      try {
        await client.setPacing({
          requestsPerMinute: 5.0,
          workingHours: { start: '09:00', end: '17:00' }
        });
        
        controller.enqueue(`data: ${JSON.stringify({ type: 'status', message: 'Orchestrator connected' })}\n\n`);
      } catch (err) {
        console.error(err);
      }

      const interval = setInterval(() => {
        controller.enqueue(': keepalive\n\n');
      }, 30000);

      // We can't cleanly detect close in standard ReadableStream without a polyfill or wrapping,
      // but Hono allows stream response natively.
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
app.use('/*', serveStatic({ path: '../web-ui/dist/index.html' }));

console.log(`Self-developing orchestrator listening on port ${port}`);
serve({
  fetch: app.fetch,
  port
});
