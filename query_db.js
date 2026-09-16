const url = 'https://actualised-db-06g8gatlddrq53037u2ngf871c.aws-euw1.surreal.cloud';

async function run() {
  let res = await fetch(`https://actualised-orchestrator-658050940120.europe-west2.run.app/api/auth/signin`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ email: 'seb@actualised.ai', password: 'password123' })
  });
  const { token } = await res.json();

  res = await fetch(`${url}/sql`, {
    method: 'POST',
    headers: { 
      'Accept': 'application/json',
      'NS': 'actualised',
      'DB': 'core',
      'Authorization': `Bearer ${token}`
    },
    body: `
      CREATE agent:test_with_company SET company_id = "company:pkx6k4z9eemy5j17ojun";
      SELECT *, ($auth.id IN (SELECT VALUE owner FROM company WHERE type::string(id) = type::string($parent.company_id))) AS access_with_parent, ($auth.id IN (SELECT VALUE owner FROM company WHERE type::string(id) = type::string(company_id))) AS access_without_parent FROM agent WHERE id = agent:test_with_company;
    `
  });
  console.log(await res.text());
}
run().catch(console.error);
