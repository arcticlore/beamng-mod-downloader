import { useEffect, useState } from "react";
import {
  detectModsFolders,
  getModsFolder,
  getRepoToken,
  openExternal,
  setModsFolder,
  setModsFolderForce,
  setRepoToken,
} from "../api";
import type { ModsFolderCandidate } from "../types";

interface Props {
  onClose: () => void;
  onChanged: () => void;
}

export function SettingsModal({ onClose, onChanged }: Props) {
  const [current, setCurrent] = useState<string | null>(null);
  const [candidates, setCandidates] = useState<ModsFolderCandidate[]>([]);
  const [manual, setManual] = useState("");
  const [token, setToken] = useState("");
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
    })();
    return () => {
      cancelled = true;
    };
  }, []);

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

        {msg && (
          <div className={msg.ok ? "banner banner-ok" : "banner banner-error"}>{msg.text}</div>
        )}
      </div>
    </div>
  );
}