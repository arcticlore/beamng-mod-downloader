import { useEffect, useState } from "react";
import {
  addCustomRepo,
  detectModsFolders,
  getCustomRepos,
  getModsFolder,
  getRepoToken,
  getSettings,
  openExternal,
  openLogDir,
  removeCustomRepo,
  setModsFolder,
  setModsFolderForce,
  setRepoToken,
  setSettings,
} from "../api";
import { applyAppearance, ACCENT_PRESETS } from "../theme";
import type { AppSettings, CustomRepo, ModsFolderCandidate } from "../types";
import { CARD_SIZES, INSTALLED_SORTS, THEMES } from "../types";

interface Props {
  onClose: () => void;
  onChanged: () => void;
}

export function SettingsModal({ onClose, onChanged }: Props) {
  const [current, setCurrent] = useState<string | null>(null);
  const [candidates, setCandidates] = useState<ModsFolderCandidate[]>([]);
  const [manual, setManual] = useState("");
  const [token, setToken] = useState("");
  const [settings, setSettingsState] = useState<AppSettings>({});
  const [customRepos, setCustomRepos] = useState<CustomRepo[]>([]);
  const [repoInput, setRepoInput] = useState("");
  const [msg, setMsg] = useState<{ ok: boolean; text: string } | null>(null);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      const folder = await getModsFolder();
      if (cancelled) return;
      setCurrent(folder);
      setManual(folder ?? "");
      try {
        const cands = await detectModsFolders();
        if (!cancelled) setCandidates(cands);
      } catch {
        if (!cancelled) setCandidates([]);
      }
      const t = await getRepoToken();
      if (!cancelled) setToken(t ?? "");
      const s = await getSettings();
      if (!cancelled) setSettingsState(s);
      const repos = await getCustomRepos();
      if (!cancelled) setCustomRepos(repos);
    })();
    return () => {
      cancelled = true;
    };
  }, []);

  const saveSettings = async (next: AppSettings, quiet = false) => {
    setSettingsState(next);
    applyAppearance(next);
    try {
      await setSettings(next);
      onChanged();
      if (!quiet) setMsg({ ok: true, text: "Настройки интерфейса сохранены" });
    } catch (e) {
      setMsg({ ok: false, text: String(e) });
    }
  };

  const applyFolder = async (path: string, force: boolean) => {
    setBusy(true);
    setMsg(null);
    try {
      if (force) {
        await setModsFolderForce(path);
      } else {
        await setModsFolder(path);
      }
      setCurrent(path);
      setManual(path);
      setMsg({ ok: true, text: "Папка модов сохранена" });
      onChanged();
    } catch (e) {
      setMsg({ ok: false, text: String(e) });
    } finally {
      setBusy(false);
    }
  };

  const saveToken = async () => {
    setBusy(true);
    setMsg(null);
    try {
      await setRepoToken(token);
      setMsg({ ok: true, text: "Токен сохранён" });
    } catch (e) {
      setMsg({ ok: false, text: String(e) });
    } finally {
      setBusy(false);
    }
  };

  const addRepo = async () => {
    setBusy(true);
    setMsg(null);
    try {
      const repos = await addCustomRepo(repoInput);
      setCustomRepos(repos);
      setRepoInput("");
      setMsg({ ok: true, text: "Источник добавлен. Откройте вкладку «Свои источники»." });
      onChanged();
    } catch (e) {
      setMsg({ ok: false, text: String(e) });
    } finally {
      setBusy(false);
    }
  };

  const removeRepo = async (full: string) => {
    setBusy(true);
    setMsg(null);
    try {
      const repos = await removeCustomRepo(full);
      setCustomRepos(repos);
      setMsg({ ok: true, text: `Источник ${full} удалён` });
      onChanged();
    } catch (e) {
      setMsg({ ok: false, text: String(e) });
    } finally {
      setBusy(false);
    }
  };

  const openLogs = async () => {
    setMsg(null);
    try {
      await openLogDir();
    } catch (e) {
      setMsg({ ok: false, text: String(e) });
    }
  };

  return (
    <div className="modal-backdrop" onClick={onClose}>
      <div className="modal modal-settings" onClick={(e) => e.stopPropagation()}>
        <button className="modal-close" onClick={onClose}>✕</button>
        <h2>Настройки</h2>

        <section>
          <h3>Папка с модами BeamNG.drive</h3>
          {current && (
            <div className="current-path">
              Текущая: <code>{current}</code>
            </div>
          )}
          <p className="hint">
            Приложение само ищет папки модов в стандартных местах (Linux, Windows, macOS).
            Выберите вариант ниже или укажите путь вручную.
          </p>
          {candidates.length > 0 ? (
            <div className="candidate-list">
              {candidates.map((c) => (
                <button
                  key={c.path}
                  className={`candidate ${current === c.path ? "candidate-active" : ""}`}
                  onClick={() => {
                    setManual(c.path);
                    applyFolder(c.path, false);
                  }}
                >
                  {c.path}
                </button>
              ))}
            </div>
          ) : (
            <div className="hint">Автоматически ничего не найдено — укажите путь вручную.</div>
          )}
          <div className="manual-row">
            <input
              className="search-input"
              placeholder="/путь/к/.../mods"
              value={manual}
              onChange={(e) => setManual(e.target.value)}
            />
            <button
              className="btn btn-primary"
              disabled={busy || !manual.trim()}
              onClick={() => applyFolder(manual.trim(), false)}
            >
              Применить
            </button>
            <button
              className="btn"
              disabled={busy || !manual.trim()}
              title="Сохранить путь без проверки .zip"
              onClick={() => applyFolder(manual.trim(), true)}
            >
              Принудительно
            </button>
          </div>
        </section>

        <section>
          <h3>Репозиторий BeamNG (токен аккаунта)</h3>
          <p className="hint">
            Официальный репозиторий BeamNG требует токен, который генерирует сама
            игра при входе в аккаунт beamng.com — отдельного сайта для его
            получения нет. Если вы уже входили, скопируйте токен сюда.
          </p>
          <ol className="hint" style={{ margin: "6px 0 10px 18px" }}>
            <li>Запустите BeamNG.drive и войдите в аккаунт beamng.com.</li>
            <li>Откройте в игре «Репозиторий» — токен появится в разделе репозитория.</li>
            <li>Скопируйте токен и вставьте поле ниже.</li>
          </ol>
          <div className="manual-row" style={{ gap: 8 }}>
            <button
              className="btn"
              onClick={() => openExternal("https://www.beamng.com/threads/mod-repository-api.106862/")}
            >
              Инструкция BeamNG
            </button>
            <button
              className="btn"
              onClick={() => openExternal("https://www.beamng.com/login/")}
            >
              Войти на beamng.com
            </button>
          </div>
          <div className="manual-row" style={{ marginTop: 8 }}>
            <input
              className="search-input"
              placeholder="Токен BeamNG"
              value={token}
              onChange={(e) => setToken(e.target.value)}
            />
            <button className="btn" disabled={busy} onClick={saveToken}>
              Сохранить токен
            </button>
          </div>
        </section>

        <section>
          <h3>Внешний вид</h3>
          <div className="hint">Тема:</div>
          <div className="theme-toggle">
            {THEMES.map((t) => (
              <button
                key={t.id}
                className={`btn ${(settings.theme ?? "dark") === t.id ? "btn-active" : ""}`}
                onClick={() => saveSettings({ ...settings, theme: t.id })}
              >
                {t.label}
              </button>
            ))}
          </div>
          <div className="hint" style={{ marginTop: 12 }}>Акцентный цвет:</div>
          <div className="accent-row">
            {ACCENT_PRESETS.map((c) => (
              <button
                key={c}
                className={`swatch ${(settings.accent ?? "#4f8cff").toLowerCase() === c ? "swatch-active" : ""}`}
                style={{ background: c }}
                onClick={() => saveSettings({ ...settings, accent: c })}
                title={c}
              />
            ))}
            <input
              type="color"
              className="color-input"
              value={settings.accent ?? "#4f8cff"}
              onChange={(e) => saveSettings({ ...settings, accent: e.target.value })}
              title="Свой цвет"
            />
          </div>
          <div className="hint" style={{ marginTop: 12 }}>Панель установленных и карточки:</div>
          <div className="manual-row">
            <select
              className="category-select"
              value={settings.installedSort ?? "date"}
              onChange={(e) => saveSettings({ ...settings, installedSort: e.target.value })}
            >
              {INSTALLED_SORTS.map((s) => (
                <option key={s.id} value={s.id}>
                  {s.label}
                </option>
              ))}
            </select>
            <select
              className="category-select"
              value={settings.cardSize ?? "normal"}
              onChange={(e) => saveSettings({ ...settings, cardSize: e.target.value })}
            >
              {CARD_SIZES.map((s) => (
                <option key={s.id} value={s.id}>
                  {s.label}
                </option>
              ))}
            </select>
          </div>
        </section>

        <section>
          <h3>Свои источники (GitHub-репозитории)</h3>
          <p className="hint">
            Закрепите репозитории, из релизов которых хотите ставить моды. Они появятся
            в одноимённой вкладке. Формат: <code>owner/repo</code> или полная ссылка.
          </p>
          <div className="manual-row">
            <input
              className="search-input"
              placeholder="BeamMP/BeamMP"
              value={repoInput}
              onChange={(e) => setRepoInput(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter") addRepo();
              }}
            />
            <button className="btn btn-primary" disabled={busy || !repoInput.trim()} onClick={addRepo}>
              Добавить
            </button>
          </div>
          {customRepos.length > 0 && (
            <div className="repo-list">
              {customRepos.map((r) => (
                <div className="repo-row" key={r.full}>
                  <code>{r.full}</code>
                  <button className="btn btn-danger btn-sm" disabled={busy} onClick={() => removeRepo(r.full)}>
                    Удалить
                  </button>
                </div>
              ))}
            </div>
          )}
        </section>

        <section>
          <h3>Логи</h3>
          <p className="hint">
            Приложение пишет подробный лог (загрузки, ошибки сети, действия) в файл
            <code> bimka.log</code> в каталоге данных приложения.
          </p>
          <button className="btn" onClick={openLogs}>
            Открыть папку логов
          </button>
        </section>

        {msg && (
          <div className={msg.ok ? "banner banner-ok" : "banner banner-error"}>{msg.text}</div>
        )}
      </div>
    </div>
  );
}