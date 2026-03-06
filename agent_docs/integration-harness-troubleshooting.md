# Integration Harness Troubleshooting

## Docker access errors

Symptom:

- `harnessctl error: docker daemon is not reachable`

Checks:

- `systemctl is-active docker`
- `id` (confirm user is in `docker` group)
- `ls -l /var/run/docker.sock`

Fixes:

- start daemon: `sudo systemctl enable --now docker`
- add user to group: `sudo usermod -aG docker $USER`
- refresh shell session: `newgrp docker` (or log out/in)

## Slow or huge Docker build context

Symptom:

- `Sending build context to Docker daemon ...` climbs very large

Checks:

- verify root `.dockerignore` exists and excludes large directories
- inspect repo size hotspots with `du -h --max-depth=2 . | sort -hr`

Fixes:

- exclude heavy paths from context (`target/`, `HeavenClient/`, etc.)
- avoid copying large runtime assets into image layers

## Buildx / BuildKit not active

Symptom:

- warning about missing buildx plugin
- no effective cache reuse during image builds

Checks:

- `docker buildx version`

Fixes:

- install buildx plugin for the host distro
- keep BuildKit cache mounts in `Dockerfile.rustms-server`

## Migration service exits early

Symptom:

- compose reports `service "migrate" didn't complete successfully`

Checks:

- `docker compose ... logs migrate`
- `docker compose ... config` to verify resolved `entrypoint`/`command`

Fixes:

- ensure migration loop command is passed as a single shell script string
- ensure migrations are mounted read-only at `/migrations`

## Server starts then disconnects clients immediately

Symptom:

- integration tests fail with `UnexpectedEof` during login/world flow

Checks:

- `docker compose ... logs login`
- `docker compose ... logs world`

Likely causes and fixes:

- missing DB schema (`relation ... does not exist`): fix migration runner and restart stack
- missing runtime DB libs (`libpq.so.5`): ensure runtime image includes `libpq`
- missing game assets (`Failed to load game data`): ensure world mounts `../assets:/app/assets:ro`

## Quick diagnostic bundle

When a run fails, collect these first:

- `docker compose -f integration-harness/docker-compose.test.yml -p rustms-integration ps`
- `docker compose -f integration-harness/docker-compose.test.yml -p rustms-integration logs --no-color --tail=200 login`
- `docker compose -f integration-harness/docker-compose.test.yml -p rustms-integration logs --no-color --tail=200 world`
- `docker compose -f integration-harness/docker-compose.test.yml -p rustms-integration logs --no-color --tail=200 migrate`
