# k8s/

Kustomize-based Kubernetes manifests: a `base/` layer plus per-environment
`overlays/`. Prefer this over hand-maintaining near-duplicate YAML per
environment.

```bash
kubectl apply -k infra/global/k8s/overlays/dev
kubectl apply -k infra/global/k8s/overlays/prod
```

| Directory | Purpose |
| --- | --- |
| `base/` | Environment-agnostic Deployment/Service/ConfigMap/Ingress |
| `overlays/dev/` | Dev patches: fewer replicas, `imagePullPolicy: Always`, dev host |
| `overlays/prod/` | Prod patches: replica count, resource limits, prod host |

These manifests deploy the `docs/website/` static site (see
`infra/global/docker/`) — this repo has no other hosted service today.

> **TODO:** Point the `image:` field in `base/deployment.yaml` at your real
> container registry once one exists. The Helm chart in `infra/global/helm/`
> packages the same base manifests for teams that prefer `helm install`
> over `kubectl apply -k`.
