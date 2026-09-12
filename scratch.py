import re

with open("C:/Users/Sebastien Ringrose/.gemini/antigravity/brain/8b7ccb9c-86b2-47fc-afc8-4ce1df53d5c7/task.md", "r", encoding="utf-8") as f:
    c = f.read()

c = c.replace("- `[ ]` **Phase 1: Data Models & Schema**", "- `[x]` **Phase 1: Data Models & Schema**")
c = c.replace("- `[ ]` Remove `shared_file`", "- `[x]` Remove `shared_file`")
c = c.replace("- `[ ]` Add `RepositoryConfig`", "- `[x]` Add `RepositoryConfig`")
c = c.replace("- `[ ]` Add `Issue` and `Comment` models", "- `[x]` Add `Issue` and `Comment` models")
c = c.replace("- `[ ]` Add `issue` table to SurrealDB", "- `[x]` Add `issue` table to SurrealDB")
c = c.replace("- `[ ]` **Phase 2: Traits and Providers**", "- `[x]` **Phase 2: Traits and Providers**")
c = c.replace("- `[ ]` Create `crates/core/src/vcs.rs`", "- `[x]` Create `crates/core/src/vcs.rs`")
c = c.replace("- `[ ]` Create `crates/core/src/providers/github.rs`", "- `[x]` Create `crates/core/src/providers/github.rs`")

with open("C:/Users/Sebastien Ringrose/.gemini/antigravity/brain/8b7ccb9c-86b2-47fc-afc8-4ce1df53d5c7/task.md", "w", encoding="utf-8") as f:
    f.write(c)
