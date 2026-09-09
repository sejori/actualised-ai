import { createEffect, createSignal, For, onCleanup, onSettled, Show } from 'solid-js';
import type { Component } from 'solid-js';
import cytoscape from 'cytoscape';
import type { Core, ElementDefinition } from 'cytoscape';
import initWasm, { OrchestratorWasm } from './wasm/actualised_core_wasm.js';
import './App.css';

type Agent = { id: string; name: string; role: string; parent_id: string | null; system_prompt: string; tools: string[] };
type Project = { id: string; title: string; description: string };
type HoverPosition = { x: number; y: number };
type InferenceSettings = { provider: string; model: string; serviceTier: string; apiKey: string };
type RateLimitSettings = { maxConcurrentRequests: number; requestsPerSecond: number };
type ConversationTurn = { role: 'operator' | 'agent'; content: string };
type AgentContext = { pending_messages: string[]; history: ConversationTurn[] };
type MemoryNode =
  | { kind: 'folder'; name: string; children: MemoryNode[] }
  | { kind: 'file'; name: string; path: string; content: string };
type ViewedFile = { path: string; content: string };

const INFERENCE_PROVIDERS = [
  { id: 'gemini', label: 'Google Gemini', models: ['gemini-3.6-flash', 'gemini-3.6-pro'], disabled: false },
  { id: 'openai', label: 'OpenAI (coming soon)', models: ['gpt-5', 'gpt-5-mini'], disabled: true },
  { id: 'anthropic', label: 'Anthropic (coming soon)', models: ['claude-4.5-sonnet'], disabled: true },
] as const;
const SERVICE_TIERS = ['default', 'flex', 'priority'];
const SETTINGS_STORAGE_KEY = 'actualised.inference-settings';
const RATE_LIMIT_STORAGE_KEY = 'actualised.rate-limits';

function loadStoredSettings(): InferenceSettings {
  const fallback: InferenceSettings = { provider: 'gemini', model: INFERENCE_PROVIDERS[0].models[0], serviceTier: 'default', apiKey: '' };
  try {
    const raw = localStorage.getItem(SETTINGS_STORAGE_KEY);
    return raw ? { ...fallback, ...JSON.parse(raw) } : fallback;
  } catch {
    return fallback;
  }
}

function loadStoredRateLimits(): RateLimitSettings {
  const fallback: RateLimitSettings = { maxConcurrentRequests: 5, requestsPerSecond: 0 };
  try {
    const raw = localStorage.getItem(RATE_LIMIT_STORAGE_KEY);
    return raw ? { ...fallback, ...JSON.parse(raw) } : fallback;
  } catch {
    return fallback;
  }
}

function graphElements(agents: Agent[]): ElementDefinition[] {
  const rootId = 'human-root';
  return [
    { data: { id: rootId, label: 'You' }, classes: 'root' },
    ...agents.map((agent) => ({ data: { id: agent.id, label: agent.name }, classes: agent.parent_id ? 'contributor' : 'lead' })),
    ...agents.map((agent) => ({ data: { id: `${agent.parent_id ?? rootId}-${agent.id}`, source: agent.parent_id ?? rootId, target: agent.id } })),
  ];
}

/// Recursive folder/file tree renderer for agent memories and the shared team directory,
/// using native <details>/<summary> so expand/collapse and keyboard navigation come for free.
const MemoryTreeView: Component<{ nodes: MemoryNode[]; onOpenFile: (file: ViewedFile) => void }> = (props) => (
  <ul class="memory-tree">
    <For each={props.nodes}>
      {(node) => <li>
        {node.kind === 'folder'
          ? <details open>
              <summary>{node.name}</summary>
              <MemoryTreeView nodes={node.children} onOpenFile={props.onOpenFile} />
            </details>
          : <button type="button" class="memory-leaf" onClick={() => props.onOpenFile({ path: node.path, content: node.content })}>{node.name}</button>}
      </li>}
    </For>
  </ul>
);

const Dashboard: Component = () => {
  let cyContainer!: HTMLDivElement;
  let cy: Core | undefined;
  let orchestrator: OrchestratorWasm | undefined;
  // Every call into `orchestrator` is funnelled through this chain so none ever overlap:
  // wasm-bindgen panics ("recursive use of an object") if a method is invoked while another
  // call on the same instance is still in flight (e.g. mid-await inside run_orchestrator()).
  let wasmQueue: Promise<unknown> = Promise.resolve();
  const callOrchestrator = <T,>(fn: (o: OrchestratorWasm) => T | Promise<T>): Promise<T> => {
    const task = wasmQueue.then(() => {
      if (!orchestrator) throw new Error('Orchestrator not ready');
      return fn(orchestrator);
    });
    wasmQueue = task.then(
      () => undefined,
      () => undefined,
    );
    return task;
  };
  let inspectorRef: HTMLElement | undefined;
  let settingsRef: HTMLElement | undefined;
  let sharedRef: HTMLElement | undefined;
  let chatScrollRef: HTMLDivElement | undefined;
  let lastFocusedNodeButton: HTMLElement | undefined;
  const [agents, setAgents] = createSignal<Agent[]>([]);
  const [projects, setProjects] = createSignal<Project[]>([]);
  const [selectedAgent, setSelectedAgent] = createSignal<Agent>();
  const [hoveredAgent, setHoveredAgent] = createSignal<Agent>();
  const [hoverPosition, setHoverPosition] = createSignal<HoverPosition>({ x: 0, y: 0 });
  const [isInspectorOpen, setIsInspectorOpen] = createSignal(false);
  const [isSettingsOpen, setIsSettingsOpen] = createSignal(false);
  const [isSharedOpen, setIsSharedOpen] = createSignal(false);
  const [inferenceSettings, setInferenceSettings] = createSignal<InferenceSettings>(loadStoredSettings());
  const [draftSettings, setDraftSettings] = createSignal<InferenceSettings>(inferenceSettings());
  const [rateLimitSettings, setRateLimitSettings] = createSignal<RateLimitSettings>(loadStoredRateLimits());
  const [draftRateLimit, setDraftRateLimit] = createSignal<RateLimitSettings>(rateLimitSettings());
  const [isOrchestratorRunning, setIsOrchestratorRunning] = createSignal(false);
  const [agentMemoryTree, setAgentMemoryTree] = createSignal<MemoryNode[]>([]);
  const [sharedTree, setSharedTree] = createSignal<MemoryNode[]>([]);
  const [viewedMemory, setViewedMemory] = createSignal<ViewedFile>();
  const [agentContext, setAgentContext] = createSignal<AgentContext>();
  const [messageDraft, setMessageDraft] = createSignal('');
  const [error, setError] = createSignal<string>();

  const applyInferenceSettings = async (settings: InferenceSettings) => {
    try {
      await callOrchestrator((o) => o.configure_inference({ provider: settings.provider, model: settings.model, service_tier: settings.serviceTier || null, api_key: settings.apiKey }));
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : 'Failed to apply inference settings');
    }
  };

  const applyRateLimits = async (settings: RateLimitSettings) => {
    try {
      await callOrchestrator((o) => o.configure_rate_limits({ max_concurrent_requests: settings.maxConcurrentRequests, requests_per_second: settings.requestsPerSecond }));
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : 'Failed to apply rate limits');
    }
  };

  const refreshAgentDetails = async (agentId: string) => {
    if (!orchestrator) return;
    try {
      const [tree, context] = await Promise.all([
        callOrchestrator((o) => o.get_agent_memory_tree(agentId) as MemoryNode[]),
        callOrchestrator((o) => o.get_agent_context(agentId) as AgentContext),
      ]);
      setAgentMemoryTree(tree);
      setAgentContext(context);
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : 'Failed to load agent details');
    }
  };

  const openSharedDirectory = async () => {
    setIsInspectorOpen(false);
    setIsSettingsOpen(false);
    setViewedMemory(undefined);
    setIsSharedOpen(true);
    if (!orchestrator) return;
    try {
      setSharedTree(await callOrchestrator((o) => o.get_shared_tree() as MemoryNode[]));
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : 'Failed to load the shared directory');
    }
  };

  const sendAgentMessage = async (agentId: string) => {
    const message = messageDraft().trim();
    if (!message || !orchestrator) return;
    setMessageDraft('');
    await callOrchestrator((o) => o.send_agent_message(agentId, message));
    await refreshAgentDetails(agentId);
  };

  const openSettings = () => {
    setIsInspectorOpen(false);
    setIsSharedOpen(false);
    setDraftSettings(inferenceSettings());
    setDraftRateLimit(rateLimitSettings());
    setIsSettingsOpen(true);
  };

  const saveSettings = (event: SubmitEvent) => {
    event.preventDefault();
    const settings = draftSettings();
    const rateLimits = draftRateLimit();
    setInferenceSettings(settings);
    setRateLimitSettings(rateLimits);
    localStorage.setItem(SETTINGS_STORAGE_KEY, JSON.stringify(settings));
    localStorage.setItem(RATE_LIMIT_STORAGE_KEY, JSON.stringify(rateLimits));
    void applyInferenceSettings(settings);
    void applyRateLimits(rateLimits);
    setIsSettingsOpen(false);
  };

  const runOrchestratorCycle = async () => {
    if (!orchestrator || isOrchestratorRunning()) return;
    setIsOrchestratorRunning(true);
    try {
      await callOrchestrator((o) => o.run_orchestrator());
      const agent = selectedAgent();
      if (agent) await refreshAgentDetails(agent.id);
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : 'Orchestrator run failed');
    } finally {
      setIsOrchestratorRunning(false);
    }
  };

  const selectAgent = (agent: Agent, openInspector = true) => {
    setIsSettingsOpen(false);
    setIsSharedOpen(false);
    setSelectedAgent(agent);
    setIsInspectorOpen(openInspector);
    setMessageDraft('');
    setViewedMemory(undefined);
    void refreshAgentDetails(agent.id);
    cy?.nodes('.selected').removeClass('selected');
    const node = cy?.$id(agent.id);
    if (node) {
      node.addClass('selected');
      cy?.animate({ center: { eles: node }, duration: 180 });
    }
  };

  const cycleAgent = (direction: 1 | -1) => {
    const companyAgents = agents();
    const currentIndex = companyAgents.findIndex(({ id }) => id === selectedAgent()?.id);
    const nextIndex = (currentIndex + direction + companyAgents.length) % companyAgents.length;
    selectAgent(companyAgents[nextIndex]);
  };

  const handleKeydown = (event: KeyboardEvent) => {
    if (event.key !== 'Escape') return;
    if (viewedMemory()) setViewedMemory(undefined);
    else if (isSharedOpen()) setIsSharedOpen(false);
    else if (isSettingsOpen()) setIsSettingsOpen(false);
    else if (isInspectorOpen()) setIsInspectorOpen(false);
  };
  window.addEventListener('keydown', handleKeydown);
  onCleanup(() => window.removeEventListener('keydown', handleKeydown));

  const trapTabWithin = (container: HTMLElement | undefined) => (event: KeyboardEvent) => {
    if (event.key !== 'Tab' || !container) return;
    const focusable = container.querySelectorAll<HTMLElement>('button, [href], input, select, textarea, [tabindex]:not([tabindex="-1"])');
    if (focusable.length === 0) return;
    const first = focusable[0];
    const last = focusable[focusable.length - 1];
    if (event.shiftKey && document.activeElement === first) {
      event.preventDefault();
      last.focus();
    } else if (!event.shiftKey && document.activeElement === last) {
      event.preventDefault();
      first.focus();
    }
  };
  const trapInspectorTab = (event: KeyboardEvent) => trapTabWithin(inspectorRef)(event);
  const trapSettingsTab = (event: KeyboardEvent) => trapTabWithin(settingsRef)(event);
  const trapSharedTab = (event: KeyboardEvent) => trapTabWithin(sharedRef)(event);

  createEffect(
    () => isInspectorOpen(),
    (open) => {
      if (open) {
        inspectorRef?.querySelector<HTMLElement>('.close-button')?.focus();
      } else {
        lastFocusedNodeButton?.focus();
      }
    },
  );

  createEffect(
    () => isSettingsOpen(),
    (open) => {
      if (open) {
        settingsRef?.querySelector<HTMLElement>('select, input')?.focus();
      } else {
        lastFocusedNodeButton?.focus();
      }
    },
  );

  createEffect(
    () => isSharedOpen(),
    (open) => {
      if (open) {
        sharedRef?.querySelector<HTMLElement>('.memory-leaf, summary, .close-button')?.focus();
      } else {
        lastFocusedNodeButton?.focus();
      }
    },
  );

  createEffect(
    () => viewedMemory(),
    (file) => {
      if (!file) {
        inspectorRef?.querySelector<HTMLElement>('.memory-leaf, summary')?.focus();
      }
    },
  );

  createEffect(
    () => agentContext()?.history.length,
    () => {
      if (chatScrollRef) chatScrollRef.scrollTop = chatScrollRef.scrollHeight;
    },
  );

  onSettled(() => {
    void (async () => {
      try {
        await initWasm();
        orchestrator = await OrchestratorWasm.init();
        await applyInferenceSettings(inferenceSettings());
        await applyRateLimits(rateLimitSettings());
        const companyAgents = orchestrator.get_agents() as Agent[];
        setAgents(companyAgents);
        setProjects(orchestrator.get_projects() as Project[]);
        setSelectedAgent(companyAgents[0]);
        if (companyAgents[0]) await refreshAgentDetails(companyAgents[0].id);

        cy = cytoscape({
          container: cyContainer,
          elements: graphElements(companyAgents),
          style: [
            { selector: 'node', style: { label: 'data(label)', color: '#3c3830', 'font-size': 11, 'font-weight': 600, 'text-valign': 'bottom', 'text-margin-y': 9, 'background-color': '#9fc7c0', 'border-width': 2, 'border-color': '#547b75', width: 44, height: 44 } },
            { selector: 'node.root', style: { 'background-color': '#f4b8a8', 'border-color': '#a56354', width: 56, height: 56 } },
            { selector: 'node.lead', style: { 'background-color': '#d9c4e9', 'border-color': '#866b9c', width: 50, height: 50 } },
            { selector: 'node.contributor', style: { 'background-color': '#b9d8d1', 'border-color': '#568b80' } },
            { selector: 'node.selected', style: { 'border-width': 4, 'border-color': '#c25b3f', 'overlay-opacity': 0 } },
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
        cy.$id(companyAgents[0]?.id).addClass('selected');
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
      <div class="canvas-actions">
        <span>{agents().length} agents</span>
        <button
          type="button"
          class="icon-button play-button"
          classList={{ 'is-running': isOrchestratorRunning() }}
          aria-label={isOrchestratorRunning() ? 'Orchestrator cycle running' : 'Run orchestrator cycle'}
          disabled={isOrchestratorRunning()}
          onClick={runOrchestratorCycle}
        >{isOrchestratorRunning() ? '⏸' : '▶'}</button>
        <button type="button" class="icon-button" aria-label="Shared team directory" onClick={openSharedDirectory}>🗂</button>
        <button type="button" class="icon-button" aria-label="Inference settings" onClick={openSettings}>⚙</button>
        <button class="secondary-button" onClick={() => cy?.fit(undefined, 60)}>Centre canvas</button>
      </div>
    </header>
    <Show when={error()}>{(message) => <p class="canvas-error">Could not initialise the company: {message()}</p>}</Show>

    <section class="canvas-shell" aria-label="Navigable company tree">
      <div ref={cyContainer} class="company-canvas" />
      <p class="canvas-hint">Drag to explore · Scroll to zoom · Tab to an agent, Enter to inspect</p>
      <Show when={hoveredAgent()}>{(agent) => <div class="node-tooltip" style={{ left: `${hoverPosition().x}px`, top: `${hoverPosition().y}px` }}><strong>{agent().name}</strong><span>{agent().role}</span></div>}</Show>
      <div class="graph-focus-nodes">
        <For each={agents()}>
          {(agent) => <button
            type="button"
            class="graph-node-button"
            aria-current={selectedAgent()?.id === agent.id}
            onFocus={(event) => { selectAgent(agent, false); lastFocusedNodeButton = event.currentTarget; }}
            onClick={(event) => { selectAgent(agent, true); lastFocusedNodeButton = event.currentTarget; }}
          >{agent.name} — {agent.role}</button>}
        </For>
      </div>
    </section>

    <Show when={isInspectorOpen() && selectedAgent()}>
      {(agent) => <aside ref={inspectorRef} class="agent-inspector" aria-label={`${agent().name} details`} onKeyDown={trapInspectorTab}>
        <div class="inspector-nav">
          <div class="agent-position">{agents().findIndex(({ id }) => id === agent().id) + 1} / {agents().length}</div>
          <div class="nav-buttons"><button class="icon-button" aria-label="Previous agent" onClick={() => cycleAgent(-1)}>←</button><button class="icon-button" aria-label="Next agent" onClick={() => cycleAgent(1)}>→</button><button class="icon-button close-button" aria-label="Close details" onClick={() => setIsInspectorOpen(false)}>×</button></div>
        </div>
        <p class="eyebrow">Agent details</p><h2>{agent().name}</h2><p class="agent-role">{agent().role}</p>
        <div class="inspector-section"><h3>Operating brief</h3><p>{agent().system_prompt}</p></div>
        <div class="inspector-section"><h3>Tools</h3><div class="tool-list"><For each={agent().tools}>{(tool) => <span>{tool}</span>}</For></div></div>
        <Show when={!agent().parent_id}><div class="inspector-section"><h3>Root projects</h3><ul class="project-list"><For each={projects()}>{(project) => <li><strong>{project.title}</strong><span>{project.description}</span></li>}</For></ul></div></Show>

        <Show
          when={viewedMemory()}
          fallback={<>
            <div class="inspector-section">
              <h3>Memories</h3>
              <Show when={agentMemoryTree().length > 0} fallback={<p class="empty-hint">No memory files written yet.</p>}>
                <MemoryTreeView nodes={agentMemoryTree()} onOpenFile={setViewedMemory} />
              </Show>
            </div>

            <div class="inspector-section">
              <h3>Inference context</h3>
              <Show when={(agentContext()?.history.length ?? 0) > 0} fallback={<p class="empty-hint">No turns run yet.</p>}>
                <div class="chat-scroll" ref={chatScrollRef}>
                  <For each={agentContext()?.history}>
                    {(turn) => <div class={`chat-bubble chat-${turn.role}`}><span class="chat-role">{turn.role === 'operator' ? 'Operator' : agent().name}</span><p>{turn.content}</p></div>}
                  </For>
                </div>
              </Show>
              <Show when={(agentContext()?.pending_messages.length ?? 0) > 0}>
                <p class="context-label">Queued for next turn</p>
                <ul class="pending-list"><For each={agentContext()?.pending_messages}>{(message) => <li>{message}</li>}</For></ul>
              </Show>
              <form
                class="message-form"
                onSubmit={(event) => { event.preventDefault(); sendAgentMessage(agent().id); }}
              >
                <label for="agent-message">Send a message into this agent's next turn</label>
                <textarea
                  id="agent-message"
                  rows="3"
                  placeholder="e.g. Prioritise the onboarding bug before anything else"
                  value={messageDraft()}
                  onInput={(event) => setMessageDraft(event.currentTarget.value)}
                />
                <button type="submit" class="secondary-button" disabled={!messageDraft().trim()}>Queue message</button>
              </form>
            </div>
          </>}
        >
          {(file) => <div class="inspector-section memory-viewer">
            <button type="button" class="secondary-button back-button" onClick={() => setViewedMemory(undefined)}>← Back to {agent().name}</button>
            <h3>{file().path}</h3>
            <pre class="memory-viewer-content">{file().content}</pre>
          </div>}
        </Show>
      </aside>}
    </Show>

    <Show when={isSharedOpen()}>
      <aside ref={sharedRef} class="settings-popover shared-popover" aria-label="Shared team directory" onKeyDown={trapSharedTab}>
        <div class="inspector-nav">
          <p class="eyebrow">Shared team directory</p>
          <button class="icon-button close-button" aria-label="Close shared directory" onClick={() => { setIsSharedOpen(false); setViewedMemory(undefined); }}>×</button>
        </div>
        <Show when={sharedTree().length > 0} fallback={<p class="empty-hint">No shared files written yet.</p>}>
          <MemoryTreeView nodes={sharedTree()} onOpenFile={(file) => setViewedMemory(file)} />
        </Show>
        <Show when={viewedMemory()}>
          {(file) => <div class="memory-viewer">
            <button type="button" class="secondary-button back-button" onClick={() => setViewedMemory(undefined)}>← Back to file list</button>
            <h3>{file().path}</h3>
            <pre class="memory-viewer-content">{file().content}</pre>
          </div>}
        </Show>
      </aside>
    </Show>

    <Show when={isSettingsOpen()}>
      <aside ref={settingsRef} class="settings-popover" aria-label="Inference settings" onKeyDown={trapSettingsTab}>
        <div class="inspector-nav">
          <p class="eyebrow">Inference settings</p>
          <button class="icon-button close-button" aria-label="Close settings" onClick={() => setIsSettingsOpen(false)}>×</button>
        </div>
        <form class="settings-form" onSubmit={saveSettings}>
          <label>Provider
            <select
              value={draftSettings().provider}
              onChange={(event) => {
                const provider = event.currentTarget.value;
                const firstModel = INFERENCE_PROVIDERS.find((p) => p.id === provider)?.models[0] ?? '';
                setDraftSettings((prev) => ({ ...prev, provider, model: firstModel }));
              }}
            >
              <For each={INFERENCE_PROVIDERS}>{(provider) => <option value={provider.id} disabled={provider.disabled}>{provider.label}</option>}</For>
            </select>
          </label>
          <label>Model
            <select
              value={draftSettings().model}
              onChange={(event) => setDraftSettings((prev) => ({ ...prev, model: event.currentTarget.value }))}
            >
              <For each={INFERENCE_PROVIDERS.find((p) => p.id === draftSettings().provider)?.models ?? []}>{(model) => <option value={model}>{model}</option>}</For>
            </select>
          </label>
          <label>Service tier
            <select
              value={draftSettings().serviceTier}
              onChange={(event) => setDraftSettings((prev) => ({ ...prev, serviceTier: event.currentTarget.value }))}
            >
              <For each={SERVICE_TIERS}>{(tier) => <option value={tier}>{tier}</option>}</For>
            </select>
          </label>
          <label>API key
            <input
              type="password"
              autocomplete="off"
              placeholder="Paste your provider API key"
              value={draftSettings().apiKey}
              onInput={(event) => setDraftSettings((prev) => ({ ...prev, apiKey: event.currentTarget.value }))}
            />
          </label>
          <fieldset class="rate-limit-fieldset">
            <legend>Rate limits</legend>
            <label>Max concurrent requests
              <input
                type="number"
                min="1"
                step="1"
                value={draftRateLimit().maxConcurrentRequests}
                onInput={(event) => setDraftRateLimit((prev) => ({ ...prev, maxConcurrentRequests: Number(event.currentTarget.value) || 1 }))}
              />
            </label>
            <label>Requests per second (0 = unlimited)
              <input
                type="number"
                min="0"
                step="0.1"
                value={draftRateLimit().requestsPerSecond}
                onInput={(event) => setDraftRateLimit((prev) => ({ ...prev, requestsPerSecond: Number(event.currentTarget.value) || 0 }))}
              />
            </label>
          </fieldset>
          <p class="settings-hint">Stored only in this browser. Google Gemini is the only provider wired up right now — the rest are placeholders.</p>
          <button type="submit" class="primary-button">Save &amp; apply</button>
        </form>
      </aside>
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
