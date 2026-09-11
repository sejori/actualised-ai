import re

with open("packages/web-ui/src/App.tsx", "r", encoding="utf-8") as f:
    c = f.read()

c = c.replace(
    "window.location.search = ?company=;",
    "window.location.search = `?company=${encodeURIComponent(name)}`;"
)

with open("packages/web-ui/src/App.tsx", "w", encoding="utf-8") as f:
    f.write(c)
