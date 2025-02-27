{{/*
Expand the name of the chart.
*/}}
{{- define "geospatial-api.name" -}}
{{- default .Chart.Name .Values.nameOverride | trunc 63 | trimSuffix "-" }}
{{- end }}

{{/*
Create a default fully qualified app name.
*/}}
{{- define "geospatial-api.fullname" -}}
{{- if .Values.fullnameOverride }}
{{- .Values.fullnameOverride | trunc 63 | trimSuffix "-" }}
{{- else }}
{{- $name := default .Chart.Name .Values.nameOverride }}
{{- if contains $name .Release.Name }}
{{- .Release.Name | trunc 63 | trimSuffix "-" }}
{{- else }}
{{- printf "%s-%s" .Release.Name $name | trunc 63 | trimSuffix "-" }}
{{- end }}
{{- end }}
{{- end }}

{{/*
Common labels
*/}}
{{- define "geospatial-api.labels" -}}
helm.sh/chart: {{ include "geospatial-api.chart" . }}
{{ include "geospatial-api.selectorLabels" . }}
app.kubernetes.io/version: {{ .Chart.AppVersion | quote }}
app.kubernetes.io/managed-by: {{ .Release.Service }}
{{- end }}

{{/*
Selector labels
*/}}
{{- define "geospatial-api.selectorLabels" -}}
app.kubernetes.io/name: {{ include "geospatial-api.name" . }}
app.kubernetes.io/instance: {{ .Release.Name }}
{{- end }}

{{/*
Create the name of the service account to use
*/}}
{{- define "geospatial-api.serviceAccountName" -}}
{{- if .Values.serviceAccount.create }}
{{- default (include "geospatial-api.fullname" .) .Values.serviceAccount.name }}
{{- else }}
{{- default "default" .Values.serviceAccount.name }}
{{- end }}
{{- end }}

{{/*
Create chart name and version as used by the chart label.
*/}}
{{- define "geospatial-api.chart" -}}
{{- printf "%s-%s" .Chart.Name .Chart.Version | replace "+" "_" | trunc 63 | trimSuffix "-" }}
{{- end }}

{{/*
Redis secret name
*/}}
{{- define "geospatial-api.redis.secretName" -}}
{{- printf "%s-%s" .Release.Name "redis" }}
{{- end }}

{{/*
PostgreSQL fullname
*/}}
{{- define "geospatial-api.postgresql.fullname" -}}
{{- printf "%s-%s" .Release.Name "postgresql" }}
{{- end }}

{{/*
PostgreSQL secret name
*/}}
{{- define "geospatial-api.postgresql.secretName" -}}
{{- printf "%s-%s" .Release.Name "postgresql" }}
{{- end }}

{{/*
Redis fullname
*/}}
{{- define "geospatial-api.redis.fullname" -}}
{{- printf "%s-%s" .Release.Name "redis" }}
{{- end }} 