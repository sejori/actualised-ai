import { createSignal, For, onCleanup, onSettled, Show } from 'solid-js';
import type { Component } from 'solid-js';
import cytoscape from 'cytoscape';
import type { Core, ElementDefinition } from 'cytoscape';
import initWasm, { OrchestratorWasm } from './wasm/actualised_core_wasm.js';
import './App.css';

type Agent = { id: string; name: string; role: string; parent_id: string | null; system_prompt: string; tools: string[] };
type Project = { id: string; title: string; description: string };
type HoverPosition = { x: number; y: number };

function graphElements(agents: Agent[]): ElementDefinition[] {
  const rootId = 'human-root';
  return [
    { data: { id: rootId, label: 'You' }, classes: 'root' },
    ...agents.map((agent) => ({ data: { id: agent.id, label: agent.name }, classes: agent.parent_id ? 'contributor' : 'lead' })),
    ...agents.map((agent) => ({ data: { id: `${agent.parent_id ?? rootId}-${agent.id}`, source: agent.parent_id ?? rootId, target: agent.id } })),
  ];
}

const Dashboard: Component = () => {
  let cyContainer!: HTMLDivElement;
  let cy: Core | undefined;
  const [agents, setAgents] = createSignal<Agent[]>([]);
  const [projects, setProjects] = createSignal<Project[]>([]);
  const [selectedAgent, setSelectedAgent] = createSignal<Agent>();
  const [hoveredAgent, setHoveredAgent] = createSignal<Agent>();
  const [hoverPosition, setHoverPosition] = createSignal<HoverPosition>({ x: 0, y: 0 });
  const [isInspectorOpen, setIsInspectorOpen] = createSignal(false);
  const [error, setError] = createSignal<string>();

  const selectAgent = (agent: Agent, openInspector = true) => {
    setSelectedAgent(agent);
    setIsInspectorOpen(openInspector);
    const node = cy?.$id(agent.id);
    if (node) cy?.animate({ center: { eles: node }, duration: 180 });
  };

  const cycleAgent = (direction: 1 | -1) => {
    const companyAgents = agents();
    const currentIndex = companyAgents.findIndex(({ id }) => id === selectedAgent()?.id);
    const nextIndex = (currentIndex + direction + companyAgents.length) % companyAgents.length;
    selectAgent(companyAgents[nextIndex]);
  };

  onSettled(() => {
    void (async () => {
      try {
        await initWasm();
        const orchestrator = await OrchestratorWasm.init();
        const companyAgents = orchestrator.get_agents() as Agent[];
        setAgents(companyAgents);
        setProjects(orchestrator.get_projects() as Project[]);
        setSelectedAgent(companyAgents[0]);

        cy = cytoscape({
          container: cyContainer,
          elements: graphElements(companyAgents),
          style: [
            { selector: 'node', style: { label: 'data(label)', color: '#3c3830', 'font-size': 11, 'font-weight': 600, 'text-valign': 'bottom', 'text-margin-y': 9, 'background-color': '#9fc7c0', 'border-width': 2, 'border-color': '#547b75', width: 44, height: 44 } },
            { selector: 'node.root', style: { 'background-color': '#f4b8a8', 'border-color': '#a56354', width: 56, height: 56 } },
            { selector: 'node.lead', style: { 'background-color': '#d9c4e9', 'border-color': '#866b9c', width: 50, height: 50 } },
            { selector: 'node.contributor', style: { 'background-color': '#b9d8d1', 'border-color': '#568b80' } },
            { selector: 'edge', style: { width: 1.5, 'line-color': '#9d9789', 'target-arrow-color': '#9d9789', 'target-arrow-shape': 'triangle', 'curve-style': 'bezier' } },
          ],
          layout: { name: 'breadthfirst', directed: true, padding: 110, spacingFactor: 1.25 },
          wheelSensitivity: 0.18,
        });

        cy.on('tap', 'node', (event) => {
          const agent = companyAgents.find(({ id }) => id === event.target.id());
          if (agent) selectAgent(agent);
        });
        cy.on('mouseover', 'node', (event) => {
          const agent = companyAgents.find(({ id }) => id === event.target.id());
          if (agent) {
            setHoveredAgent(agent);
            setHoverPosition(event.renderedPosition);
          }
        });
        cy.on('mouseout', 'node', () => setHoveredAgent());
      } catch (cause) {
        console.error('Failed to initialise the company dashboard', cause);
        setError(cause instanceof Error ? cause.message : 'Unknown initialization error');
      }
    })();
  });

  onCleanup(() => cy?.destroy());

  return <main class="canvas-page">
    <header class="canvas-header">
      <div><p class="eyebrow">Actualised.ai / company canvas</p><h1>Company structure</h1></div>
      <div class="canvas-actions"><span>{agents().length} agents</span><button class="secondary-button" onClick={() => cy?.fit(undefined, 60)}>Centre canvas</button></div>
    </header>
    <Show when={error()}>{(message) => <p class="canvas-error">Could not initialise the company: {message()}</p>}</Show>

    <section class="canvas-shell" aria-label="Navigable company tree">
      <div ref={cyContainer} class="company-canvas" />
      <p class="canvas-hint">Drag to explore · Scroll to zoom · Select an agent to inspect</p>
      <Show when={hoveredAgent()}>{(agent) => <div class="node-tooltip" style={{ left: `${hoverPosition().x}px`, top: `${hoverPosition().y}px` }}><strong>{agent().name}</strong><span>{agent().role}</span></div>}</Show>
    </section>

    <Show when={isInspectorOpen() && selectedAgent()}>
      {(agent) => <aside class="agent-inspector" aria-label={`${agent().name} details`}>
        <div class="inspector-nav">
          <div class="agent-position">{agents().findIndex(({ id }) => id === agent().id) + 1} / {agents().length}</div>
          <div class="nav-buttons"><button class="icon-button" aria-label="Previous agent" onClick={() => cycleAgent(-1)}>←</button><button class="icon-button" aria-label="Next agent" onClick={() => cycleAgent(1)}>→</button><button class="icon-button close-button" aria-label="Close details" onClick={() => setIsInspectorOpen(false)}>×</button></div>
        </div>
        <p class="eyebrow">Agent details</p><h2>{agent().name}</h2><p class="agent-role">{agent().role}</p>
        <div class="inspector-section"><h3>Operating brief</h3><p>{agent().system_prompt}</p></div>
        <div class="inspector-section"><h3>Tools</h3><div class="tool-list"><For each={agent().tools}>{(tool) => <span>{tool}</span>}</For></div></div>
        <Show when={!agent().parent_id}><div class="inspector-section"><h3>Root projects</h3><ul class="project-list"><For each={projects()}>{(project) => <li><strong>{project.title}</strong><span>{project.description}</span></li>}</For></ul></div></Show>
      </aside>}
    </Show>
  </main>;
};

const SetupPage: Component<{ onComplete: () => void }> = (props) => <main class="setup-page">
  <p class="eyebrow">Actualised.ai</p><h1>Build the company that builds the product.</h1>
  <p>Start with the Engineering, Product, and Growth teams from the default company.</p>
  <button class="primary-button" onClick={props.onComplete}>Initialise company</button>
</main>;

const App: Component = () => {
  const [isSetup, setIsSetup] = createSignal(false);
  return <Show when={isSetup()} fallback={<SetupPage onComplete={() => setIsSetup(true)} />}><Dashboard /></Show>;
};

export default App;
