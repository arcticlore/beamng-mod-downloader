import { useEffect, useState } from "react";
import {
  detectModsFolders,
  getModsFolder,
  getRepoToken,
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
            Официальный репозиторий требует авторизацию. Токен можно получить в самом
            BeamNG.drive при входе в аккаунт — игра использует его для доступа к repo API.
            Вкладка «Официальный сайт BeamNG» работает без токена.
          </p>
          <div className="manual-row">
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