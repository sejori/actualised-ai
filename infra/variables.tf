variable "project_id" {
  type        = string
  description = "The GCP project ID"
}

variable "region" {
  type        = string
  description = "The GCP region"
  default     = "europe-west2"
}

variable "orchestrator_image" {
  type        = string
  description = "The container image URI for the Actualised.ai orchestrator"
}

variable "surrealdb_url" {
  type        = string
  description = "Connection string for SurrealDB Cloud"
}

variable "surrealdb_user" {
  type        = string
  description = "SurrealDB Cloud username"
}

variable "surrealdb_pass" {
  type        = string
  sensitive   = true
  description = "SurrealDB Cloud password"
}

variable "telegram_token" {
  type        = string
  sensitive   = true
  description = "Telegram Bot Token"
}
