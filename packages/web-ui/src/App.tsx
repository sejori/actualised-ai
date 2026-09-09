import { createSignal, createEffect, Show } from 'solid-js';
import type { Component } from 'solid-js';
import cytoscape from 'cytoscape';
// @ts-ignore
import initWasm, { OrchestratorWasm } from './wasm/actualised_core_wasm.js';

const Dashboard: Component = () => {
  let cyContainer!: HTMLDivElement;

  createEffect(async () => {
    // Initialize WASM
    try {
      await initWasm();
      console.log("WASM Initialized Successfully");
      const orchestrator = await OrchestratorWasm.init();
      console.log("Orchestrator created", orchestrator);
    } catch (e) {
      console.error("Failed to load WASM or initialize orchestrator", e);
    }

    const cy = cytoscape({
      container: cyContainer,
      elements: [
        { data: { id: 'ceo', label: 'CEO Agent' } },
        { data: { id: 'eng', label: 'Engineering Lead' } },
        { data: { id: 'prod', label: 'Product Lead' } },
        { data: { source: 'ceo', target: 'eng' } },
        { data: { source: 'ceo', target: 'prod' } }
      ],
      style: [
        {
          selector: 'node',
          style: {
            'background-color': '#666',
            'label': 'data(label)'
          }
        },
        {
          selector: 'edge',
          style: {
            'width': 3,
            'line-color': '#ccc',
            'target-arrow-color': '#ccc',
            'target-arrow-shape': 'triangle',
            'curve-style': 'bezier'
          }
        }
      ],
      layout: {
        name: 'breadthfirst',
        directed: true,
        padding: 10
      }
    });
  });

  return (
    <div class="h-full w-full flex flex-col">
      <h2 class="text-2xl font-bold p-4">Dashboard</h2>
      <div ref={cyContainer} class="flex-grow border border-gray-300 m-4 rounded min-h-[500px]" />
    </div>
  );
};

const SetupPage: Component<{ onComplete: () => void }> = (props) => {
  return (
    <div class="flex flex-col items-center justify-center h-full space-y-4 pt-20">
      <h1 class="text-3xl font-bold">Actualised AI - Setup</h1>
      <p>Initialize your company state and top-level agents.</p>
      <button 
        class="bg-blue-600 text-white px-4 py-2 rounded hover:bg-blue-700"
        onClick={() => props.onComplete()}
      >
        Initialize Company
      </button>
    </div>
  );
};

const App: Component = () => {
  const [isSetup, setIsSetup] = createSignal(false);

  return (
    <div class="min-h-screen w-full bg-gray-50 text-gray-900 font-sans">
      <Show when={!isSetup()} fallback={<Dashboard />}>
        <SetupPage onComplete={() => setIsSetup(true)} />
      </Show>
    </div>
  );
};

export default App;
