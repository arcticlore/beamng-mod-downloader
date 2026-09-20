#!/usr/bin/env bash
# Release gate: ожидает и проверяет успешные workflow runs для exact commit SHA.
# Используется release.yml (workflow_dispatch): публикация разрешена только когда
# build, CodeQL и реальный OSV scan прошли ровно на этом SHA.
#
# Применение: bash scripts/wait-for-checks.sh <sha>
#   GH_TOKEN (или gh auth) должен быть доступен; в Actions — GITHUB_TOKEN.
#   GITHUB_REPOSITORY — owner/repo; иначе берётся из git remote.
#
# Семантика:
#   - проверяет workflow runs по exact head_sha (не по UI-имени check);
#   - требует conclusion 'success' для каждого обязательного workflow;
#   - для build дополнительно проверяет ВСЕ обязательные jobs и ВСЕ matrix bundle jobs;
#   - различает missing / queued|in_progress / failed / api_error и сообщает состояние;
#   - при API/JSON ошибке завершается non-zero (никогда не «пустой success»);
#   - bounded timeout + backoff.

set -euo pipefail

SHA="${1:?usage: wait-for-checks.sh <sha>}"
TIMEOUT_S="${TIMEOUT_S:-1500}"
POLL_S="${POLL_S:-30}"

REPO="${GITHUB_REPOSITORY:-$(git remote get-url origin | sed -E 's#.*[:/]([^/]+/[^/]+)(\.git)?$#\1#')}"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

# Обязательные workflow (PATH в .github/workflows). UI-имя НЕ используется.
REQUIRED_WORKFLOWS=(build.yml codeql.yml osv-scanner.yml)

# Jobs build.yml, которые обязаны быть success (без matrix bundle-jobs).
printf '%s\n' fmt test clippy tsc frontend-build frontend-test npm-audit \
  cargo-deny version-consistency tauri-config packaging-static > "$WORK/required_jobs.txt"

MIN_BUNDLE_JOBS=3

deadline=$(( $(date +%s) + TIMEOUT_S ))

# --- Fetch: API → JSON-файл. {"api_error":true} при ошибке ------------------

fetch_runs() {
  local file="$1" out
  if out=$(gh api "repos/$REPO/actions/workflows/$file/runs?head_sha=$SHA&per_page=100" \
      --jq '[.workflow_runs[] | {id, name, status, conclusion: (.conclusion // null), head_sha, created_at}]' 2>/dev/null); then
    printf '%s' "$out" > "$WORK/runs_$file.json"
  else
    printf '{"api_error": true}' > "$WORK/runs_$file.json"
  fi
}

fetch_jobs() {
  local run_id="$1" out
  if out=$(gh api "repos/$REPO/actions/runs/$run_id/jobs?per_page=100" \
      --jq '[.jobs[] | {name, status, conclusion: (.conclusion // null)}]' 2>/dev/null); then
    printf '%s' "$out" > "$WORK/jobs.json"
  else
    printf '{"api_error": true}' > "$WORK/jobs.json"
  fi
}

# --- Оценка: один Python-вызов читает $WORK ---------------------------------
# Печатает ДВЕ строки: <status> затем <run_id> (пусто, если нет).

eval_run_status() {
  local wf="$1"
  python3 - "$SHA" "$WORK" "$wf" <<'PY'
import json, os, sys
sha, work, wf = sys.argv[1], sys.argv[2], sys.argv[3]

def load(p):
    with open(p) as f:
        return json.load(f)

runs = load(os.path.join(work, f"runs_{wf}.json"))
if isinstance(runs, dict) and runs.get("api_error"):
    print("apierror"); print(""); sys.exit(0)
cand = [r for r in runs if r.get("head_sha") == sha]
done = [r for r in cand if r.get("status") == "completed"]
if not done:
    print("wait" if cand else "missing"); print(""); sys.exit(0)
latest = max(done, key=lambda r: r.get("created_at") or "")
if latest["conclusion"] != "success":
    print(latest["conclusion"] or "unknown"); print(""); sys.exit(0)
print("ok"); print(latest["id"])
PY
}

# Оценка jobs build run (читает $WORK/jobs.json). Печатает ok|apierror|details.
eval_build_jobs() {
  local min_bundles="$1"
  python3 - "$min_bundles" "$WORK" <<'PY'
import json, os, sys
min_bundles, work = int(sys.argv[1]), sys.argv[2]

def load(p):
    with open(p) as f:
        return json.load(f)

with open(os.path.join(work, "required_jobs.txt")) as f:
    needed = [l.strip() for l in f if l.strip()]
jobs = load(os.path.join(work, "jobs.json"))
if isinstance(jobs, dict) and jobs.get("api_error"):
    print("apierror"); sys.exit(0)
byname = {j["name"]: j for j in jobs}
miss = [n for n in needed if n not in byname]
if miss:
    print("missing_jobs:" + ",".join(miss)); sys.exit(0)
bad = [n for n in needed if byname[n]["conclusion"] != "success"]
if bad:
    print("failed_jobs:" + ",".join(bad)); sys.exit(0)
bundles = [j for j in jobs if j["name"].startswith("bundle")]
if len(bundles) < min_bundles:
    print(f"missing_bundle_jobs:{len(bundles)}<{min_bundles}"); sys.exit(0)
bbad = [j["name"] for j in bundles if j["conclusion"] != "success"]
if bbad:
    print("failed_bundle_jobs:" + ",".join(bbad)); sys.exit(0)
print("ok")
PY
}

# --- Main loop --------------------------------------------------------------

while :; do
  echo "=== gate: ожидаем workflow runs для $SHA (repo: $REPO) ==="
  pending=0
  failed=0

  for wf in "${REQUIRED_WORKFLOWS[@]}"; do
    fetch_runs "$wf"
    out="$(eval_run_status "$wf")"
    st="$(printf '%s\n' "$out" | sed -n 1p)"
    rid="$(printf '%s\n' "$out" | sed -n 2p)"

    if [ "$wf" = "build.yml" ] && [ "$st" = "ok" ] && [ -n "$rid" ]; then
      fetch_jobs "$rid"
      jobs_st="$(eval_build_jobs "$MIN_BUNDLE_JOBS")"
      if [ "$jobs_st" = "ok" ]; then
        st="ok"
      elif [ "$jobs_st" = "apierror" ]; then
        st="error"
        echo "  ERROR build (jobs API error)"
        failed=1
        continue
      else
        echo "  FAIL  build (jobs: $jobs_st)"
        failed=1
        continue
      fi
    fi

    case "$st" in
      ok)       echo "  OK    $wf" ;;
      wait)     echo "  WAIT  $wf (in_progress/queued)"; pending=$((pending+1)) ;;
      missing)  echo "  WAIT  $wf (нет workflow run на этом SHA)"; pending=$((pending+1)) ;;
      error|apierror) echo "  ERROR $wf (API/JSON error)"; failed=1 ;;
      *)        echo "  FAIL  $wf (conclusion=$st)"; failed=1 ;;
    esac
  done

  if [ "$failed" -gt 0 ]; then
    echo "Gate: обязательные workflow завершились неуспешно или API error. Release отменяется."
    exit 1
  fi
  if [ "$pending" -eq 0 ]; then
    echo "Gate: все обязательные workflow (build + CodeQL + OSV) success на $SHA. Gate пройден."
    exit 0
  fi
  if [ "$(date +%s)" -ge "$deadline" ]; then
    echo "Gate: таймаут ожидания ($TIMEOUT_S с). Проверьте статус commit $SHA вручную."
    exit 1
  fi
  echo "  повтор через ${POLL_S}s (осталось $(($deadline - $(date +%s)))s)"
  sleep "$POLL_S"
done