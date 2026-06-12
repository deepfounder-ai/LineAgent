#!/usr/bin/env sh
# lineagent one-line installer.
#
#   curl -fsSL https://raw.githubusercontent.com/OWNER/lineagent/main/scripts/install.sh | sh
#
# What it does:
#   1. starts a lineagent container (pulls the published image, or builds from a
#      clone if the image isn't available),
#   2. waits for it to become healthy,
#   3. registers a first user and prints its API key + the dashboard URL.
#
# Override any of these with environment variables:
#   LINEAGENT_IMAGE   container image           (default: ghcr.io/deepfounder-ai/lineagent:latest)
#   LINEAGENT_PORT    host port to publish      (default: 8080)
#   LINEAGENT_USER    initial username          (default: me)
#   LINEAGENT_PASS    initial password          (default: a random 24-char string)
#   LINEAGENT_REPO    git URL for the build fallback
set -eu

IMAGE="${LINEAGENT_IMAGE:-ghcr.io/deepfounder-ai/lineagent:latest}"
PORT="${LINEAGENT_PORT:-8080}"
USER_NAME="${LINEAGENT_USER:-me}"
REPO="${LINEAGENT_REPO:-https://github.com/deepfounder-ai/LineAgent}"
NAME="lineagent"

say() { printf '\033[1;36m==>\033[0m %s\n' "$1"; }
die() { printf '\033[1;31merror:\033[0m %s\n' "$1" >&2; exit 1; }

command -v docker >/dev/null 2>&1 || die "docker is required but not installed (see https://docs.docker.com/get-docker/)"

# Random password if none supplied.
if [ -z "${LINEAGENT_PASS:-}" ]; then
  PASS="$(LC_ALL=C tr -dc 'A-Za-z0-9' </dev/urandom 2>/dev/null | head -c 24 || echo changemechangeme1234)"
else
  PASS="$LINEAGENT_PASS"
fi

# Remove a previous container with the same name (keeps the data volume).
if docker ps -a --format '{{.Names}}' | grep -qx "$NAME"; then
  say "removing existing '$NAME' container (data volume is preserved)"
  docker rm -f "$NAME" >/dev/null 2>&1 || true
fi

# Get an image: pull the published one, else build from a clone.
if ! docker image inspect "$IMAGE" >/dev/null 2>&1; then
  say "pulling $IMAGE"
  if ! docker pull "$IMAGE" >/dev/null 2>&1; then
    say "image not available; building from source ($REPO)"
    command -v git >/dev/null 2>&1 || die "git is required for the build fallback"
    TMP="$(mktemp -d)"
    git clone --depth 1 "$REPO" "$TMP/lineagent" >/dev/null 2>&1 || die "git clone failed"
    IMAGE="lineagent:local"
    docker build -t "$IMAGE" "$TMP/lineagent" || die "docker build failed"
    rm -rf "$TMP"
  fi
fi

say "starting container on port $PORT"
docker run -d \
  --name "$NAME" \
  --restart unless-stopped \
  -p "${PORT}:8080" \
  -v lineagent-data:/data \
  "$IMAGE" >/dev/null

BASE="http://127.0.0.1:${PORT}"
say "waiting for lineagent to become healthy"
i=0
until curl -fsS "${BASE}/healthz" >/dev/null 2>&1; do
  i=$((i + 1))
  [ "$i" -gt 60 ] && die "lineagent did not become healthy in time (check: docker logs $NAME)"
  sleep 1
done

say "registering user '$USER_NAME'"
RESP="$(curl -fsS -X POST "${BASE}/api/v1/auth/register" \
  -H 'Content-Type: application/json' \
  -d "{\"username\":\"${USER_NAME}\",\"password\":\"${PASS}\"}" || true)"

API_KEY="$(printf '%s' "$RESP" | sed -n 's/.*"api_key":"\([^"]*\)".*/\1/p')"

printf '\n'
say "lineagent is running"
printf '  Dashboard : %s/\n' "$BASE"
printf '  Health    : %s/healthz\n' "$BASE"
if [ -n "$API_KEY" ]; then
  printf '  Username  : %s\n' "$USER_NAME"
  printf '  Password  : %s\n' "$PASS"
  printf '  API key   : %s\n' "$API_KEY"
  printf '\nSave the API key — it is shown only once. Use it like:\n'
  printf '  export LINEAGENT_API_URL=%s\n' "$BASE"
  printf '  export LINEAGENT_API_KEY=%s\n' "$API_KEY"
else
  printf '  (user "%s" may already exist — log in manually)\n' "$USER_NAME"
fi
printf '\nStop with:  docker rm -f %s\n' "$NAME"
