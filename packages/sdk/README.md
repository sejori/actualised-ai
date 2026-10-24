# @actualised-ai/sdk

Official Node.js SDK and TypeScript client for Actualised AI — an autonomous multi-agent orchestration runtime powered by Rust and SurrealDB.

## Installation

```bash
npm install @actualised-ai/sdk
# or
pnpm add @actualised-ai/sdk
```

Supported platforms:
- Linux x64 (glibc 2.31+)
- Windows x64

## Quick Start

```typescript
import { ActualisedClient } from '@actualised-ai/sdk';

async function main() {
  // Initialize company with in-memory or persisted SurrealDB
  const client = await ActualisedClient.create('mem://');

  // Found company (seeds default leadership structure)
  await client.foundCompany('Acme AI Studio');

  // Configure inference provider (Doubleword, Gemini, OpenAI, etc.)
  await client.configureInference({
    provider: 'doubleword',
    model: 'deepseek-ai/DeepSeek-V4.1-Flash',
    apiKey: process.env.DOUBLEWORD_API_KEY,
  });

  // Register asynchronous or synchronous tools
  client.registerToolExecutor(async (agentId, toolName, argsJson) => {
    const args = JSON.parse(argsJson);
    console.log(`Agent ${agentId} called ${toolName} with args:`, args);

    if (toolName === 'execute_task') {
      const result = await doAsyncWork(args);
      return JSON.stringify(result);
    }
    return JSON.stringify({ status: 'ok' });
  });

  // Queue a message for an agent
  await client.queueMessage('agent_1', 'Plan the project roadmap.');

  // Run the orchestration turn
  await client.start();
}

main().catch(console.error);
```

## Features

- **Blazing Fast Native Core:** Powered by Tokio and compiled Rust native bindings (`napi-rs`).
- **Async Tool Execution:** Full support for asynchronous tool handlers returning promises (`string | Promise<string>`).
- **SurrealDB Storage:** Seamless persistence with in-memory (`mem://`), local file (`surrealkv://`), or remote SurrealDB instances.
- **Provider Routing:** Route per agent to Doubleword, Google Gemini, OpenAI, or Anthropic models.
- **JSONL Batch Engine:** Asynchronous batch generation and turn injection.

## License

MIT
