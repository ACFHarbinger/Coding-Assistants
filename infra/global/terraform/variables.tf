variable "environment" {
  description = "Deployment environment name (dev, staging, prod)."
  type        = string
  default     = "dev"
}

variable "project_name" {
  description = "Short name used to prefix provisioned resources."
  type        = string
  default     = "coding-assistants"
}

variable "region" {
  description = "Cloud provider region."
  type        = string
  default     = "us-east-1"
}
