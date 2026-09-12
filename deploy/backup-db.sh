#!/usr/bin/env bash
# Nightly PostgreSQL dump for the single-instance layout (deploy/lightsail.md, section 1b).
# Keeps 14 daily dumps in /var/backups/propertypilot; restore with:
#   gunzip -c renewal-YYYY-MM-DD.sql.gz | sudo -u postgres psql renewal
set -euo pipefail
DIR=/var/backups/propertypilot
mkdir -p "$DIR"
sudo -u postgres pg_dump --format=plain renewal | gzip > "$DIR/renewal-$(date +%F).sql.gz"
find "$DIR" -name 'renewal-*.sql.gz' -mtime +14 -delete
