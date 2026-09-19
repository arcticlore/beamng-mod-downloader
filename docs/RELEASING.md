# Релизный процесс (releasing)

Цели (см. §12–14 remediation-промта):

- версия едина во всех manifests/locks (CI проверяет `version-consistency`);
- тег `vX.Y.Z` указывает ровно на тот commit, из которого собран source
  tarball / release;
- release начинается только после зелёного `build` workflow (release gate);
- всё выполняется удалённо; на ноутбуке ничего собирать не нужно.

## Смена версии

```bash
node scripts/set-version.mjs 0.2.0   # правит package.json, package-lock.json,
                                     # Cargo.toml, Cargo.lock, tauri.conf.json
node scripts/check-version.mjs       # та же проверка, что в CI
```

`set-version.mjs` и `check-version.mjs` живут в `scripts/`; yaml-шаблоны OBS
используют `@PARENT_TAG@`, поэтому отдельные правки версии в packaging не
требуются (кроме PKGBUILD при создании релиза AUR).

## Tag и репозиторий

1. Получите approve/merge PR (только через PR, `main` защищён).
2. Убедитесь, что `build` зелёный на main (все job'ы выше).
3. Создайте тег и push его:
   ```bash
   git tag -s v0.2.0            # или -a без ключа
   git push origin v0.2.0
   ```
   Signed tags предпочтительны; если ключ не настроен — аннотированный tag и
   фиксация этого в security-статусе.
4. `release` workflow сначала выполняет gate (`scripts/wait-for-checks.sh`),
   который ждёт зелёных check-runs `build` для SHA тега. Если gate не прошёл,
   release не начинается.
5. tauri-action создаёт **draft release** с assets. При наличии сертификатов
   подписи настраиваются отдельно (окно/macOS) — без них bundles остаются
   unsigned и так и помечаются.

## Check-sum, SBOM, provenance

Перед публикацией draft: вычисляются SHA-256 checksum'ы, генерируются
SBOM (CycloneDX/SPDX) и GitHub artifact attestations (добавляются к
release-workflow в PR supply-chain). Draft публикуется только после approval и
проверки всех assets.

## Source tarball для OBS (не на ноутбуке)

`packaging/obs/push.sh` на ноутбуке больше **не** является частью процесса
(§15.5). OBS-загрузку выполняет защищённый workflow из tag-commit, с
credentials в Environment Secrets и ожиданием terminal-статусов всех targets.