// @ts-ignore
import { Company } from '../packages/sdk/src/index';

async function main() {
    console.log("Starting Actualised.ai Code-First MVP...");

    // 1. Initialize the Company
    const company = new Company(
        "DogWalker SaaS", 
        "Build a cross-platform SaaS application for scheduling dog walkers.", 
        "./local_state"
    );

    // 2. Define the Team Graph
    company.addAgent("node_1_engineering", "Engineering Lead", "Manage architecture and break down tasks.");
    company.addAgent("node_2_backend", "Backend Dev", "Write Rust APIs.");

    // 3. Define the Initial Project
    company.addProject("proj_1", "V1 Launch", "Design the database schema and build the initial API.");

    // 4. Start the Engine
    console.log("Starting engine... (This will setup memory directories and start DFS loop)");
    await company.start();

    console.log("Done.");
}

main().catch(console.error);
