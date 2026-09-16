let db = new Surreal();
await db.connect('mem://');
let res = await db.query("RETURN NONE ?? 'Unnamed Agent'");
console.log(res);
