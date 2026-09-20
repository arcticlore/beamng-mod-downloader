#!/usr/bin/env bash
# Fixture-based remote tests for scripts/wait-for-checks.sh.
#
# Создаёт фейковый `gh`, который обслуживает JSON из scripts/fixtures/gh/<scenario>
# по шаблону URL реальной GitHub Actions API:
#   repos/$REPO/actions/workflows/<file>/runs?head_sha=...&per_page=...
#   repos/$REPO/actions/runs/<id>/jobs?per_page=...
#
# Соглашения fixtures/<scenario>:
#   runs_<file>.N.json  — ответ workflow runs-эндпоинта на N-й вызов (1-индекс);
#                         если файла .N нет, используется runs_<file>.1.json.
#   runs_<file>.FORCE_ERROR — эндпоинт возвращает non-zero (API error).
#   jobs.N.json         — ответ jobs-эндпоинта аналогично.
#
# Запуск: bash scripts/test-wait-for-checks.sh
set -euo pipefail

SCRIPTS="$(cd "$(dirname "$0")" && pwd)"
FIXTURES="$SCRIPTS/fixtures/gh"
SHA=1111111111111111111111111111111111111111
REPO=acme/demo
PASS=0
FAIL=0

run_case() {
  local name="$1" expected="$2"
  local work scenario err
  work="$(mktemp -d)"
  scenario="$FIXTURES/$name"
  err="$(mktemp)"

  # --- fake gh ---
  cat > "$work/gh" <<SH
#!/usr/bin/env bash
set -euo pipefail
scenario="\$FAKE_SCENARIO"
url=""
for a in "\$@"; do
  case "\$a" in
    --jq) break ;;
    repos/*) url="\$a" ;;
    *) ;;
  esac
done

# endpoint: runs or jobs
endpoint=""
stepfile=""
if [[ "\$url" =~ /workflows/([^/]+)/runs ]]; then
  endpoint="runs_\${BASH_REMATCH[1]%.yml}"
elif [[ "\$url" =~ /runs/([0-9]+)/jobs ]]; then
  endpoint="jobs"
else
  echo "unexpected URL: \$url" >&2
  exit 2
fi

# decide step index (counter в изолированном per-run каталоге)
counter="\$FAKE_COUNTER_DIR/\$endpoint.counter"
step=1
[ -f "\$counter" ] && step=\$(cat "\$counter")
echo \$((step+1)) > "\$counter"

# forced API error
if [ -f "\$scenario/\$endpoint.FORCE_ERROR" ]; then
  echo "simulated API error for \$endpoint (call \$step)" >&2
  exit 1
fi

# serve file: prefer .<step>, else .1
for f in "\$scenario/\$endpoint."\$step".json" "\$scenario/\$endpoint.1.json"; do
  if [ -f "\$f" ]; then
    cat "\$f"
    exit 0
  fi
done
echo "no fixture for \$endpoint step \$step" >&2
exit 2
SH
  chmod +x "$work/gh"

  # --- run gate ---
  local rc=0
  mkdir -p "$work/counters"
  PATH="$work:$PATH" FAKE_SCENARIO="$scenario" FAKE_COUNTER_DIR="$work/counters" \
    GITHUB_REPOSITORY="$REPO" SHA="$SHA" TIMEOUT_S=4 POLL_S=1 \
    bash "$SCRIPTS/wait-for-checks.sh" "$SHA" > "$err" 2>&1 || rc=$?

  local verdict
  if { [ "$expected" = "0" ] && [ $rc -eq 0 ]; } || { [ "$expected" = "1" ] && [ $rc -ne 0 ]; }; then
    verdict="PASS"
    PASS=$((PASS+1))
  else
    verdict="FAIL"
    FAIL=$((FAIL+1))
  fi
  printf '%-20s expected=%-1s got=%-1s  %s\n' "$name" "$expected" "$rc" "$verdict"
  if [ "$verdict" = "FAIL" ]; then
    sed 's/^/    | /' "$err" || true
  fi
  rm -rf "$work" "$err"
}

run_case all-green 0
run_case poll-then-green 0
run_case failed-run 1
run_case jobs-incomplete 1
run_case missing-run 1
run_case api-error 1

echo "----"
echo "wait-for-checks tests: PASS=$PASS FAIL=$FAIL"
[ "$FAIL" -eq 0 ]