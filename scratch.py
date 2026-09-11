# -*- coding: utf-8 -*-
import re

with open("packages/web-ui/src/App.tsx", "r", encoding="utf-8") as f:
    c = f.read()

# Original header block to replace:
old_header = """    <header class="canvas-header">
      <button type="button" class="icon-button" aria-label="Toggle sidebar" onClick={() => setIsSidebarOpen(!isSidebarOpen())}>
          <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="3" y1="12" x2="21" y2="12"></line><line x1="3" y1="6" x2="21" y2="6"></line><line x1="3" y1="18" x2="21" y2="18"></line></svg>
        </button>
        <h1>{props.companyName}</h1>
      <div class="canvas-actions">
        <span>{agents().length} agents</span>
        <Show when={isOrchestratorRunning() || isContinuousLoop()}>"""

# Note: spacing/indentation in original might differ. Let's construct a regex.
regex = r'<header class="canvas-header">\s*<button type="button" class="icon-button" aria-label="Toggle sidebar" onClick=\{\(\) => setIsSidebarOpen\(!isSidebarOpen\(\)\)\}>\s*<svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="3" y1="12" x2="21" y2="12"></line><line x1="3" y1="6" x2="21" y2="6"></line><line x1="3" y1="18" x2="21" y2="18"></line></svg>\s*</button>\s*<h1>\{props\.companyName\}</h1>\s*<div class="canvas-actions">\s*<span>\{agents\(\)\.length\} agents</span>\s*<Show when=\{isOrchestratorRunning\(\) \|\| isContinuousLoop\(\)\}>'

new_header = """<header class="canvas-header">
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
        <Show when={isOrchestratorRunning() || isContinuousLoop()}>"""

c = re.sub(regex, new_header, c, count=1, flags=re.DOTALL)

with open("packages/web-ui/src/App.tsx", "w", encoding="utf-8") as f:
    f.write(c)

