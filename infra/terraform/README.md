# terraform/

Minimal Terraform skeleton for provisioning cloud resources this project
might eventually depend on (a container registry for `infra/docker/`, a
future cloud synchronization resources. No provider is wired up yet, and
nothing here is required for the desktop/Android app itself — this is a
starting point for if/when a hosted service is added.

```bash
cd infra/terraform
terraform init
terraform plan -var-file=environments/dev.tfvars
terraform apply -var-file=environments/dev.tfvars
```

| File | Purpose |
| --- | --- |
| `versions.tf` | Required Terraform + provider versions, remote state backend (commented, fill in before first `init`) |
| `variables.tf` | Input variables |
| `main.tf` | Resources — currently empty, add your provider blocks and resources here |
| `outputs.tf` | Values to surface after `apply` (e.g. registry URL, cluster endpoint) |
| `environments/*.tfvars` | Per-environment variable values |

> **TODO:** Pick a cloud provider, uncomment/configure the matching provider
> block in `versions.tf`, and replace the placeholder resources in `main.tf`.
