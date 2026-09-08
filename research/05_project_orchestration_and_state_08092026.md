# actualised.ai: Project Orchestration & State

**Date:** 08-09-2026
**Topic:** Task dependencies, DAG execution, and Embedded Graph Databases

## 1. The Challenge
Team Leads act as Project Managers, breaking down top-level projects into sub-projects and task dependencies. Agents are only invoked when their assigned tasks are unblocked. This requires robust state management (who is doing what) and workflow orchestration (is this task ready?).

## 2. State Management: SurrealDB
Building a robust relational tree from scratch using JSON files or low-level key-value stores (`sled`) is error-prone. The ideal open-source Rust tool for this is **SurrealDB**.

*   **Embedded & Lightweight**: It can run completely embedded within the Rust core (no separate server needed), perfectly fitting the local app model.
*   **Native Graph Capabilities**: It natively understands nodes and edges. 
    *   `agent:backend_lead -> MANAGES -> agent:db_engineer`
    *   `task:api_spec -> BLOCKS -> task:db_schema`
    *   `agent:db_engineer -> ASSIGNED_TO -> task:db_schema`
*   **Benefit**: The human user and the orchestrator can run a simple SurrealQL query to instantly see the entire state of the company, which projects are stalled, and who is assigned to what.

## 3. Workflow Orchestration: The DAG Engine
The requirement for tasks blocking other tasks (Engineering waiting on Design) forms a Directed Acyclic Graph (DAG). We do not need to write a DFS orchestrator from scratch.

*   **The Logic Foundation (`petgraph`)**: The standard Rust graph library. When a Team Lead agent generates a project plan, the core uses `petgraph` to validate that the agent didn't create a circular dependency (e.g., A waits on B, B waits on A).
*   **The Execution Loop (`dag-executor` or `dagrs`)**: These are production-ready Rust crates designed exactly for this. 
    *   Instead of writing a custom DFS polling loop, you feed the tasks into the DAG executor. 
    *   The engine inherently knows when a dependency is satisfied.
    *   When a task becomes "Ready", the engine triggers an async event that wakes up the assigned Agent, providing it with the precise context it needs to execute.

## 4. The Agent Loop
With these tools, the inference loop becomes highly efficient:
1.  **Top-Level**: Human creates `project:Launch_V1`.
2.  **Breakdown**: The Lead Agent is woken up, reads the project, and writes sub-tasks into SurrealDB. It defines the DAG dependencies (Design -> Frontend).
3.  **Idle**: Agents sleep. Zero inference cost.
4.  **Trigger**: The DAG engine sees `task:Design` is ready. It wakes up the Designer Agent.
5.  **Completion**: Designer Agent finishes, writes output to filesystem memory, and marks the task Complete in SurrealDB.
6.  **Cascade**: The DAG engine immediately unlocks `task:Frontend` and wakes up the Frontend Agent.
