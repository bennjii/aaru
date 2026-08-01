{{/* Common labels applied to every rendered resource. */}}
{{- define "routers.labels" -}}
app.kubernetes.io/name: {{ .Chart.Name }}
app.kubernetes.io/instance: {{ .Release.Name }}
app.kubernetes.io/managed-by: {{ .Release.Service }}
helm.sh/chart: {{ printf "%s-%s" .Chart.Name .Chart.Version | replace "+" "_" }}
{{- end }}

{{/*
The cell token of a shard: its leading `cellPrecision` geohash characters.
Call with (dict "root" $ "shard" $shard).
*/}}
{{- define "routers.cell" -}}
{{- substr 0 (int .root.Values.cellPrecision) .shard -}}
{{- end }}

{{/*
The shard's geohash past the cell prefix, which is the second subject token.
Call with (dict "root" $ "shard" $shard).
*/}}
{{- define "routers.shardSuffix" -}}
{{- substr (int .root.Values.cellPrecision) (len .shard) .shard -}}
{{- end }}

{{/*
Distinct cells across .Values.shards, as a JSON array. Callers decode with
`fromJsonArray`; Helm templates cannot return a list directly.
*/}}
{{- define "routers.cells" -}}
{{- $cells := list -}}
{{- range $shard := .Values.shards -}}
{{- $cells = append $cells (include "routers.cell" (dict "root" $ "shard" $shard)) -}}
{{- end -}}
{{- toJson ($cells | uniq | sortAlpha) -}}
{{- end }}

{{/*
A shard's subject for a given verb: `<prefix>.<verb>.<cell>.<rest>`.
Call with (dict "root" $ "shard" $shard "verb" "position").
*/}}
{{- define "routers.subject" -}}
{{- $cell := include "routers.cell" (dict "root" .root "shard" .shard) -}}
{{- $rest := include "routers.shardSuffix" (dict "root" .root "shard" .shard) -}}
{{- printf "%s.%s.%s.%s" .root.Values.subjectPrefix .verb $cell $rest -}}
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
Chart-level guard, included by every workload template so it fires whether or
not the historian is enabled. Ordered so a mis-scaled historian is reported as
itself rather than as a missing URL.
*/}}
{{- define "routers.validate" -}}

{{- if not (has .Values.historian.mode (list "per-cell" "per-shard" "global")) -}}
{{- fail (printf "historian.mode must be 'per-cell', 'per-shard' or 'global', got %q" .Values.historian.mode) -}}
{{- end -}}

{{/*
The subject scheme depends on the geohash splitting into two non-empty tokens.
Equal precisions would render `<cell>.` with an empty second token, which
matches nothing.
*/}}
{{- if ge (int .Values.cellPrecision) (int .Values.shardPrecision) -}}
{{- fail (printf "cellPrecision (%d) must be below shardPrecision (%d), or the shard token of the subject would be empty." (int .Values.cellPrecision) (int .Values.shardPrecision)) -}}
{{- end -}}

{{- range $shard := .Values.shards -}}
{{- if ne (len $shard) (int $.Values.shardPrecision) -}}
{{- fail (printf "shard %q is %d characters but shardPrecision is %d. The matcher would load a different extent from the one its subject covers." $shard (len $shard) (int $.Values.shardPrecision)) -}}
{{- end -}}
{{- end -}}

{{- if and (gt (int .Values.historian.replicas) 1) (not .Values.historian.queueGroup) -}}
{{- fail (printf "historian.replicas is %d with historian.queueGroup=false, which cannot work. Each replica holds a plain ephemeral subscription, so all of them receive every event and each issues its own XADD. The duplicates consume the MAXLEN budget (historian.env.HISTORY) and evict the real history the orchestrator reads back, silently shortening the resume overlap. Set historian.queueGroup=true so replicas share the subject's deliveries, or leave replicas at 1 and partition further with historian.mode=per-shard." (int .Values.historian.replicas)) -}}
{{- end -}}

{{- include "routers.requireInfra" . -}}
{{- end }}

{{/*
Fully qualified image reference. `image.registry` is prepended when set so the
same service-level repository/tag pair resolves against Artifact Registry in
GKE and against the local daemon in dev, with no per-service duplication.
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
{{- .Values.serviceAccount.name | default (printf "%s" .Chart.Name) -}}
{{- else -}}
{{- .Values.serviceAccount.name | default "" -}}
{{- end -}}
{{- end }}

{{/*
Sampler environment shared by all three binaries. See the telemetry block in
values.yaml for the caveat about whether the Rust SDK honours these.
*/}}
{{- define "routers.samplerEnv" -}}
{{- with .Values.telemetry.sampleRatio }}
- name: OTEL_TRACES_SAMPLER
  value: "parentbased_traceidratio"
- name: OTEL_TRACES_SAMPLER_ARG
  value: {{ . | toString | quote }}
{{- end }}
{{- end }}

{{/*
Pod-spec fields that are identical in shape for the matcher, orchestrator and
historian: identity, pull secrets and placement. Held in one place so a new
scheduling knob is added once rather than three times.
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
{{- with .service.priorityClassName }}
priorityClassName: {{ . }}
{{- end }}
{{- with .service.nodeSelector }}
nodeSelector:
  {{- toYaml . | nindent 2 }}
{{- end }}
{{- with .service.tolerations }}
tolerations:
  {{- toYaml . | nindent 2 }}
{{- end }}
{{- with .service.affinity }}
affinity:
  {{- toYaml . | nindent 2 }}
{{- end }}
{{- with .service.topologySpreadConstraints }}
topologySpreadConstraints:
  {{- toYaml . | nindent 2 }}
{{- end }}
{{- end }}

{{/*
Pod template annotations for a service. The GCS FUSE CSI driver only injects
its sidecar when the pod carries `gke-gcsfuse/volumes: "true"`, so the shard
cache mode decides part of this map; without the annotation the pod stays
Pending with an unmountable volume.
Call with (dict "root" $ "service" $.Values.<svc> "gcsfuse" true|false).
*/}}
{{- define "routers.podAnnotations" -}}
{{- $ann := .service.podAnnotations | default dict }}
{{- if and .gcsfuse (eq .root.Values.shardCache.mode "gcsfuse") }}
{{- $ann = merge (dict "gke-gcsfuse/volumes" "true") $ann }}
{{- end }}
{{- with $ann }}
{{- toYaml . }}
{{- end }}
{{- end }}
