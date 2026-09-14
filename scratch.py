import re
with open("packages/sdk/native.d.ts", "r", encoding="utf-8") as f:
    c = f.read()

if "syncIssue" not in c:
    c = c.replace("start(): Promise<void>;", "start(): Promise<void>;\n  syncIssue(issueJson: string): Promise<void>;")

with open("packages/sdk/native.d.ts", "w", encoding="utf-8") as f:
    f.write(c)
