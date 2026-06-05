output "sync_fqdn" {
  description = "Fully-qualified hostname for the sync API."
  value       = "${var.record_name}.${var.domain}"
}

output "droplet_ip" {
  description = "Public IPv4 the record resolves to."
  value       = data.digitalocean_droplet.web.ipv4_address
}
