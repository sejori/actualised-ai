import re

with open("crates/core/src/inference.rs", "r", encoding="utf-8") as f:
    c = f.read()

c = c.replace('parameters: json!({ "type": "object" }),', 'parameters: json!({ "type": "object" }),\n            company_id: None,')
c = c.replace('parameters: json!({}),', 'parameters: json!({}),\n            company_id: None,')

with open("crates/core/src/inference.rs", "w", encoding="utf-8") as f:
    f.write(c)
