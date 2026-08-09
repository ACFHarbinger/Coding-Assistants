# Resources go here once a provider is configured in versions.tf.
#
# Example shape once you add one (illustrative only, not applied):
#
# resource "aws_ecr_repository" "app" {
#   name = "${var.project_name}-${var.environment}"
# }

locals {
  name_prefix = "${var.project_name}-${var.environment}"
}
