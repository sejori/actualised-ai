import re
with open("packages/sdk/examples/server.ts", "r", encoding="utf-8") as f:
    c = f.read()

old_route = """app.post('/api/shared-files', async (c) => {
  await (await requireClient(c)).addSharedFile(await c.req.json());
  publishStateChanged();
  return c.json({ created: true });
});"""

new_route = """app.post('/api/webhooks/github', async (c) => {
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
});"""
c = c.replace(old_route, new_route)

# Since token is passed via cookie usually, we might need to allow token in query string for requireClient
require_client = """async function requireClient(c: Context) {
  const token = getCookie(c, 'session_token');"""
new_require_client = """async function requireClient(c: Context) {
  const token = getCookie(c, 'session_token') || c.req.query('token');"""
c = c.replace(require_client, new_require_client)

with open("packages/sdk/examples/server.ts", "w", encoding="utf-8") as f:
    f.write(c)

