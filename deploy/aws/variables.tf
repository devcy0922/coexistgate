variable "aws_region" {
  type    = string
  default = "us-east-1"
}

variable "name" {
  type    = string
  default = "coexistgate"
}

variable "github_app_id" {
  type = string
}

variable "github_app_private_key_path" {
  type        = string
  description = "Path to the GitHub App private key PEM"
}

variable "webhook_secret" {
  type      = string
  default   = ""
  sensitive = true
}

variable "image_uri" {
  type        = string
  description = "ECR image URI for the Lambda container (built from ../../Dockerfile.lambda)"
}

variable "lambda_arch" {
  type    = string
  default = "arm64"
}
