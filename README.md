
## Notes

Currently output metrics that looks like this:

```
# HELP eks_version_exporter Bunch of values
# TYPE eks_version_exporter gauge
eks_version_exporter{eks_latest_available_version="1.18.0",last_updated="1603289688143",last_updated_text="2020-10-21 14:14:48.143734000",server_current_version="1.15.0"} 0
```

## Kubernetes Deployment

Deploy with the manifests in `k8s-flux`:

```bash
kubectl apply -f k8s-flux/
```

Update the image tag in `k8s-flux/deployment.yaml` before applying if you are deploying manually.