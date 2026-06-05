# Infrastructure for the Cascade listening-time sync service.
#
# Scope is deliberately *additive and non-destructive*: the only resource this
# creates is the DNS record for the new subdomain. The droplet, volume, and
# firewall it depends on already exist and are referenced as read-only data
# sources, so `terraform apply` here cannot recreate or destroy load-bearing
# infrastructure. The commented blocks at the bottom show how those existing
# resources would be brought under management via `terraform import` when you
# codify the rest of the fleet (see README).

terraform {
  required_version = ">= 1.5"
  required_providers {
    digitalocean = {
      source  = "digitalocean/digitalocean"
      version = "~> 2.40"
    }
  }
}

provider "digitalocean" {
  token = var.do_token
}

# The existing application droplet (read-only reference).
data "digitalocean_droplet" "web" {
  name = var.droplet_name
}

# The new subdomain for the sync API → the existing droplet. Apache on the
# droplet reverse-proxies 443 to the service on localhost (see the Ansible
# vhost template), so no new public port is opened.
resource "digitalocean_record" "sync" {
  domain = var.domain
  type   = "A"
  name   = var.record_name
  value  = data.digitalocean_droplet.web.ipv4_address
  ttl    = 300
}

# ---------------------------------------------------------------------------
# Codifying the rest of the fleet (do this incrementally with `import`, never a
# blind apply). Example for the existing droplet:
#
#   terraform import digitalocean_droplet.web <droplet-id>
#
# resource "digitalocean_droplet" "web" {
#   name     = var.droplet_name
#   region   = "nyc3"
#   size     = "s-2vcpu-4gb"
#   image    = "rockylinux-9-x64"
#   # ...match the live config exactly before the first apply, then evolve.
# }
#
# resource "digitalocean_volume" "data" {
#   name   = "volume-nyc3-01"
#   region = "nyc3"
#   size   = 100
# }
# ---------------------------------------------------------------------------
