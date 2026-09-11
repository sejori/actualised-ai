import { createEffect, createSignal, For, onCleanup, onSettled, Show } from 'solid-js';
import type { Component } from 'solid-js';
import cytoscape from 'cytoscape';
import type { Core, ElementDefinition } from 'cytoscape';
import type { OrchestratorWasm } from './wasm/actualised_core_wasm.js';
import { RemoteOrchestrator } from './remote-orchestrator';
import { createOrchestrator, foundCompany, inspectCompany, signin, signup, getCompanies, deleteCompany, setActiveCompanyId } from './company-runtime';
import './App.css';

type ScheduledTask = { id: string; description: string; due_date: string; completed: boolean };
type Agent = { id: string; name: string; role: string; parent_id: string | null; system_prompt: string; tools: string[]; scheduled_tasks?: ScheduledTask[] };
type Project = { id: string; title: string; description: string };
type Tool = { name: string; description: string; parameters: any };
type SharedFile = { id: string; name: string; content: string };
type HoverPosition = { x: number; y: number };
type InferenceSettings = { provider: string; model: string; serviceTier: string; apiKey: string };
type RateLimitSettings = { maxConcurrentRequests: number; requestsPerMinute: number };
type ConversationTurn = { role: 'operator' | 'agent'; content: string };
type AgentContext = { pending_messages?: string[]; history?: ConversationTurn[] };
type MemoryNode =
  | { kind: 'folder'; name: string; children: MemoryNode[] }
  | { kind: 'file'; name: string; path: string; content: string };
type ViewedFile = { path: string; content: string };
type OrchestratorClient = OrchestratorWasm | RemoteOrchestrator;

const INFERENCE_PROVIDERS = [
  { id: 'gemini', label: 'Google Gemini', models: ['gemini-3.6-flash', 'gemini-3.6-pro'], disabled: false },
  { id: 'openai', label: 'OpenAI (coming soon)', models: ['gpt-5', 'gpt-5-mini'], disabled: true },
  { id: 'anthropic', label: 'Anthropic (coming soon)', models: ['claude-4.5-sonnet'], disabled: true },
] as const;
const SERVICE_TIERS = ['default', 'flex', 'priority'];
const SETTINGS_STORAGE_KEY = 'actualised.inference-settings';
const RATE_LIMIT_STORAGE_KEY = 'actualised.rate-limits';
const USE_REMOTE_ORCHESTRATOR = import.meta.env.VITE_ORCHESTRATOR_MODE === 'remote';
const ORCHESTRATOR_STREAM_URL = USE_REMOTE_ORCHESTRATOR ? '/api/orchestrator/stream' : import.meta.env.VITE_ORCHESTRATOR_STREAM_URL as string | undefined;

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
  const fallback: RateLimitSettings = { maxConcurrentRequests: 5, requestsPerMinute: 0 };
  try {
    const raw = localStorage.getItem(RATE_LIMIT_STORAGE_KEY);
    if (!raw) return fallback;
    const stored = JSON.parse(raw) as Partial<RateLimitSettings> & { requestsPerSecond?: number };
    return {
      maxConcurrentRequests: stored.maxConcurrentRequests ?? fallback.maxConcurrentRequests,
      requestsPerMinute: stored.requestsPerMinute ?? (stored.requestsPerSecond ?? 0) * 60,
    };
  } catch {
    return fallback;
  }
}

function graphElements(agents: Agent[]): ElementDefinition[] {
  const root = agents.find((agent) => !agent.parent_id);
  return [
    ...agents.map((agent) => ({
      data: { id: agent.id, label: agent.name.replaceAll(' ', '\n') },
      classes: !agent.parent_id ? 'root' : agent.parent_id === root?.id ? 'lead' : 'contributor',
    })),
    ...agents.filter((agent) => agent.parent_id).map((agent) => ({ data: { id: `${agent.parent_id}-${agent.id}`, source: agent.parent_id!, target: agent.id } })),
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

const Dashboard: Component<{ companyId: string; companyName: string; companies: Array<{id: string, name: string}>; initialOrchestrator?: OrchestratorClient; onSwitchCompany: (id: string) => void; onAddCompany: () => void }> = (props) => {
  const [isSidebarOpen, setIsSidebarOpen] = createSignal(false);
  let cyContainer!: HTMLDivElement;
  let cy: Core | undefined;
  let orchestrator: OrchestratorClient | undefined = props.initialOrchestrator;
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
  let toolsRef: HTMLElement | undefined;
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
  const [tools, setTools] = createSignal<Tool[]>([]);
  const [isToolLibraryOpen, setIsToolLibraryOpen] = createSignal(false);
  const [isEditingAgent, setIsEditingAgent] = createSignal(false);
  const [draftAgent, setDraftAgent] = createSignal<Agent>();
  const [isContinuousLoop, setIsContinuousLoop] = createSignal(false);

  const applyInferenceSettings = async (settings: InferenceSettings) => {
    try {
      await callOrchestrator((o) => o.configure_inference({ provider: settings.provider, model: settings.model, service_tier: settings.serviceTier || null, api_key: settings.apiKey }));
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : 'Failed to apply inference settings');
    }
  };

  const applyRateLimits = async (settings: RateLimitSettings) => {
    try {
      await callOrchestrator((o) => o.configure_rate_limits({
        max_concurrent_requests: settings.maxConcurrentRequests,
        requests_per_minute: settings.requestsPerMinute,
        working_hours: null,
      }));
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
    setIsToolLibraryOpen(false);
    setViewedMemory(undefined);
    setIsSharedOpen(true);
    if (!orchestrator) return;
    try {
      setSharedTree(await callOrchestrator((o) => o.get_shared_tree() as MemoryNode[]));
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : 'Failed to load the shared directory');
    }
  };

  // SSE streaming connection to serve as the chat interface boundary to the orchestrator/SDKs
  createEffect(() => ORCHESTRATOR_STREAM_URL, (streamUrl) => {
    if (!streamUrl) return;
    // In a deployed environment, this connects to the SDK/Orchestrator backend
    const sse = new EventSource(streamUrl);
    sse.onmessage = (event) => {
      const data = JSON.parse(event.data);
      if (data.type === 'chat_update' && data.agentId === selectedAgent()?.id) {
        void refreshAgentDetails(data.agentId);
      } else if (data.type === 'state_changed' && orchestrator instanceof RemoteOrchestrator) {
        void callOrchestrator(async (remote) => {
          if (!(remote instanceof RemoteOrchestrator)) return;
          await remote.refresh();
          const companyAgents = remote.get_agents() as Agent[];
          setAgents(companyAgents);
          setProjects(remote.get_projects() as Project[]);
          setTools(remote.get_tools() as Tool[]);
          const selected = companyAgents.find((agent) => agent.id === selectedAgent()?.id);
          if (selected) {
            setSelectedAgent(selected);
            return selected.id;
          }
        }).then((selectedId) => selectedId && refreshAgentDetails(selectedId));
      }
    };
    return () => sse.close();
  });

  const sendAgentMessage = async (agentId: string) => {
    const message = messageDraft().trim();
    if (!message || !orchestrator) return;
    setMessageDraft('');
    // For local WASM mode, we call directly. For remote mode, we would POST to an API.
    await callOrchestrator((o) => o.send_agent_message(agentId, message));
    await refreshAgentDetails(agentId);
  };

  const openSettings = () => {
    setIsInspectorOpen(false);
    setIsSharedOpen(false);
    setIsToolLibraryOpen(false);
    setDraftSettings(inferenceSettings());
    setDraftRateLimit(rateLimitSettings());
    setIsSettingsOpen(true);
  };

  const openToolLibrary = () => {
    setIsInspectorOpen(false);
    setIsSharedOpen(false);
    setIsSettingsOpen(false);
    setIsToolLibraryOpen(true);
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

  const toggleContinuousLoop = async () => {
    if (isContinuousLoop()) {
      setIsContinuousLoop(false);
      return;
    }
    setIsContinuousLoop(true);
    while (isContinuousLoop()) {
      await runOrchestratorCycle();
      await new Promise(resolve => setTimeout(resolve, 500));
    }
  };

  const selectAgent = (agent: Agent, openInspector = true) => {
    setIsSettingsOpen(false);
    setIsSharedOpen(false);
    setIsToolLibraryOpen(false);
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
    else if (isToolLibraryOpen()) setIsToolLibraryOpen(false);
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
  const trapToolsTab = (event: KeyboardEvent) => trapTabWithin(toolsRef)(event);

  createEffect(isToolLibraryOpen, (open) => {
    if (open) toolsRef?.querySelector<HTMLElement>('.close-button')?.focus();
    else lastFocusedNodeButton?.focus();
  });

  createEffect(isInspectorOpen, (open) => {
    if (open) {
      inspectorRef?.querySelector<HTMLElement>('.close-button')?.focus();
    } else {
      lastFocusedNodeButton?.focus();
    }
  });

  createEffect(isSettingsOpen, (open) => {
    if (open) {
      settingsRef?.querySelector<HTMLElement>('select, input')?.focus();
    } else {
      lastFocusedNodeButton?.focus();
    }
  });

  createEffect(isSharedOpen, (open) => {
    if (open) {
      sharedRef?.querySelector<HTMLElement>('.memory-leaf, summary, .close-button')?.focus();
    } else {
      lastFocusedNodeButton?.focus();
    }
  });

  createEffect(viewedMemory, (file) => {
    if (!file) {
      inspectorRef?.querySelector<HTMLElement>('.memory-leaf, summary')?.focus();
    }
  });

  createEffect(() => agentContext()?.history?.length, () => {
    if (chatScrollRef) chatScrollRef.scrollTop = chatScrollRef.scrollHeight;
  });

  onSettled(() => {
    void (async () => {
      try {
        orchestrator = await createOrchestrator(orchestrator as OrchestratorWasm | undefined);
        await applyInferenceSettings(inferenceSettings());
        await applyRateLimits(rateLimitSettings());
        const companyAgents = orchestrator.get_agents() as Agent[];
        setAgents(companyAgents);
        setProjects(orchestrator.get_projects() as Project[]);
        setTools(orchestrator.get_tools() as Tool[]);
        setSelectedAgent(companyAgents[0]);
        if (companyAgents[0]) await refreshAgentDetails(companyAgents[0].id);

        const isDark = window.matchMedia('(prefers-color-scheme: dark)').matches;
        const labelColor = isDark ? '#e8e6e3' : '#3c3830';
        const edgeColor = isDark ? '#6b665b' : '#9d9789';

        cy = cytoscape({
          container: cyContainer,
          elements: graphElements(companyAgents),
          style: [
            { selector: 'node', style: { label: 'data(label)', color: labelColor, 'font-size': 11, 'font-weight': 600, 'text-halign': 'center', 'text-valign': 'top', 'text-justification': 'center', 'text-wrap': 'wrap', 'text-margin-y': -10, 'background-color': '#9fc7c0', 'border-width': 2, 'border-color': '#547b75', width: 44, height: 44 } },
            { selector: 'node.root', style: { 'background-color': '#f4b8a8', 'border-color': '#a56354', width: 56, height: 56 } },
            { selector: 'node.lead', style: { 'background-color': '#d9c4e9', 'border-color': '#866b9c', width: 50, height: 50 } },
            { selector: 'node.contributor', style: { 'background-color': '#b9d8d1', 'border-color': '#568b80' } },
            { selector: 'node.selected', style: { 'border-width': 4, 'border-color': '#c25b3f', 'overlay-opacity': 0 } },
            { selector: 'edge', style: { width: 1.5, 'line-color': edgeColor, 'target-arrow-color': edgeColor, 'target-arrow-shape': 'triangle', 'curve-style': 'bezier' } },
          ],
          layout: { name: 'breadthfirst', directed: true, padding: 110, spacingFactor: 1.65 },
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

  createEffect(agents, (currentAgents) => {
    if (cy && currentAgents.length > 0) {
      cy.elements().remove();
      cy.add(graphElements(currentAgents));
      cy.layout({ name: 'breadthfirst', directed: true, padding: 110, spacingFactor: 1.65 }).run();
      cy.$id(selectedAgent()?.id ?? '').addClass('selected');
    }
  });

  onCleanup(() => cy?.destroy());

  return <div class="app-layout" style="display:flex; height:100vh; overflow:hidden;">
    <Show when={isSidebarOpen()}>
      <div class="sidebar-overlay" style="position:fixed; top:0; left:0; right:0; bottom:0; background:rgba(0,0,0,0.5); z-index:90;" onClick={() => setIsSidebarOpen(false)}></div>
      <aside class="left-sidebar" style="position:fixed; top:0; left:0; bottom:0; width:260px; background:var(--bg); border-right:1px solid var(--border); z-index:100; display:flex; flex-direction:column; box-shadow: 4px 0 16px rgba(0,0,0,0.1);">
        <div class="inspector-nav" style="justify-content: space-between; align-items: center; border-bottom: 1px solid var(--border);">
          <p class="eyebrow" style="margin:0;">Your Companies</p>
          <button class="icon-button close-button" aria-label="Close sidebar" onClick={() => setIsSidebarOpen(false)}>X</button>
        </div>
        <ul style="flex:1; overflow-y:auto; list-style:none; padding:8px 0; margin:0;">
          <For each={props.companies}>
            {(company) => (
              <li 
                style={`padding: 12px 24px; cursor: pointer; display: flex; align-items: center; gap: 8px; ${company.id === props.companyId ? 'background: var(--bg-hover); font-weight: 600;' : ''}`}
                onClick={() => { setIsSidebarOpen(false); props.onSwitchCompany(company.id); }}
              >
                <div style={`width:8px; height:8px; border-radius:50%; ${company.id === props.companyId ? 'background:#c25b3f;' : 'background:transparent;'}`}></div>
                {company.name}
              </li>
            )}
          </For>
        </ul>
        <div style="padding: 16px 24px; border-top: 1px solid var(--border);">
          <button class="secondary-button" style="width:100%; display:flex; align-items:center; justify-content:center; gap:8px;" onClick={() => { setIsSidebarOpen(false); props.onAddCompany(); }}>
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><line x1="12" y1="5" x2="12" y2="19"></line><line x1="5" y1="12" x2="19" y2="12"></line></svg>
            New Company
          </button>
        </div>
      </aside>
    </Show>
<main class="canvas-page" style="flex:1; position:relative;">
    <header class="canvas-header">
      <div style="display: flex; align-items: center; gap: 16px;">
        <button type="button" class="icon-button" aria-label="Toggle sidebar" onClick={() => setIsSidebarOpen(!isSidebarOpen())}>
          <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="3" y1="12" x2="21" y2="12"></line><line x1="3" y1="6" x2="21" y2="6"></line><line x1="3" y1="18" x2="21" y2="18"></line></svg>
        </button>
        <div style="display: flex; align-items: baseline; gap: 12px;">
          <h1>{props.companyName}</h1>
          <span style="color: var(--muted); font-size: 15px; font-weight: 500;">{agents().length} agents</span>
        </div>
      </div>
      <div class="canvas-actions">
        <Show when={isOrchestratorRunning() || isContinuousLoop()}>
          <div class="pulsing-indicator" aria-label="Orchestrator is running" style="display:flex; align-items:center; color:#c25b3f; margin-right:4px;">
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" class="pulse-anim">
              <circle cx="12" cy="12" r="8" fill="currentColor" />
            </svg>
          </div>
        </Show>
        <button type="button" class="icon-button" aria-label="Add Agent" onClick={async () => {
          if (!orchestrator) return;
          const id = `agent_${Date.now()}`;
          await callOrchestrator(o => o.add_agent({ id, name: 'New Agent', role: 'Staff Engineer', system_prompt: 'You are a new agent.', tools: [], parent_id: selectedAgent()?.id ?? null }));
          const companyAgents = orchestrator!.get_agents() as Agent[];
          setAgents(companyAgents);
          selectAgent(companyAgents.find(a => a.id === id)!, true);
          setIsEditingAgent(true);
          setDraftAgent(companyAgents.find(a => a.id === id)!);
        }}>
          <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="12" y1="5" x2="12" y2="19"></line><line x1="5" y1="12" x2="19" y2="12"></line></svg>
        </button>
        <button
          type="button"
          class="icon-button step-button"
          classList={{ 'is-running': isOrchestratorRunning() && !isContinuousLoop() }}
          aria-label="Step orchestrator cycle"
          disabled={isOrchestratorRunning()}
          onClick={runOrchestratorCycle}
        >
          <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polygon points="5 4 15 12 5 20 5 4"></polygon><line x1="19" y1="5" x2="19" y2="19"></line></svg>
        </button>
        <button
          type="button"
          class="icon-button play-button"
          classList={{ 'is-running': isContinuousLoop() }}
          aria-label={isContinuousLoop() ? 'Pause continuous execution' : 'Start continuous execution'}
          onClick={toggleContinuousLoop}
        >
          {isContinuousLoop() ? 
            <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="6" y="4" width="4" height="16"></rect><rect x="14" y="4" width="4" height="16"></rect></svg> : 
            <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polygon points="5 3 19 12 5 21 5 3"></polygon></svg>
          }
        </button>
        <button type="button" class="icon-button" aria-label="Shared team directory" onClick={openSharedDirectory}>
          <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"></path></svg>
        </button>
        <button type="button" class="icon-button" aria-label="Tool Library" onClick={openToolLibrary}>
          <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M14.7 6.3a1 1 0 0 0 0 1.4l1.6 1.6a1 1 0 0 0 1.4 0l3.77-3.77a6 6 0 0 1-7.94 7.94l-6.91 6.91a2.12 2.12 0 0 1-3-3l6.91-6.91a6 6 0 0 1 7.94-7.94l-3.76 3.76z"></path></svg>
        </button>
        <button type="button" class="icon-button" aria-label="Inference settings" onClick={openSettings}>
          <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="3"></circle><path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z"></path></svg>
        </button>
        <button type="button" class="icon-button" aria-label="Centre canvas" onClick={() => cy?.fit(undefined, 60)}>
          <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"></circle><line x1="12" y1="2" x2="12" y2="6"></line><line x1="12" y1="18" x2="12" y2="22"></line><line x1="4.93" y1="4.93" x2="7.76" y2="7.76"></line><line x1="16.24" y1="16.24" x2="19.07" y2="19.07"></line><line x1="2" y1="12" x2="6" y2="12"></line><line x1="18" y1="12" x2="22" y2="12"></line><line x1="4.93" y1="19.07" x2="7.76" y2="16.24"></line><line x1="16.24" y1="7.76" x2="19.07" y2="4.93"></line></svg>
        </button>
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
        <div class="inspector-header" style="display:flex; justify-content:space-between; align-items:center;">
          <p class="eyebrow">Agent details</p>
          <Show when={!isEditingAgent()}>
            <button class="secondary-button" onClick={() => { setDraftAgent({...agent()}); setIsEditingAgent(true); }}>Edit</button>
          </Show>
        </div>
        
        <Show when={!isEditingAgent()}>
          <h2>{agent().name}</h2><p class="agent-role">{agent().role}</p>
          <div class="inspector-section"><h3>Operating brief</h3><p>{agent().system_prompt}</p></div>
          <Show when={(agent().scheduled_tasks?.length ?? 0) > 0}>
            <div class="inspector-section">
              <h3>Scheduled Tasks</h3>
              <ul class="task-list">
                <For each={agent().scheduled_tasks}>
                  {(task) => <li>
                    <input type="checkbox" checked={task.completed} disabled />
                    <div>
                      <span style={task.completed ? "text-decoration: line-through;" : ""}>{task.description}</span>
                      <div style="font-size: 11px; color: var(--text-muted); margin-top: 4px;">Due: {new Date(task.due_date).toLocaleString()}</div>
                    </div>
                  </li>}
                </For>
              </ul>
            </div>
          </Show>
          <div class="inspector-section"><h3>Tools</h3><div class="tool-list"><For each={agent().tools}>{(tool) => <span>{tool}</span>}</For></div></div>
        </Show>
        
        <Show when={isEditingAgent()}>
          <form onSubmit={async (e) => { 
            e.preventDefault(); 
            if(!orchestrator) return;
            await callOrchestrator(o => o.update_agent(draftAgent()!.id, draftAgent()));
            setAgents(orchestrator!.get_agents() as Agent[]);
            setSelectedAgent(draftAgent());
            setIsEditingAgent(false);
          }} class="settings-form" style="margin-top: 16px;">
            <label>Name <input required value={draftAgent()?.name} onInput={e => setDraftAgent(p => ({...p!, name: e.currentTarget.value}))} /></label>
            <label>Role <input required value={draftAgent()?.role} onInput={e => setDraftAgent(p => ({...p!, role: e.currentTarget.value}))} /></label>
            <label>Manager
              <select 
                value={draftAgent()?.parent_id || ''} 
                onChange={e => setDraftAgent(p => ({...p!, parent_id: e.currentTarget.value || null}))}
              >
                <option value="">None (Top Level)</option>
                <For each={agents().filter(a => a.id !== draftAgent()?.id)}>
                  {(a) => <option value={a.id}>{a.name} ({a.role})</option>}
                </For>
              </select>
            </label>
            <label>Operating brief <textarea required value={draftAgent()?.system_prompt} rows={4} onInput={e => setDraftAgent(p => ({...p!, system_prompt: e.currentTarget.value}))} /></label>
            <div class="inspector-section">
              <label>Tools
              <div class="tool-list" style="margin-bottom: 8px;">
                <For each={draftAgent()?.tools}>{(tool) => 
                  <span style="display:inline-flex; align-items:center; gap:4px; background:var(--bg); border:1px solid var(--border); padding:2px 8px; border-radius:4px; font-size:13px;">
                    {tool} 
                    <button type="button" onClick={() => setDraftAgent(p => ({...p!, tools: p!.tools.filter(t => t !== tool)}))} style="border:none; background:none; cursor:pointer; color:var(--text); padding:0;">×</button>
                  </span>
                }</For>
              </div>
                <select onChange={(e) => {
                  const val = e.currentTarget.value;
                  if (val && !draftAgent()?.tools.includes(val)) {
                     setDraftAgent(p => ({...p!, tools: [...p!.tools, val]}));
                  }
                  e.currentTarget.value = "";
                }}>
                  <option value="">-- Add Tool --</option>
                  <For each={tools()}>{(t) => <option value={t.name}>{t.name}</option>}</For>
                </select>
              </label>
            </div>
            <div style="display:flex; justify-content:space-between; margin-top:16px;">
              <div style="display:flex; gap:8px;">
                <button type="submit" class="primary-button">Save</button>
                <button type="button" class="secondary-button" onClick={() => setIsEditingAgent(false)}>Cancel</button>
              </div>
              <button type="button" class="secondary-button" style="color:#c25b3f; border-color:#c25b3f;" onClick={async () => {
                if (confirm('Are you sure you want to delete this agent?')) {
                  if(!orchestrator) return;
                  await callOrchestrator(o => o.remove_agent(draftAgent()!.id));
                  const newAgents = orchestrator!.get_agents() as Agent[];
                  setAgents(newAgents);
                  setSelectedAgent(newAgents[0]);
                  setIsEditingAgent(false);
                }
              }}>Delete</button>
            </div>
          </form>
        </Show>

        <Show when={!agent().parent_id && !isEditingAgent()}><div class="inspector-section"><h3>Root projects</h3><ul class="project-list"><For each={projects()}>{(project) => <li><strong>{project.title}</strong><span>{project.description}</span></li>}</For></ul></div></Show>

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
              <Show when={(agentContext()?.history?.length ?? 0) > 0} fallback={<p class="empty-hint">No turns run yet.</p>}>
                <div class="chat-scroll" ref={chatScrollRef}>
                  <For each={agentContext()?.history ?? []}>
                    {(turn) => <div class={`chat-bubble chat-${turn.role}`}><span class="chat-role">{turn.role === 'operator' ? 'Operator' : agent().name}</span><p>{turn.content}</p></div>}
                  </For>
                </div>
              </Show>
              <Show when={(agentContext()?.pending_messages?.length ?? 0) > 0}>
                <p class="context-label">Queued for next turn</p>
                <ul class="pending-list"><For each={agentContext()?.pending_messages ?? []}>{(message) => <li>{message}</li>}</For></ul>
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
      <aside ref={sharedRef} class="settings-popover shared-popover" aria-label="Shared team directory" onKeyDown={trapSharedTab}
        onDragOver={(e) => { e.preventDefault(); e.stopPropagation(); }}
        onDrop={async (e) => {
          e.preventDefault();
          e.stopPropagation();
          if (!orchestrator) return;
          if (e.dataTransfer?.items) {
            for (const item of Array.from(e.dataTransfer.items)) {
              if (item.kind === 'file') {
                const file = item.getAsFile();
                if (file) {
                  const content = await file.text();
                  await callOrchestrator(o => o.add_shared_file({
                    id: `file_${Date.now()}_${file.name}`,
                    name: file.name,
                    content
                  }));
                }
              }
            }
            setSharedTree(await callOrchestrator((o) => o.get_shared_tree() as MemoryNode[]));
          }
        }}
      >
        <div class="inspector-nav">
          <p class="eyebrow">Shared team directory</p>
          <button class="icon-button close-button" aria-label="Close shared directory" onClick={() => { setIsSharedOpen(false); setViewedMemory(undefined); }}>×</button>
        </div>
        <Show when={sharedTree().length > 0} fallback={<p class="empty-hint">No shared files written yet. Drop files here to upload.</p>}>
          <MemoryTreeView nodes={sharedTree()} onOpenFile={(file) => setViewedMemory(file)} />
          <p class="settings-hint" style="margin-top: 16px;">Drop files here to upload to the shared directory.</p>
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
            <label>Requests per minute (0 = unlimited)
              <input
                type="number"
                min="0"
                step="0.1"
                value={draftRateLimit().requestsPerMinute}
                onInput={(event) => setDraftRateLimit((prev) => ({ ...prev, requestsPerMinute: Number(event.currentTarget.value) || 0 }))}
              />
            </label>
          </fieldset>
          
          <fieldset class="rate-limit-fieldset" style="margin-top: 24px; border-color: #c25b3f;">
            <legend style="color: #c25b3f;">Danger Zone</legend>
            <p style="font-size: 13px; margin-bottom: 12px; color: var(--text-muted);">Permanently delete this company and all of its agents, projects, and files. This action cannot be undone.</p>
            <button type="button" class="secondary-button" style="color: #c25b3f; border-color: #c25b3f;" onClick={async () => {
              if (confirm(`Are you sure you want to delete ${props.companyName}?`)) {
                await deleteCompany(props.companyId);
                window.location.reload();
              }
            }}>Delete Company</button>
          </fieldset>
<p class="settings-hint">Stored only in this browser. Google Gemini is the only provider wired up right now — the rest are placeholders.</p>
          <button type="submit" class="primary-button">Save &amp; apply</button>
        </form>
      </aside>
    </Show>

    <Show when={isToolLibraryOpen()}>
      <aside ref={toolsRef} class="settings-popover" aria-label="Tool Library" onKeyDown={trapToolsTab}>
        <div class="inspector-nav">
          <p class="eyebrow">Tool Library</p>
          <button class="icon-button close-button" aria-label="Close tool library" onClick={() => setIsToolLibraryOpen(false)}>×</button>
        </div>
        <div class="inspector-section" style="overflow-y: auto; max-height: calc(100vh - 120px);">
          <For each={tools()}>{(tool) => 
            <div style="margin-bottom: 24px; border-bottom: 1px solid var(--border); padding-bottom: 24px;">
              <div style="display:flex; justify-content:space-between; align-items:center; margin-bottom:8px;">
                <h3 style="margin:0;">{tool.name}</h3>
                <button class="secondary-button" style="padding:4px 8px; font-size:12px; color:#c25b3f; border-color:#c25b3f;" onClick={async () => {
                  if(!orchestrator) return;
                  await callOrchestrator(o => o.remove_tool(tool.name));
                  setTools(orchestrator!.get_tools() as Tool[]);
                }}>Remove</button>
              </div>
              <p style="font-size:14px; margin-bottom:12px; line-height:1.4;">{tool.description}</p>
              <h4 style="font-size:12px; text-transform:uppercase; letter-spacing:0.5px; margin-bottom:4px; color:var(--text);">Parameters (JSON Schema)</h4>
              <pre class="memory-viewer-content" style="font-size:12px; padding:12px; background:var(--code-bg); border-radius:6px; overflow-x:auto;">{JSON.stringify(tool.parameters, null, 2)}</pre>
            </div>
          }</For>
        </div>
      </aside>
    </Show>
  </main></div>;
};

const SetupPage: Component<{ onComplete: (companyName: string, orchestrator?: OrchestratorClient) => void }> = (props) => {
  let localOrchestrator: OrchestratorWasm | undefined;
  const [companyName, setCompanyName] = createSignal<string | null>();
  const [draftName, setDraftName] = createSignal('');
  const [error, setError] = createSignal<string>();
  const [isSubmitting, setIsSubmitting] = createSignal(false);

  onSettled(() => {
    void (async () => {
      try {
        if (USE_REMOTE_ORCHESTRATOR) {
          const result = await inspectCompany();
          setCompanyName(result.name);
        } else {
          const result = await inspectCompany();
          localOrchestrator = result.orchestrator as OrchestratorWasm;
          setCompanyName(result.name);
        }
      } catch (cause) {
        setError(cause instanceof Error ? cause.message : 'Could not check company status');
        setCompanyName(null);
      }
    })();
  });

  const foundVenture = async (event: SubmitEvent) => {
    event.preventDefault();
    const name = draftName().trim();
    if (!name) return;
    if (USE_REMOTE_ORCHESTRATOR) {
      window.location.search = `?company=${encodeURIComponent(name)}`;
      return;
    }
    setIsSubmitting(true);
    setError();
    try {
      const orchestrator = await foundCompany(name, localOrchestrator);
      props.onComplete(name, orchestrator);
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : 'Could not found venture');
    } finally {
      setIsSubmitting(false);
    }
  };

  return <main class="setup-page">
    <p class="eyebrow">Actualised.ai</p><h1>Build the company that builds the product.</h1>
    <p>Start with a Project Manager coordinating the Engineering, Product, and Growth teams.</p>
    <Show when={companyName() !== undefined} fallback={<p>Checking company status...</p>}>
      <Show
        when={companyName()}
        fallback={<form class="setup-form" onSubmit={foundVenture}>
          <label for="company-name">Company name</label>
          <input id="company-name" required value={draftName()} onInput={(event) => setDraftName(event.currentTarget.value)} />
          <button class="primary-button" type="submit" disabled={isSubmitting()}>{isSubmitting() ? 'Founding...' : 'Found Venture'}</button>
          <div style="margin-top: 16px; text-align: center;">
            <a href="?auth=login" style="color: var(--text-muted); font-size: 14px; text-decoration: underline;">Already have an account? Sign in</a>
          </div>
        </form>}
      >
        {(name) => <button class="primary-button" onClick={() => props.onComplete(name(), localOrchestrator)}>Continue building {name()}</button>}
      </Show>
    </Show>
    <Show when={error()}>{(message) => <p class="error-banner">{message()}</p>}</Show>
  </main>;
};

const AuthPage: Component<{ onComplete: () => void }> = (props) => {
  const params = new URLSearchParams(window.location.search);
  const companyName = params.get('company');
  const isLoginMode = params.get('auth') === 'login';
  
  const [isLogin, setIsLogin] = createSignal(isLoginMode || !companyName);
  const [email, setEmail] = createSignal('');
  const [password, setPassword] = createSignal('');
  const [error, setError] = createSignal<string>();
  const [isSubmitting, setIsSubmitting] = createSignal(false);

  const handleSubmit = async (e: SubmitEvent) => {
    e.preventDefault();
    setIsSubmitting(true);
    setError();
    try {
      if (isLogin()) {
        await signin(email(), password());
      } else {
        await signup(email(), password());
      }
      
      // Auto-create company if we have it in URL
      if (companyName && !isLogin()) {
        await foundCompany(companyName);
      }
      
      // Clear URL params and proceed
      window.history.replaceState({}, '', window.location.pathname);
      props.onComplete();
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : 'Authentication failed');
    } finally {
      setIsSubmitting(false);
    }
  };

  return <main class="setup-page">
    <p class="eyebrow">Actualised.ai</p>
    <h1>{isLogin() ? 'Sign back in' : (companyName ? `Create an account to found ${companyName}` : 'Create an account')}</h1>
    <form class="setup-form" onSubmit={handleSubmit}>
      <label for="email">Email</label>
      <input id="email" type="email" required value={email()} onInput={(e) => setEmail(e.currentTarget.value)} />
      
      <label for="password">Password</label>
      <input id="password" type="password" required value={password()} onInput={(e) => setPassword(e.currentTarget.value)} />
      
      <button class="primary-button" type="submit" disabled={isSubmitting()}>
        {isSubmitting() ? 'Please wait...' : (isLogin() ? 'Sign In' : 'Sign Up')}
      </button>
      
      <div style="margin-top: 16px; text-align: center;">
        <button type="button" onClick={() => setIsLogin(!isLogin())} style="background: none; border: none; color: var(--text-muted); text-decoration: underline; cursor: pointer; font-size: 14px;">
          {isLogin() ? "Don't have an account? Sign up" : "Already have an account? Sign in"}
        </button>
      </div>
    </form>
    <Show when={error()}>{(message) => <p class="error-banner">{message()}</p>}</Show>
  </main>;
};

const App: Component = () => {
  const [companies, setCompanies] = createSignal<Array<{id: string, name: string}>>([]);
  const [activeCompanyId, setActiveCompanyIdState] = createSignal<string | null>(null);
  const [initialOrchestrator, setInitialOrchestrator] = createSignal<OrchestratorClient>();
  const [isLoading, setIsLoading] = createSignal(true);
  
  const params = new URLSearchParams(window.location.search);
  const showAuth = params.has('company') || params.has('auth');
  const [viewState, setViewState] = createSignal<'auth' | 'setup' | 'dashboard'>(showAuth ? 'auth' : 'setup');

  onSettled(() => {
    if (showAuth) {
      setIsLoading(false);
      return;
    }
    void (async () => {
      try {
        const userCompanies = await getCompanies();
        setCompanies(userCompanies);
        if (userCompanies.length > 0) {
          const stored = localStorage.getItem('activeCompanyId');
          const toSelect = userCompanies.find(c => c.id === stored) || userCompanies[0];
          switchCompany(toSelect.id);
        } else {
          setViewState('setup');
          setIsLoading(false);
        }
      } catch {
        setViewState('setup');
        setIsLoading(false);
      }
    })();
  });

  const switchCompany = async (id: string) => {
    setActiveCompanyIdState(id);
    setActiveCompanyId(id);
    localStorage.setItem('activeCompanyId', id);
    try {
      const result = await inspectCompany();
      setInitialOrchestrator(result.orchestrator);
      setViewState('dashboard');
    } catch {
      setViewState('setup');
    } finally {
      setIsLoading(false);
    }
  };

  const completeSetup = async (name: string, orchestrator?: OrchestratorClient) => {
    // Reload companies to get the newly created one
    const userCompanies = await getCompanies();
    setCompanies(userCompanies);
    const newComp = userCompanies.find(c => c.name === name) || userCompanies[userCompanies.length - 1];
    if (newComp) {
      switchCompany(newComp.id);
    } else {
      // Local WASM mock
      setInitialOrchestrator(orchestrator);
      setActiveCompanyIdState('local');
      setCompanies([{ id: 'local', name }]);
      setViewState('dashboard');
    }
  };
  
  const completeAuth = () => {
    window.location.search = '';
  };

  return (
    <Show when={!isLoading()} fallback={<div style="padding: 40px; text-align: center; color: var(--text-muted);">Loading workspace...</div>}>
      <Show when={viewState() === 'dashboard'} fallback={
        <Show when={viewState() === 'auth'} fallback={<SetupPage onComplete={completeSetup} />}>
          <AuthPage onComplete={completeAuth} />
        </Show>
      }>
        <Dashboard 
          companyId={activeCompanyId()!} 
          companyName={companies().find(c => c.id === activeCompanyId())?.name || ''} 
          companies={companies()} 
          initialOrchestrator={initialOrchestrator()} 
          onSwitchCompany={(id) => { setIsLoading(true); switchCompany(id); }}
          onAddCompany={() => setViewState('setup')}
        />
      </Show>
    </Show>
  );
};

export default App;
