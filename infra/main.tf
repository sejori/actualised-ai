terraform {
  required_providers {
    google = {
      source  = "hashicorp/google"
      version = "~> 5.0"
    }
  }
}

provider "google" {
  project = var.project_id
  region  = var.region
}

resource "google_cloud_run_v2_service" "orchestrator" {
  name     = "actualised-orchestrator"
  location = var.region

  template {
    scaling {
      # Scale to zero when not in use
      min_instance_count = 0
      max_instance_count = 1
    }

    containers {
      image = var.orchestrator_image

      env {
        name  = "SURREALDB_URL"
        value = var.surrealdb_url
      }

      env {
        name  = "SURREALDB_USER"
        value = var.surrealdb_user
      }

      env {
        name  = "SURREALDB_PASS"
        value = var.surrealdb_pass
      }

      env {
        name  = "TELEGRAM_TOKEN"
        value = var.telegram_token
      }

      # Add more env vars here (e.g. GEMINI_API_KEY, TELEGRAM_TOKEN)
    }
  }
}

output "service_url" {
  value = google_cloud_run_v2_service.orchestrator.uri
}
