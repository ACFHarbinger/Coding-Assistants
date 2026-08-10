# Infrastructure

The infrastructure tree is intentionally flat and limited to retained use
cases:

| Directory | Purpose |
| --- | --- |
| [`docker/`](docker/) | Local documentation-site container and Compose files |
| [`terraform/`](terraform/) | Future cloud/device-synchronization prototypes |
| [`ansible/`](ansible/) | Reproducible host configuration |
| [`firebase/`](firebase/) | Cloud synchronization prototypes |

Unused Kubernetes, Helm, serverless, AWS, Azure Pipelines, WordPress, Webpack,
Nginx, and proxy scaffolding has been removed from the active tree.
