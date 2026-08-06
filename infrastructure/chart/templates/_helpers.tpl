{{/* Common labels applied to every rendered resource. */}}
{{- define "routers.labels" -}}
app.kubernetes.io/name: {{ .Chart.Name }}
app.kubernetes.io/instance: {{ .Release.Name }}
app.kubernetes.io/managed-by: {{ .Release.Service }}
helm.sh/chart: {{ printf "%s-%s" .Chart.Name .Chart.Version | replace "+" "_" }}
{{- end }}

{{/* Fail with a clear message if required infra endpoints are missing. */}}
{{- define "routers.requireInfra" -}}
{{- if not .Values.infra.nats.url -}}
{{- fail "infra.nats.url is required; supply a values overlay (e.g. -f values-local-dev.yaml)" -}}
{{- end -}}
{{- if not .Values.infra.valkey.url -}}
{{- fail "infra.valkey.url is required; supply a values overlay (e.g. -f values-local-dev.yaml)" -}}
{{- end -}}
{{- end }}

{{/*
Chart-level guard, included by both workload templates.

A shard string whose length disagrees with `shardPrecision` is the one
misconfiguration nothing downstream can survive: the matcher would load one
extent and serve the subject of another, and every orchestrator routing to
that subject would time out against a matcher that cannot answer.
*/}}
{{- define "routers.validate" -}}
{{- range $shard := .Values.shards -}}
{{- if ne (len $shard) (int $.Values.shardPrecision) -}}
{{- fail (printf "shard %q is %d characters but shardPrecision is %d. The matcher would load a different extent from the one its request subject covers." $shard (len $shard) (int $.Values.shardPrecision)) -}}
{{- end -}}
{{- end -}}
{{- include "routers.requireInfra" . -}}
{{- end }}

{{/*
Fully qualified image reference. `image.registry` is prepended when set, so
the same service-level repository/tag pair resolves against Artifact Registry
in GKE and against the local daemon in dev with no per-service duplication.
Call with (dict "registry" $.Values.image.registry "image" $.Values.<svc>.image).
*/}}
{{- define "routers.image" -}}
{{- $registry := .registry | default "" -}}
{{- if $registry -}}
{{- printf "%s/%s:%s" (trimSuffix "/" $registry) .image.repository (.image.tag | toString) -}}
{{- else -}}
{{- printf "%s:%s" .image.repository (.image.tag | toString) -}}
{{- end -}}
{{- end }}

{{/*
Name of the ServiceAccount the pods run as. Empty means "do not set
serviceAccountName", which leaves the namespace default in place and keeps
local dev identical to before Workload Identity existed in this chart.
*/}}
{{- define "routers.serviceAccountName" -}}
{{- if .Values.serviceAccount.create -}}
{{- .Values.serviceAccount.name | default .Chart.Name -}}
{{- else -}}
{{- .Values.serviceAccount.name | default "" -}}
{{- end -}}
{{- end }}

{{/*
Pod-spec fields identical in shape for the matcher and the orchestrator:
identity, pull secrets and placement. Held in one place so a new scheduling
knob is added once rather than twice.
Call with (dict "root" $ "service" $.Values.<svc>).
*/}}
{{- define "routers.podSpecCommon" -}}
{{- $sa := include "routers.serviceAccountName" .root }}
{{- if $sa }}
serviceAccountName: {{ $sa }}
{{- end }}
{{- with .root.Values.imagePullSecrets }}
imagePullSecrets:
  {{- toYaml . | nindent 2 }}
{{- end }}
{{- with .service.nodeSelector }}
nodeSelector:
  {{- toYaml . | nindent 2 }}
{{- end }}
{{- with .service.tolerations }}
tolerations:
  {{- toYaml . | nindent 2 }}
{{- end }}
{{- end }}
