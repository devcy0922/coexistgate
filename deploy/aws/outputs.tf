output "webhook_url" {
  description = "Set this as the GitHub App webhook URL"
  value       = "${aws_apigatewayv2_api.this.api_endpoint}/webhook"
}

output "secret_arn" {
  value = aws_secretsmanager_secret.this.arn
}

output "lambda_name" {
  value = aws_lambda_function.this.function_name
}

output "github_settings" {
  value = {
    webhook_url     = "${aws_apigatewayv2_api.this.api_endpoint}/webhook"
    webhook_secret  = "see Secrets Manager ${aws_secretsmanager_secret.this.name} key webhook_secret (or terraform output webhook_secret)"
    required_events = ["pull_request"]
    required_perms = {
      checks         = "write"
      contents       = "read"
      pull_requests  = "read"
    }
  }
}

output "webhook_secret" {
  description = "GitHub App webhook secret"
  value       = local.webhook_secret
  sensitive   = true
}
