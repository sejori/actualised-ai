# actualised.ai: Code-First TypeScript SDK MVP

**Date:** 08-09-2026
**Topic:** Implementation Strategy: Programmatic Base and State Persistence

## 1. The Pivot: Code-First over UI-First
To minimize moving parts and validate the core orchestration engine (the DAG, the memory access, and agent loops) as quickly as possible, the initial implementation will forego the visual UI and the complex browser-based WebContainer adapters. 

Instead, the MVP will be entirely programmatic, driven by a **TypeScript SDK**. 

## 2. Defining the Company in Code
The user will act as the "Admin" by writing a TypeScript script to define the team structure, assign roles, and seed the initial projects. This serves as the precise, deterministic input for the orchestrator.

### Example SDK Usage:
```typescript
import { Company, Agent, Project } from '@actualised-ai/core';

// 1. Initialize the Company with a local state directory
const company = new Company({
    name: "DogWalker SaaS",
    mission: "Build a cross-platform SaaS application for scheduling dog walkers.",
    stateDirectory: "./my_company_state" // Persists SurrealDB and Filesystem Memory
});

// 2. Define the Team Graph
const engLead = new Agent({ name: "Engineering Lead", role: "Manage architecture and break down tasks." });
const backendDev = new Agent({ name: "Backend Dev", role: "Write Rust APIs." });

company.addNode(engLead, { parent: "root" });
company.addNode(backendDev, { parent: engLead.id });

// 3. Define the Initial Project
const mvpProject = new Project({
    title: "V1 Launch",
    description: "Design the database schema and build the initial API."
});
company.assignProject(mvpProject, engLead.id);

// 4. Start the Engine
// This boots the DAG executor, checks for unblocked tasks, and spawns local subprocess sandboxes.
await company.start();
```

## 3. State Persistence (The Local Directory)
By passing a `stateDirectory` into the SDK, the orchestrator has a persistent physical location to store the company. 
*   **Database**: The embedded SurrealDB file (or a lightweight SQLite file for the MVP) is saved here, persisting the graph, task states, and assignments.
*   **Memory**: The `node_X/memories/` directory tree is generated inside this folder.
*   **Resilience**: Because all state is written to this directory, the Node.js/Rust process can be terminated at any time (e.g., `Ctrl+C`). When the user runs the script again, the SDK reads the directory, re-hydrates the DAG, and resumes the exact state where it left off.

## 4. Roadmap to the UI
By building this programmatic foundation first:
1.  We prove the local subprocess execution adapter works.
2.  We prove the filesystem memory architecture works.
3.  We prove the DAG orchestration loops correctly.

Once the CLI/SDK can successfully compile a "Hello World" application autonomously, the foundation is proven. At that point, the Tauri visual UI and the Browser Sandbox adapters can be built on top of this exact same core logic.
