# actualised.ai: Team Topology & Filesystem Memory

**Date:** 08-09-2026
**Topic:** Tree Structure, Roles, and Filesystem-Based Memory

## 1. The Tree Topology & Roles
The actualised.ai company structure is a strict tree. 
*   **Team Leads (Nodes with children)**: A Team Lead is not heavily involved in manual inference work (coding/writing). Instead, it acts as a central communication hub, a shared memory store, and a router for other agents wanting progress updates on the team.
*   **Worker Agents (Leaf nodes)**: These agents execute the actual tasks, doing the research, coding, and building.

## 2. Filesystem-Based Memory (No Vector DB)
To minimize complexity and adhere to a transparent, UNIX-like philosophy, the system avoids complex Vector Databases in the initial stages. Memory is handled entirely via the filesystem.

Every node in the tree has a dedicated directory.
```text
company_root/
├── node_0_admin/
│   ├── index.md
│   └── memories/
├── node_1_engineering_lead/
│   ├── index.md
│   └── memories/
│       ├── mem_001_architecture_spec.md
│       └── mem_002_tech_stack.md
│   ├── node_2_frontend_dev/
│   │   ├── index.md
│   │   └── memories/
│   └── node_3_backend_dev/
│       ├── index.md
│       └── memories/
```

### The Index File (`index.md`)
Each node has a central `index.md` file. This file acts as a catalog or table of contents for the team's knowledge. It contains brief summaries and relative file paths to detailed documents inside the `memories/` folder.
*   **Retrieval**: Agents use standard tools (like `grep` or reading the file) to search the index. If they find a relevant topic, they follow the file path to read the full memory document.

## 3. Access Control (Reading and Writing)
The file system naturally enforces the access control requirements of the tree structure.

*   **Reading Up (Global Context)**: An agent is granted Read access to its own directory and the directories of all its ancestors, up to the root.
    *   *Example*: `node_3_backend_dev` can `grep` the `index.md` of `node_1_engineering_lead` and `node_0_admin` to understand company-wide goals.
*   **Writing Up (Reporting)**: Children can write directly into their Team Lead's (direct parent's) memory.
    *   *Example*: When `node_3_backend_dev` finishes an API, it creates a new file `mem_003_api_docs.md` inside `node_1_engineering_lead/memories/` and appends a reference to it in `node_1_engineering_lead/index.md`.
    *   *Result*: The Team Lead's memory becomes an aggregated, up-to-date source of truth for the entire team's output, without the Team Lead having to manually synthesize it. Other teams can query the Engineering Lead for updates, and the Lead simply references its organically populated index.
