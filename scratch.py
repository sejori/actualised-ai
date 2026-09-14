import re
with open("packages/sdk/examples/server.ts", "r", encoding="utf-8") as f:
    c = f.read()

patch = """const requireClient = async (c: any) => {
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
  return client;
};"""

c = re.sub(r"const requireClient = async \(c: any\) => \{\n  const token = getCookie\(c, 'session_token'\) \|\| c\.req\.query\('token'\);\n  if \(!token\) throw new Error\('Unauthorized'\);\n  \n  const companyId = c\.req\.header\('X-Company-ID'\);\n  const cacheKey = `\$\{token\}:\$\{companyId \|\| 'default'\}\`;\n  if \(clientMap\.has\(cacheKey\)\) return clientMap\.get\(cacheKey\)!\;\n  \n  const client = await ActualisedClient\.createWithToken\(process\.env\.SURREALDB_URL \|\| 'mem://', token, companyId\);\n  clientMap\.set\(cacheKey, client\);\n  return client;\n\};", patch, c, flags=re.DOTALL)

with open("packages/sdk/examples/server.ts", "w", encoding="utf-8") as f:
    f.write(c)
