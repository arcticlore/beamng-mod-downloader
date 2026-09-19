# Branch protection: `main`

Политика ремедиации (§2, §11 промта): работа только через ветки и PR,
прямой push в `main` запрещён, вдобавок CI обязателен.

## Требуемые настройки (GitHub → Settings → Branches → Add rule → `main`)

- [ ] **Require a pull request before merging** — включено;
- [ ]   **Require approvals**: 1 (или 0, если single owner; тогда документируем
       личную политику);
- [ ]   **Dismiss stale pull request approvals when new commits are pushed** —
       включено;
- [ ] **Require status checks to pass before merging** — включено;
- [ ]   required checks (имена job'ов `build` workflow):
       `fmt`, `test`, `clippy`, `tsc`, `frontend-build`, `frontend-test`,
       `npm-audit`, `cargo-deny`, `osv-scanner`, `version-consistency`,
       `tauri-config`, `packaging-static`, `bundle`;
- [ ] **Require branches to be up to date before merging** — включено;
- [ ] **Do not allow bypassing the above settings** — включено;
- [ ] **Require linear history** — включено (merge policy: Squash/Squash-and-merge);
- [ ] **Block force pushes** — включено;
- [ ] **Block deletions** — включено.

## Как применить через GitHub API / gh

```bash
gh api -X PUT repos/arcticlore/beamng-mod-downloader/branches/main/protection \
  -f "required_status_checks[strict]=true" \
  -f "required_status_checks[contexts][]=fmt" \
  -f "required_status_checks[contexts][]=test" \
  -f "required_status_checks[contexts][]=clippy" \
  -f "required_status_checks[contexts][]=tsc" \
  -f "required_status_checks[contexts][]=frontend-build" \
  -f "required_status_checks[contexts][]=frontend-test" \
  -f "required_status_checks[contexts][]=npm-audit" \
  -f "required_status_checks[contexts][]=cargo-deny" \
  -f "required_status_checks[contexts][]=osv-scanner" \
  -f "required_status_checks[contexts][]=version-consistency" \
  -f "required_status_checks[contexts][]=tauri-config" \
  -f "required_status_checks[contexts][]=packaging-static" \
  -f "required_status_checks[contexts][]=bundle" \
  -f "enforce_admins=true" \
  -f "required_pull_request_reviews[required_approving_review_count]=1" \
  -f "required_pull_request_reviews[dismiss_stale_reviews]=true" \
  -f "restrictions=null" \
  -f "allow_force_pushes=false" \
  -f "allow_deletions=false" \
  -f "required_linear_history=true"
```

(Параметр `contexts` включает только фактически существующие checks; список
обновляется при изменении job'ов `build` workflow.)

## Статус

Если у аутентифицированного токена (remediation-runner PAT) нет прав admin,
приведённая выше команда вернёт 403. Тогда защита настраивается владельцем
вручную по чек-листу, а для репозитория фиксируется **внешний blocker**
(см. docs/THREAT-MODEL.md и итоговый отчёт).