# Actualised.ai — Generic Multi-Agent & Batch Engine Upgrade Plan

This document outlines the architectural upgrades to enhance **Actualised.ai** into a multi-agent and bulk-workload orchestration engine supporting **Doubleword (DeepSeek-V4.1-Flash)**, per-agent model/temperature routing, and native **JSONL Batch Processing**.

The engine remains **100% domain-agnostic**. Domain-specific applications (such as game engines, compiler runners, or web platforms) interact with Actualised.ai as downstream consumers via its SDK and generic tool execution bridges.

---

## 1. System Topology & Decoupled Architecture

```mermaid
flowchart TD
    subgraph actualisedAI (Generic Core Engine)
        ORCH[Orchestrator Engine]
        STATE[(SurrealDB State Graph)]
        ROUTER[Inference Router]
        BATCH[JSONL Batch Job Manager]
        TOOL_EXEC[Generic Tool Executor / NAPI Bridge]
    end

    subgraph Doubleword AI Platform (https://api.doubleword.ai/v1)
        SYNC[/v1/chat/completions (Real-Time Leads)]
        FILES[/v1/files (JSONL Storage)]
        BATCH_API[/v1/batches (24h Bulk Window)]
    end

    subgraph Downstream Consumers (e.g., playable.gg)
        SDK[ActualisedClient SDK]
        CONSUMER[Game Studio / Web App / SaaS]
    end

    ORCH --> STATE
    ORCH --> ROUTER
    ORCH --> BATCH
    ROUTER --> SYNC
    BATCH --> FILES
    BATCH --> BATCH_API
    SDK --> ORCH
    CONSUMER --> SDK
    CONSUMER -.->|Custom HTTP / Script Tools| TOOL_EXEC
```

---

## 2. Key Architectural Upgrades

### 2.1 Doubleword Provider & Per-Agent Model/Temperature Routing
* **Generic OpenAI-Compatible Client:** Add support for configurable `base_url` (defaulting to `https://api.doubleword.ai/v1` for Doubleword).
* **Per-Agent Settings:** Enhance the `Agent` struct in `crates/core/src/state.rs` with:
  * `provider: Option<String>` (e.g., `"doubleword"`, `"gemini"`, `"anthropic"`)
  * `model: Option<String>` (e.g., `"deepseek-ai/DeepSeek-V4.1-Flash"`)
  * `temperature: Option<f32>` (e.g., `0.1` for deterministic reasoning, `0.7` for creative agents)
* **Dynamic Resolution:** During `InferenceQueue::process_batch()`, each request resolves the appropriate endpoint, model, and sampling temperature for that specific agent.

### 2.2 Native JSONL Batch Submission & Polling Loop
For bulk asynchronous workloads (e.g., generating hundreds of documents, dialogues, or simulation runs):
1. **Tool `submit_batch_job`:**
   * Accepts an array of task items.
   * Compiles them into standard OpenAI-compatible JSONL lines (`{"custom_id": "...", "method": "POST", "url": "/v1/chat/completions", "body": {...}}`).
   * Uploads file to Doubleword `POST /v1/files` (`purpose: "batch"`).
   * Creates batch via `POST /v1/batches` with `completion_window: "24h"`.
   * Returns a `batch_id` to the agent.
2. **Autonomous Background Poller:**
   * Periodically queries `GET /v1/batches/{batch_id}` during orchestrator runs.
   * On `status: "completed"`, automatically downloads the output JSONL file, unpacks results, and injects a notification turn:
     `queue_message(agent_id, "Batch job <batch_id> completed. Output processed.")`.

### 2.3 Decoupled Tool Execution (HTTP & NAPI)
* Actualised.ai remains clean and decoupled: tools like external syntax linters, remote test runners, or compilation harnesses are executed via the existing `ToolExecutor` bridge or generic HTTP POST tools registered by the downstream consumer.

---

## 3. Phased Implementation Roadmap

### Phase 1: Doubleword Provider & Per-Agent Model Configuration
* [ ] **Inference Layer (`crates/core/src/inference.rs`)**:
  * Add `base_url` to `InferenceConfig`.
  * Support `Doubleword` (OpenAI-compatible) and pass `temperature` down to the completions request payload.
* [ ] **State Schema Upgrade (`crates/core/src/state.rs`)**:
  * Add `model`, `provider`, and `temperature` fields to `Agent`.
  * Ensure SurrealDB migrations / permissions preserve these fields.
* [ ] **SDK & NAPI Bindings (`packages/sdk`)**:
  * Expose `model`, `provider`, and `temperature` in the TypeScript `Agent` interface.

### Phase 2: Doubleword JSONL Batch Engine
* [ ] **Batch Module (`crates/core/src/batch.rs`)**:
  * JSONL serializer for batch completions.
  * File upload (`/v1/files`) and batch creation (`/v1/batches`).
  * Batch status polling and result retrieval.
* [ ] **Batch Agent Tool**:
  * Expose `submit_batch_job` and `check_batch_status` to agents.
* [ ] **Completion Poller & Turn Injection**:
  * Hook a periodic batch check into `orchestrator.run()` or `CompanyRunner`.
  * Deliver completions as conversational turns to wake up the requesting agent.

### Phase 3: Verification & Integration Testing
* [ ] **Unit & Integration Tests**:
  * Verify per-agent temperature and model routing.
  * Verify JSONL creation, batch submission, and polling with Doubleword mock or live key.
  * Verify downstream consumption via `packages/sdk/examples/`.
