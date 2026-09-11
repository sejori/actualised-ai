import { ActualisedClient } from '../src/index';

async function test() {
  const dbPath = 'surrealkv://test.db';
  console.log('Bootstrapping schema...');
  // This will initialize the schema using ROOT credentials (since SURREALDB_USER is not present, mem:// is just empty without auth, wait... mem:// doesn't require root auth by default!)
  await ActualisedClient.create(dbPath);

  console.log('Testing signup...');
  const token = await ActualisedClient.signup(dbPath, 'test@example.com', 'password123');
  console.log('Signup successful, token:', token);

  console.log('Testing signin...');
  const token2 = await ActualisedClient.signin(dbPath, 'test@example.com', 'password123');
  console.log('Signin successful, token:', token2);
}
test().catch(console.error);
