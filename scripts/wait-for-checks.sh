#!/usr/bin/env bash
# Release gate: ожидает, что все обязательные check-runs 'build' workflow
# для заданного commit успешны. Используется в release.yml на tag-push.
#
# Применение: bash scripts/wait-for-checks.sh <sha>
#   GH_TOKEN (или gh auth) должен быть доступен; в Actions — GITHUB_TOKEN.
#
# Обязательные job'ы build.yml перечислены в REQUIRED. Все они обязаны
# завершиться success для commit tag'а; иначе release запрещён.

set -euo pipefail

SHA="${1:?usage: wait-for-checks.sh <sha>}"
TIMEOUT_S="${TIMEOUT_S:-1500}"
POLL_S="${POLL_S:-30}"

REQUIRED=(fmt test clippy tsc frontend-build frontend-test npm-audit cargo-deny version-consistency tauri-config packaging-static "bundle")

REPO="${GITHUB_REPOSITORY:-$(git remote get-url origin | sed -E 's#.*[:/]([^/]+/[^/]+)(\.git)?$#\1#')}"

deadline=$(( $(date +%s) + TIMEOUT_S ))

while :; do
  # check-runs, привязанные к commit (не только последний suite)
  runs=$(gh api "repos/$REPO/commits/$SHA/check-runs" --paginate --jq \
    '.check_runs[] | select(.name | startswith("build / ")) | {name: (.name|sub("^build / "; "")), status: .status, conclusion: .conclusion}' 2>/dev/null || echo "[]")

  missing=0
  failed=0
  done=0
  echo "=== держим check-runs для $SHA ==="
  for req in "${REQUIRED[@]}"; do
    # bundle — матрица: достаточно одного успешного bundle job
    if [ "$req" = "bundle" ]; then
      n=$(echo "$runs" | python3 -c "
import json,sys
rows=json.load(sys.stdin)
ok=[r for r in rows if r['name'].startswith('bundle') and r['status']=='completed' and r['conclusion']=='success']
print(len(ok))")
      if [ "$n" -ge 1 ]; then echo "  OK   bundle (>=1)"; done=$((done+1)); else echo "  WAIT bundle"; fi
      continue
    fi
    row=$(echo "$runs" | python3 -c "
import json,sys
rows=json.load(sys.stdin)
want=sys.argv[1]
m=[r for r in rows if r['name']==want]
if not m:
    print('missing')
elif m[-1]['status']=='completed' and m[-1]['conclusion']=='success':
    print('ok')
else:
    print(m[-1]['status']+'/'+str(m[-1]['conclusion']))
" "$req")
    case "$row" in
      ok)        echo "  OK    $req"; done=$((done+1)) ;;
      completed*) echo "  FAIL  $req ($row)"; failed=$((failed+1)) ;;
      missing)   echo "  WAIT  $req (нет check-run для этого commit)"; missing=$((missing+1)) ;;
      *)         echo "  WAIT  $req ($row)"; missing=$((missing+1)) ;;
    esac
  done

  if [ "$failed" -gt 0 ]; then
    echo "Один или несколько обязательных check-runs завершились неуспешно; release отменяется."
    exit 1
  fi
  if [ "$missing" -eq 0 ] && [ "$done" -eq "${#REQUIRED[@]}" ]; then
    echo "Все обязательные check-runs успешны. Gate пройден."
    exit 0
  fi

  if [ "$(date +%s)" -ge "$deadline" ]; then
    echo "Таймаут ожидания check-runs ($TIMEOUT_S с). Проверьте статус commit вручную."
    exit 1
  fi
  sleep "$POLL_S"
done