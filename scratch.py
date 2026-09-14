import re
with open("packages/sdk/examples/server.ts", "r", encoding="utf-8") as f:
    c = f.read()

telegram_webhook_patch = """app.post('/api/webhooks/telegram', async (c) => {
  const payload = await c.req.json();
  if (payload && payload.message && payload.message.text) {
    const text = payload.message.text;
    const client = await requireClient(c);
    
    // We queue the telegram message directly to the Project Manager's context
    // and instruct them to handle it.
    const instruction = `[TELEGRAM MESSAGE FROM USER]: ${text}\\n\\nNOTE: You MUST reply to the user using the 'telegram_notify' tool immediately.`;
    
    // Process asynchronously so we can return 200 OK immediately and prevent Telegram webhook timeouts/retries
    setTimeout(async () => {
      try {
        await client.queueMessage('node_project_manager', instruction); // Blocks here if inference is active
        await client.start(); // Triggers response, blocking subsequent tasks linearly
        publishStateChanged();
      } catch (e) {
        console.error("Telegram background processing error:", e);
      }
    }, 0);
  }
  return c.json({ ok: true });
});"""

c = re.sub(r"app\.post\('/api/webhooks/telegram'.*?return c\.json\(\{ ok: true \}\);\n\}\);", telegram_webhook_patch, c, flags=re.DOTALL)

with open("packages/sdk/examples/server.ts", "w", encoding="utf-8") as f:
    f.write(c)
