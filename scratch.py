import re

with open("crates/core-wasm/src/lib.rs", "r", encoding="utf-8") as f:
    c = f.read()

# Remove SharedFile from imports
c = c.replace(", SharedFile}", "}")

# Replace shared_files access with issues
c = c.replace("&self.inner.state.shared_files", "&self.inner.state.issues")

# Remove add_shared_file and remove_shared_file methods
c = re.sub(r'#\[wasm_bindgen\]\s*pub async fn add_shared_file.*?Ok\(\(.*?\}\n', '', c, flags=re.DOTALL)
c = re.sub(r'#\[wasm_bindgen\]\s*pub async fn remove_shared_file.*?Ok\(\(.*?\}\n', '', c, flags=re.DOTALL)

with open("crates/core-wasm/src/lib.rs", "w", encoding="utf-8") as f:
    f.write(c)
