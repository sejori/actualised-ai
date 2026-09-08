# actualised.ai: Human Seeding Flow

**Date:** 08-09-2026
**Topic:** Company Bootstrap, User Onboarding, and Initialization

## 1. The Onboarding Wizard (UI Flow)
The human user begins their journey in the local Tauri app via a three-step "Company Incorporation" wizard.

### Step 1: The Founding Statement
The user provides the core mission of the company. 
*   *Example Input*: "Build a cross-platform SaaS application for scheduling dog walkers, utilizing a Rust backend and React Native frontend."
*   This statement is saved as the foundational context that all agents will eventually inherit or be able to reference.

### Step 2: Org Chart Design (AI-Assisted)
Instead of forcing the user to manually create every node, a dedicated "Co-Founder Agent" (a lightweight LLM prompt within the app) analyzes the Founding Statement and proposes a Tree Structure.
*   The UI displays this as an interactive visual node graph.
*   The user can drag, drop, add, or delete nodes to refine the team.
*   *Example Tree*: Admin (Human) -> Product Lead -> [UI Designer, UX Researcher]. Admin -> Engineering Lead -> [Rust Backend Dev, React Native Dev].

### Step 3: Agent Provisioning
Once the tree is approved, the user reviews the specific profiles for each node. The Co-Founder Agent pre-fills these profiles, which the user can edit:
*   **Name & Avatar**: For the messaging interface.
*   **Role & System Prompt**: The core instructions for the agent's LLM.
*   **Capabilities (Tools)**: Assigned based on role. (e.g., The React Native Dev is given access to the `WebContainerSandbox` tool, while the UX Researcher is given access to the `BrowserbaseEnvironment` for web scraping).

## 2. Technical Initialization (Under the Hood)
When the user clicks "Incorporate Company", the Rust core executes the bootstrap sequence:

### A. Graph Database Initialization
The core connects to the embedded **SurrealDB** and inserts the nodes and relationships.
*   Creates records for each Agent.
*   Creates `MANAGES` relationships to construct the exact tree the user designed.

### B. Filesystem Initialization
The core generates the physical directory structure for the memory system.
```bash
mkdir -p company_root/node_0_admin/memories
mkdir -p company_root/node_1_product_lead/memories
# ... etc
```
It then generates the initial `index.md` for each node, pre-populating them with the Founding Statement and the specific Agent's goals. 

## 3. The Kickoff
With the DB populated and the filesystem created, the company is officially "running," but the agents are dormant (zero inference). 
To start work, the Admin (Human) opens the messaging platform, tags a Team Lead, and creates the first Top-Level Project (e.g., `@Product_Lead Please research our top 3 competitors and spec out the MVP`). This triggers the DAG orchestrator, waking up the Product Lead to begin breaking down the task.
