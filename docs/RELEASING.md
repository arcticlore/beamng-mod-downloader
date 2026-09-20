# Релизный процесс (releasing)

Цели (см. §12–14 и задание «working releases»):

- версия едина во всех manifests/locks (CI проверяет `version-consistency`);
- тег `vX.Y.Z` указывает ровно на тот commit, который прошёл **все** проверки
  (build + CodeQL + реальный OSV scan);
- тег и релиз создаются только **после** прохождения release gate — никогда
  заранее, никогда «вручную в обход проверок»;
- существующие теги/релизы никогда не перезаписываются и не удаляются
  (история preserve-first; release idempotent для повторных досдачи);
- всё выполняется удалённо через GitHub UI/gh; на ноутбуке ничего собирать
  не нужно.

## Запуск релиза

Релиз запускается вручную через `workflow_dispatch` из GitHub Actions:

1. Получите merge PR (только через PR, `main` защищён).
2. Дождитесь, пока на новом HEAD `main` будут зелёные:
   `build` (все jobs + bundle на 3 платформах), `codeql`, `osv-scanner`.
3. Убедитесь, что версии подняты и согласованы:
   ```bash
   node scripts/set-version.mjs 0.2.1   # правит package.json, package-lock.json,
                                        # Cargo.toml, Cargo.lock, tauri.conf.json
   node scripts/check-version.mjs       # та же проверка, что в CI
   ```
4. Откройте Actions → **release** → *Run workflow*, введите:
   - `version`: `0.2.1` (semver, без `v`);
   - `target_sha`: полный 40-hex SHA того commit, что прошёл все проверки
     (например HEAD `main` после merge);
   - `prerelease`: `false` (или `true` для prerelease-релиза).

## Что делает workflow

1. **validate** — формат входов, `target_sha` существует, версия в репозитории
   совпадает с `inputs.version`; тег, если уже существует, должен указывать
   ровно на `target_sha` (иначе fail — чужой тег не трогаем).
2. **build-and-gate** — `scripts/wait-for-checks.sh <target_sha>`: ждёт, пока
   workflow runs `build` (все обязательные jobs + все 3 matrix bundle-платформы),
   `codeql` и `osv-scanner` завершились `success` ровно на этом SHA. Gate
   проверяет по `head_sha` workflow runs, а не по UI-имени check-run.
   Отличия статусов: `wait`/`missing` → polling; failed/API error → fail
   немедленно (no fake-silent-success).
3. **build** (matrix: Ubuntu/Windows/macOS) — собирает bundles (AppImage/deb/rpm,
   NSIS.exe/MSI, DMG), загружает только как artifacts. **не** публикует ничего.
4. **publish** — после трёх зелёных сборок:
   - создаёт **annotated tag** `vX.Y.Z` на `target_sha` (если тега ещё нет);
   - создаёт **draft release** с `--generate-notes` (если ещё нет);
   - вычисляет `X.Y.Z_SHA256SUMS.txt` по всем bundles и self-проверяет;
   - загружает все assets + SHA256SUMS (`--clobber`, idempotent);
   - снимает draft (`--draft=false`) → публикация.

Повторный запуск того же `version`+`target_sha` безопасен (resume): тег уже
указывает на `target_sha`, draft или published релиз не создаётся заново,
assets дозаливаются.

## Отмена / отклонение

- Если gate упал (failed checks или API error) — workflow завершается
  неуспешно, тег/релиз **не** создаются. Это ожидаемое поведение.
- Если нужно прервать до публикации — release создаётся как `draft` и остаётся
  в черновиках; assets и тег при этом уже могут существовать (retry-safe).

## Signed tags

Signed tags предпочтительны, но автоматически в Actions не подписываются;
документируется в security-статусе. Тег в этом процессе — annotated (не
lightweight), создаётся через `git tag -a` на runner и пушится через
`GITHUB_TOKEN`.

## Source tarball для OBS

`packaging/obs/push.sh` на ноутбуке не является частью процесса (§15.5).
OBS-загрузку выполняет отдельный защищённый workflow из tag-commit, с
credentials в Environment Secrets и ожиданием terminal-статусов всех targets.