import re
with open("infra/variables.tf", "r", encoding="utf-8") as f:
    c = f.read()

patch = """variable "telegram_token" {
  type        = string
  sensitive   = true
  description = "Telegram Bot Token"
}

variable "gemini_api_key" {
  type        = string
  sensitive   = true
  description = "Gemini API Key for inference fallback"
}"""

if "gemini_api_key" not in c:
    c = c.replace("""variable "telegram_token" {
  type        = string
  sensitive   = true
  description = "Telegram Bot Token"
}""", patch)

with open("infra/variables.tf", "w", encoding="utf-8") as f:
    f.write(c)

with open("infra/main.tf", "r", encoding="utf-8") as f:
    m = f.read()

env_patch = """      env {
        name  = "TELEGRAM_TOKEN"
        value = var.telegram_token
      }

      env {
        name  = "GEMINI_API_KEY"
        value = var.gemini_api_key
      }"""

if "GEMINI_API_KEY" not in m:
    m = m.replace("""      env {
        name  = "TELEGRAM_TOKEN"
        value = var.telegram_token
      }""", env_patch)

with open("infra/main.tf", "w", encoding="utf-8") as f:
    f.write(m)

with open(".github/workflows/cloud-run-deploy.yml", "r", encoding="utf-8") as f:
    w = f.read()

wf_patch = """          TF_VAR_surrealdb_pass: ${{ secrets.SURREALDB_PASS }}
          TF_VAR_telegram_token: ${{ secrets.TELEGRAM_TOKEN }}
          TF_VAR_gemini_api_key: ${{ secrets.GEMINI_API_KEY }}"""

if "TF_VAR_gemini_api_key" not in w:
    w = w.replace("""          TF_VAR_surrealdb_pass: ${{ secrets.SURREALDB_PASS }}
          TF_VAR_telegram_token: ${{ secrets.TELEGRAM_TOKEN }}""", wf_patch)

with open(".github/workflows/cloud-run-deploy.yml", "w", encoding="utf-8") as f:
    f.write(w)
