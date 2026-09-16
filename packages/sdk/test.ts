import { Surreal } from 'surrealdb';
async function run() {
  let db = new Surreal();
  await db.connect('mem://');
  let res = await db.query("RETURN NONE ?? 'Unnamed Agent'; RETURN {}.name ?? 'Unnamed'; RETURN {name: NONE}.name ?? 'Unnamed'");
  console.log(JSON.stringify(res));
}
run();
