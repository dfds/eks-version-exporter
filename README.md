
## Notes

Currently output metrics that looks like this:

```
# HELP eks_version_exporter Bunch of values
# TYPE eks_version_exporter gauge
eks_version_exporter{eks_latest_available_version="1.18.0",last_updated="1603289688143",last_updated_text="2020-10-21 14:14:48.143734000",server_current_version="1.15.0"} 0
```

## Helm

The repository includes a Helm chart at `chart/eks-version-exporter`.

Render manifests locally:

```bash
helm template eks-version-exporter chart/eks-version-exporter
```

Install or upgrade in a namespace:

```bash
helm upgrade --install eks-version-exporter chart/eks-version-exporter \
	--namespace <namespace> \
	--create-namespace
```

Override image tag at deploy time:

```bash
helm upgrade --install eks-version-exporter chart/eks-version-exporter \
	--namespace <namespace> \
	--set image.tag=<tag>
```