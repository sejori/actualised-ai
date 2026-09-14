# Actualised.ai — Agent Context & Tooling Guide

A deep-dive reference for understanding how agents are structured, how context and memory work, how the inference pipeline runs, and how to extend the system with custom tools.

---

## 1. Agent Architecture

### The Agent Struct

Each agent is a record in SurrealDB with the following shape:

```typescript
interface Agent {
  id: string;           // "agent:{uuid}" — always prefixed
  company_id: string;   // "company:{uuid}" — links to parent company
  name: string;
  role: string;
  parent_id?: string;   // null = root agent; otherwise "agent:{uuid}" of parent
  system_prompt: string;
  tools: string[];      // list of Tool **names** (not Tool objects)
  telemetry?: AgentTelemetry;
  scheduled_tasks?: ScheduledTask[];
  pending_messages?: string[];  // operator messages queued for next run()
  issue_triggers?: string[];    // issue IDs to inject into this agent's prompt
}
```

### The Hierarchy Invariant

The agent tree is always rooted at exactly one agent (the one with `parent_id = null`). The `add_agent`, `update_agent`, and `remove_agent` methods in `CompanyState` actively enforce this — if you remove the root, a child is automatically promoted; if you add a new root, the old root is re-parented beneath it.

**This invariant is enforced in the DB layer, not just in memory.** Every hierarchy change triggers `UPDATE $record CONTENT $agent` queries for all affected nodes.

### Telegram routing

Incoming Telegram messages are routed to whichever agent has `parent_id = null` (the root). This is resolved dynamically at webhook time — no agent ID is hardcoded.

---

## 2. Context & Memory

### Pending Messages (Short-Term / Operator Input)

`pending_messages` on an agent are strings queued via `queueMessage(agentId, message)`. On each `run()`, the orchestrator:

1. Reads all pending messages from state
2. Appends them to the agent's `AgentContext.history` as `"operator"` role turns
3. Clears them from state

Use `queueMessage` to inject instructions (e.g. from a Telegram webhook) before calling `start()`.

### Conversation History (In-Memory)

Each agent has an in-memory `AgentContext.history: ConversationTurn[]` where each turn is:
```typescript
{ role: "operator" | "agent", content: string }
```
History is capped at **40 turns** (`MAX_HISTORY_TURNS`). When the cap is reached, older turns are dropped. History is **not persisted to SurrealDB** — it only lives in the in-process `Orchestrator` instance and is lost on restart.

### File Memory (Persistent)

Agents have persistent key-value file storage managed by `MemoryManager`:

| Path pattern | Purpose |
|---|---|
| `{base_dir}/{agent_id}/memories/{file_name}` | Per-agent memory files |
| `{base_dir}/_shared/{path}` | Cross-agent shared files |

Files are stored on disk (native) or in-memory only (WASM/browser).

#### The `index.md` File

Each agent has a special `{agent_id}/memories/index.md` file. If it exists, the orchestrator **automatically prepends its content to the agent's system prompt** on every `run()`. This is the intended place for persistent agent identity, long-term goals, and accumulated knowledge.

> **Tip:** Think of `index.md` as the agent's persistent memory brain. It should be kept concise since it is injected on every inference call.

#### Writing Memory (via Tool)

The built-in `write_memory` tool lets agents write to their own memory files during a `run()`:
```json
{
  "name": "write_memory",
  "args": { "file_name": "strategy.md", "content": "..." }
}
```

#### Reading Memory (via Tool)

The built-in `read_memory` tool queues the file content back as a pending message to the agent:
```json
{
  "name": "read_memory",
  "args": { "file_name": "strategy.md" }
}
```

#### Viewing Memory Trees (API)

```typescript
const tree = await client.getAgentMemoryTree(agentId); // per-agent
const shared = await client.getSharedTree();            // cross-agent shared files
```

Memory is returned as a nested `MemoryNode[]` tree:
```typescript
type MemoryNode =
  | { kind: "Folder"; name: string; children: MemoryNode[] }
  | { kind: "File"; name: string; path: string; content: string }
```

---

## 3. Tool System

### Tool Definition

Tools are registered in `CompanyState` via `addTool()`. A tool is:
```typescript
interface Tool {
  company_id?: string;
  name: string;        // unique key — also the DB record key
  description: string;
  parameters: object;  // JSON Schema describing the args the LLM should produce
}
```

### Agent-Tool Association

Agents reference tools by **name only** in their `tools: string[]` array. At inference time, the orchestrator resolves the full `Tool` definition from state and passes it to the LLM.

```typescript
// When adding an agent:
await client.addAgent({
  id: 'analyst',
  name: 'Data Analyst',
  system_prompt: '...',
  tools: ['web_search', 'write_memory', 'telegram_notify'] // tool names
});

// Separately register any custom tools:
await client.addTool({
  name: 'web_search',
  description: 'Search the web for information',
  parameters: {
    type: 'object',
    properties: {
      query: { type: 'string', description: 'The search query' }
    },
    required: ['query']
  }
});
```

> **Note:** Built-in tools (`write_memory`, `read_memory`, `create_sub_project`, `telegram_notify`, `github_*`) are handled natively in the orchestrator. You do **not** need to register them via `addTool()` — but you do need to include their names in the agent's `tools` array for the LLM to see them.

### Built-in Tools Reference

| Tool Name | Handler | Description |
|---|---|---|
| `write_memory` | `MemoryManager::write_memory` | Writes a file to agent's persistent memory |
| `read_memory` | `MemoryManager::read_memory` | Reads a memory file; result queued back to agent |
| `create_sub_project` | `CompanyState::add_project` | Creates a project record associated to the company |
| `telegram_notify` | `NotificationDispatcher` | Sends a message via Telegram Bot API (requires `TELEGRAM_TOKEN` + `TELEGRAM_CHAT_ID` env vars) |
| `github_read_file` | `GithubProvider` | Reads a file from the configured repository |
| `github_write_file` | `GithubProvider` | Writes a file to the repository |
| `github_list_directory` | `GithubProvider` | Lists files at a path in the repository |
| `github_search_codebase` | `GithubProvider` | Stub — not yet implemented |
| `github_create_issue` | `GithubProvider` | Creates a GitHub issue |
| `github_add_comment` | `GithubProvider` | Adds a comment to an issue |
| `github_close_issue` | `GithubProvider` | Closes an issue |

> **WASM limitation:** `telegram_notify` and all `github_*` tools are **no-ops** on WASM/browser targets and return a fallback string to the agent.

### Custom Tools (JavaScript Executor)

For tools not handled natively, register a JavaScript executor callback:

```typescript
client.registerToolExecutor(async (agentId, toolName, argsJson) => {
  const args = JSON.parse(argsJson);
  if (toolName === 'web_search') {
    const results = await mySearchApi(args.query);
    return JSON.stringify(results);
  }
  throw new Error(`Unknown tool: ${toolName}`);
});
```

> **Important:** The executor callback signature is synchronous from the Rust perspective (`=> string`). If your tool is async, the native layer blocks the thread. Plan accordingly — heavy async work should be pre-fetched or offloaded before the `start()` call.

### Tool Dispatch Order

1. Orchestrator receives `ToolCalls` from the LLM
2. **Built-in tools** are checked first by exact name match (string literal `match` in `orchestrator.rs`)
3. Unmatched tools fall through to the registered `ToolExecutor` callback
4. If no executor is registered, logs `"Unknown tool: {name}"` and continues

---

## 4. Inference Pipeline

### Flow: Single `run()` / `start()` Cycle

```
start() → Orchestrator::run()
  │
  ├─ For each agent:
  │    ├─ Resolve Tool definitions from CompanyState
  │    ├─ Collect overdue ScheduledTasks + issue_triggers
  │    ├─ Build user_prompt (pending_messages + tasks + issues)
  │    ├─ Inject index.md → system_prompt
  │    └─ Create InferenceRequest
  │
  ├─ InferenceQueue::process_batch(requests)
  │    ├─ Semaphore: max_concurrent_requests
  │    ├─ Rate limiter: dispatch_interval between requests
  │    └─ Working-hours gate: sleep 60s if outside window
  │
  └─ Handle InferenceResponse per agent:
       ├─ Text  → append to AgentContext.history
       └─ ToolCalls → dispatch (built-in → custom → log unknown)
            └─ Update telemetry in CompanyState
```

### InferenceEngine

The `InferenceEngine` trait has a single method:
```rust
generate_response(
  system_prompt: &str,
  user_prompt:   &str,
  tools:         Vec<Tool>
) -> Result<InferenceResponse, String>
```

The response is:
```rust
enum InferenceResult {
  Text(String),
  ToolCalls(Vec<ToolCall>)  // { id, name, args: serde_json::Value }
}
```

**Important:** The engine is stateless — no conversation history is passed to it. The `Orchestrator` manages history and assembles the full prompt text before each call.

### Configuring Inference

```typescript
await client.configureInference({
  provider: 'gemini',
  model: 'gemini-2.0-flash',
  api_key: process.env.GEMINI_API_KEY  // optional; falls back to GEMINI_API_KEY env var
});
```

Default model: `gemini-3.6-flash`.

### Rate Limiting & Working Hours

```typescript
await client.setPacing({
  requestsPerMinute: 10,           // 0 = unlimited
  workingHours: {
    start: '09:00',                // HH:MM local time
    end: '17:00'
  }
});
```

The `InferenceQueue` enforces:
- **Concurrency**: defaults to 5 parallel requests (one per agent running simultaneously)
- **RPM**: space dispatches at `60 / requestsPerMinute` seconds apart
- **Working hours**: if set, all dispatch is paused outside the window (checked every 60s)

> **Note:** Working-hours gating blocks the entire dispatch loop — all agents pause together, not individually.

---

## 5. Scheduled Tasks

Agents support optional scheduled tasks stored in the `scheduled_tasks` field:
```typescript
interface ScheduledTask {
  id: string;
  description: string;
  due_date: string;    // RFC3339 datetime string
  completed: boolean;
}
```

On each `run()`, the orchestrator checks for overdue tasks (where `due_date < now() AND completed = false`) and appends them to the agent's user prompt automatically. There is no built-in tool for marking tasks complete — agents must use `write_memory` or a custom tool to do so.

---

## 6. Issue Tracking Integration

Issues can be synced from GitHub (or any external system) via:
```typescript
await client.syncIssue(JSON.stringify({
  id: '42',
  title: 'Fix login bug',
  body: 'Users cannot log in on mobile',
  state: 'open',
  labels: ['bug'],
  comments: [],
  assignee: 'agent:some_agent_id'
}));
```

If an issue's `assignee` matches an agent's ID, the issue is added to that agent's `issue_triggers`. On the next `run()`, the issue content is automatically injected into that agent's prompt.

The GitHub webhook handler in `server.ts` processes `issues` and `issue_comment` events and calls `syncIssue` automatically. To assign issues to agents, set the GitHub issue assignee to the agent's name or use a label that maps to an agent.

---

## 7. WASM / Browser Target

The core crate compiles to WASM for browser use. In WASM mode:
- **No SurrealDB** — `CompanyState` is purely in-memory; state is lost on page reload
- **No file system** — `MemoryManager` uses in-memory `HashMap` only
- **No Telegram / GitHub** — built-in tools that require `reqwest` return stub results
- **No env vars** — `GEMINI_API_KEY` must be passed via `configureInference()`, not env

The WASM build is primarily for the Web UI demo. Production deployments use the Node.js NAPI build.

---

## 8. Known Limitations & Stubs

| Feature | Status |
|---|---|
| `web_search` tool | **Stub** — returns hardcoded mock results. Not connected to any search API. |
| `github_search_codebase` | **Stub** — not implemented in `GithubProvider`. |
| `create_commit` (GithubClient) | **Stub** — prints a log line only. The actual API call is commented-out pseudocode. |
| `delete_company` | **Broken** — the SurrealQL query has incomplete `WHERE` clauses for cascade deletion. |
| `registerToolExecutor` sync constraint | The JS executor is called synchronously. Async JS tools require workarounds. |
| Conversation history persistence | History is in-memory only. Lost on Cloud Run container restart / cold start. |
| `task_graph` (DiGraph in Orchestrator) | Scaffolded but never populated. Reserved for future dependency-aware scheduling. |
| Gemini tool call IDs | Gemini`s `generateContent` API does not reliably return unique call IDs. All tool call IDs are hard-coded as `"gemini_tool_call"`. |
