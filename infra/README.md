# Actualised.ai Infrastructure

This directory contains the Terraform configuration to provision the Google Cloud Run environment for the Actualised.ai orchestrator.

## Prerequisites

- Terraform CLI installed.
- `gcloud` CLI installed and authenticated (`gcloud auth application-default login`).
- A GCP Project with billing enabled and the Cloud Run API enabled.

## Deployment Instructions

1. **Initialize Terraform:**
   ```bash
   terraform init
   ```

2. **Create a `terraform.tfvars` file:**
   Create a file named `terraform.tfvars` in this directory (it is ignored by git) and populate it with your specific values:
   ```hcl
   project_id         = "your-gcp-project-id"
   region             = "us-central1"
   orchestrator_image = "gcr.io/your-gcp-project-id/actualised-orchestrator:latest"
   surrealdb_url      = "wss://your-surreal-cloud-instance.surreal.cloud/rpc"
   surrealdb_user     = "your-db-user"
   surrealdb_pass     = "your-db-password"
   ```

3. **Plan the Deployment:**
   ```bash
   terraform plan
   ```

4. **Apply the Deployment:**
   ```bash
   terraform apply
   ```

## CI/CD Integration

The Engineering Manager agent is responsible for updating this infrastructure. In the future, this directory can be integrated with GitHub Actions to automatically apply infrastructure changes when merged to `main`.
