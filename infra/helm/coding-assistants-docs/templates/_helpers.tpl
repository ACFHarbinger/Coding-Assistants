{{- define "coding-assistants-docs.name" -}}
{{- .Chart.Name -}}
{{- end -}}

{{- define "coding-assistants-docs.labels" -}}
app: {{ include "coding-assistants-docs.name" . }}
{{- end -}}
