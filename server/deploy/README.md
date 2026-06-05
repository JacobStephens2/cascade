# Deploying cascade-sync-server (Infrastructure as Code)

This directory codifies the deploy of the Cascade listening-time sync service —
a real, load-bearing service on an existing DigitalOcean droplet — as
**Terraform** (the DNS wiring) plus **Ansible** (the host config). The point is
reproducibility: the service can be rebuilt from text, reviewed in a PR, and
audited, instead of living as undocumented steps in someone's shell history.

It is written to be **safe against live infrastructure**: Terraform only
*creates* the new DNS record and references the existing droplet read-only, so an
apply can't recreate or destroy anything load-bearing. Ansible is idempotent and
converges to the declared state.

## Layout

```
terraform/   DNS record for sync.cascade.stephens.page -> existing droplet
ansible/     role `cascade_sync`: postgres role/db, systemd unit (hardened),
             Apache reverse-proxy vhost, certbot TLS, env file
```

## What it provisions

- A DigitalOcean **DNS A record** for the sync subdomain (Terraform).
- A dedicated **`cascade_sync` Postgres role + database** on the existing
  Postgres instance (Ansible).
- A **systemd service** running the release binary as an unprivileged
  `cascade-sync` user, with filesystem/syscall hardening (`ProtectSystem=strict`,
  empty `CapabilityBoundingSet`, `MemoryDenyWriteExecute`, …) — the runtime half
  of the threat model in [`../docs/threat-model.md`](../docs/threat-model.md).
- An **Apache reverse-proxy vhost** that proxies 443→localhost and explicitly
  refuses to expose `/metrics`.
- A **Let's Encrypt certificate** via `certbot --apache` (idempotent).

## Prerequisites

- A DigitalOcean API token (`export TF_VAR_do_token=...`).
- Ansible + collections: `ansible-galaxy collection install -r ansible/requirements.yml`,
  and `psycopg2` on the target host for the Postgres modules.
- A built release binary staged at
  `ansible/roles/cascade_sync/files/cascade-sync-server`
  (CI uploads it — see [`.github/workflows/deploy-sync.yml`](../../.github/workflows/deploy-sync.yml)).

## Apply

```bash
# 1. DNS
cd terraform
cp terraform.tfvars.example terraform.tfvars   # set droplet_name
terraform init && terraform plan && terraform apply

# 2. Host config (secrets live in an encrypted vault)
cd ../ansible
cp inventory.example.ini inventory.ini                       # set droplet IP
cp group_vars/all/vault.example.yml group_vars/all/vault.yml # fill in, then:
ansible-vault encrypt group_vars/all/vault.yml
ansible-playbook -i inventory.ini playbook.yml --ask-vault-pass
```

## Codifying the rest of the fleet

This service is the first slice. The existing droplet, volume, and firewall are
referenced read-only; bring them under management incrementally with
`terraform import` (examples in `terraform/main.tf`) rather than a blind apply —
match the live config first, then evolve. A sanitized copy of this module is
mirrored to [`infrastructure-patterns`](https://github.com/JacobStephens2/infrastructure-patterns).

## Secrets

Nothing secret is committed. Terraform reads the DO token from the environment;
Ansible reads the DB password and SMTP creds from an `ansible-vault`-encrypted
`group_vars/all/vault.yml`. `deploy/.gitignore` blocks tfstate, tfvars, the real
inventory, the vault file, and the staged binary.
