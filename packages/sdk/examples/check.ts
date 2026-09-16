import { ActualisedClient } from '../src/index';

async function check() {
  const dbUrl = process.env.SURREALDB_URL || 'wss://actualised-db-06g8gatlddrq53037u2ngf871c.aws-euw1.surreal.cloud';
  const email = 'seb@actualised.ai';
  const pass = 'password123';
  
  const token = await ActualisedClient.signin(dbUrl, email, pass);
  const companiesStr = await (ActualisedClient as any).getCompanies(dbUrl, token);
  const companies = typeof companiesStr === 'string' ? JSON.parse(companiesStr) : companiesStr;
  const companyId = companies[0].id;
  
  const client = await ActualisedClient.createWithToken(dbUrl, token, companyId);
  const agents = await client.getAgents();
  console.log('Current agents:');
  for (const a of agents) {
    console.log(`${a.id} (parent: ${a.parent_id}) - ${a.name}`);
  }
}
check().catch(console.error);
