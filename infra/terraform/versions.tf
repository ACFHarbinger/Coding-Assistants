terraform {
  required_version = ">= 1.7"

  required_providers {
    # TODO: uncomment and pin the provider(s) your project actually needs.
    # aws = {
    #   source  = "hashicorp/aws"
    #   version = "~> 5.0"
    # }
    # google = {
    #   source  = "hashicorp/google"
    #   version = "~> 5.0"
    # }
  }

  # TODO: configure a remote state backend before using this in a team setting.
  # backend "s3" {
  #   bucket = "coding-assistants-tfstate"
  #   key    = "coding-assistants/terraform.tfstate"
  #   region = "us-east-1"
  # }
}
