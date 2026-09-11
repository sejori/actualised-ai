import re

with open("packages/web-ui/src/App.tsx", "r", encoding="utf-8") as f:
    c = f.read()

# 1. Fix sidebar inspector-nav padding and the Close button
old_sidebar_header = """<div class="inspector-nav" style="justify-content: space-between; align-items: center; border-bottom: 1px solid var(--border);">
          <p class="eyebrow" style="margin:0;">Your Companies</p>
          <button class="icon-button close-button" aria-label="Close sidebar" onClick={() => setIsSidebarOpen(false)}>X</button>
        </div>"""

new_sidebar_header = """<div class="inspector-nav" style="justify-content: space-between; align-items: center; border-bottom: 1px solid var(--border); padding: 16px 24px;">
          <p class="eyebrow" style="margin:0;">Your Companies</p>
          <button class="icon-button close-button" aria-label="Close sidebar" onClick={() => setIsSidebarOpen(false)}>
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="18" y1="6" x2="6" y2="18"></line><line x1="6" y1="6" x2="18" y2="18"></line></svg>
          </button>
        </div>"""

c = c.replace(old_sidebar_header, new_sidebar_header)

# 2. Fix the Login link positioning so it's not trapped in the CSS Grid form
old_form = """<form class="setup-form" onSubmit={foundVenture}>
            <label for="company-name">Company name</label>
            <input id="company-name" required value={draftName()} onInput={(event) => setDraftName(event.currentTarget.value)} />
            <button class="primary-button" type="submit" disabled={isSubmitting()}>{isSubmitting() ? 'Founding...' : 'Found Venture'}</button>
            <div style="margin-top: 16px; text-align: center;">
              <a href="?auth=login" style="color: var(--text-muted); font-size: 14px; text-decoration: underline;">Already have an account? Sign in</a>
            </div>
          </form>"""

new_form = """<div>
          <form class="setup-form" onSubmit={foundVenture}>
            <label for="company-name">Company name</label>
            <input id="company-name" required value={draftName()} onInput={(event) => setDraftName(event.currentTarget.value)} />
            <button class="primary-button" type="submit" disabled={isSubmitting()}>{isSubmitting() ? 'Founding...' : 'Found Venture'}</button>
          </form>
          <div style="margin-top: 24px;">
            <a href="?auth=login" style="color: var(--text-muted); font-size: 14px; text-decoration: underline;">Already have an account? Sign in</a>
          </div>
          </div>"""

c = c.replace(old_form, new_form)

with open("packages/web-ui/src/App.tsx", "w", encoding="utf-8") as f:
    f.write(c)

