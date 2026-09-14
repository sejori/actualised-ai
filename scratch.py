import re
with open("packages/sdk/examples/server.ts", "r", encoding="utf-8") as f:
    c = f.read()

new_endpoint = """app.post('/api/webhooks/telegram', async (c) => {
  const payload = await c.req.json();
  if (payload && payload.message && payload.message.text) {
    const text = payload.message.text;
    const client = await requireClient(c);
    
    // We queue the telegram message directly to the Project Manager's context
    // and instruct them to handle it.
    const instruction = `[TELEGRAM MESSAGE FROM USER]: ${text}`;
    await client.queueMessage('node_project_manager', instruction);
    
    // Wake up the orchestrator
    await client.start();
    
    publishStateChanged();
  }
  return c.json({ ok: true });
});

"""

c = c.replace("app.post('/api/webhooks/github', async (c) => {", new_endpoint + "app.post('/api/webhooks/github', async (c) => {")

with open("packages/sdk/examples/server.ts", "w", encoding="utf-8") as f:
    f.write(c)
