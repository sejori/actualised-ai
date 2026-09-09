import { ActualisedClient } from '../src/index';

async function bootstrap() {
  const client = new ActualisedClient('surreal-cloud-url');

  // Set pacing to avoid inference limits, e.g., 5 requests per minute, 9-5 working hours
  await client.setPacing({
    requestsPerMinute: 5.0,
    workingHours: {
      start: '09:00',
      end: '17:00'
    }
  });

  // Define the Self-Developing Team
  
  // 1. Product Manager
  await client.addAgent({
    id: 'pm',
    name: 'Product Manager',
    role: 'Product Lead',
    system_prompt: 'You are the Product Manager. Research UX patterns and hot products, talk to the user, and maintain the product roadmap.',
    tools: ['web_search', 'read_browser', 'update_roadmap', 'telegram_notify', 'chat_with_user']
  });

  // 2. Engineering Manager
  await client.addAgent({
    id: 'em',
    name: 'Engineering Manager',
    role: 'Engineering Lead',
    parent_id: 'pm',
    system_prompt: 'You are the Engineering Manager. Distill projects from the roadmap into technical tasks. Speak with the user about tech strategy. Review codebase.',
    tools: ['read_roadmap', 'chat_with_user', 'github_review', 'codebase_analysis']
  });

  // 3. Core Rust Maintainer
  await client.addAgent({
    id: 'rust_ic',
    name: 'Core Rust Maintainer',
    role: 'Rust Developer',
    parent_id: 'em',
    system_prompt: 'You are the Core Rust Maintainer. Develop and test the Rust crates.',
    tools: ['cargo_compile', 'terminal_execution', 'fs_access', 'github_commit', 'github_pr']
  });

  // 4. Web UX Developer
  await client.addAgent({
    id: 'web_ic',
    name: 'Web UX Developer',
    role: 'React/TypeScript Developer',
    parent_id: 'em',
    system_prompt: 'You are the Web UX Developer. Build the React frontend in packages/web-ui.',
    tools: ['npm_run', 'terminal_execution', 'fs_access', 'github_commit', 'github_pr']
  });

  // 5. TS and Python SDK Engineer
  await client.addAgent({
    id: 'sdk_ic',
    name: 'SDK Engineer',
    role: 'TypeScript & Python Developer',
    parent_id: 'em',
    system_prompt: 'You are the SDK Engineer. Develop the TypeScript and Python SDKs.',
    tools: ['tsc_compile', 'python_run', 'terminal_execution', 'fs_access', 'github_commit', 'github_pr']
  });

  console.log("Self-developing company state initialized!");
}

bootstrap().catch(console.error);
