variable "do_token" {
  description = "DigitalOcean API token (provide via TF_VAR_do_token or a tfvars file kept out of git)."
  type        = string
  sensitive   = true
}

variable "domain" {
  description = "The apex domain managed in DigitalOcean DNS."
  type        = string
  default     = "stephens.page"
}

variable "record_name" {
  description = "Subdomain record under `domain` for the sync API (host = record_name.domain)."
  type        = string
  default     = "sync.cascade"
}

variable "droplet_name" {
  description = "Name of the existing application droplet to point the record at."
  type        = string
}
