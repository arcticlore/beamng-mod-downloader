/**
 * Фейковое Tauri-окружение для node-тестов: подменяет window.__TAURI_INTERNALS__,
 * как это делает harness-заглушка в /home/samsa/.opencode-shots/harness.mjs.
 * Вызовы IPC отправляются в handler(cmd, args); необработанные команды возвращают undefined.
 */
(globalThis as Record<string, unknown>).window ??= globalThis;

export type IpcHandler = (cmd: string, args: Record<string, unknown>) => unknown;

export function stubTauriIpc(handler: IpcHandler): void {
  const w = globalThis as unknown as Record<string, unknown>;
  w.__TAURI_INTERNALS__ = {
    transformCallback: () => {
      w.__cb = (((w.__cb as number) ?? 0) + 1) as number;
      return w.__cb;
    },
    unregisterCallback: () => {},
    unregisterListener: () => {},
    convertFileSrc: (p: string) => p,
    metadata: {
      currentWindow: { label: "main" },
      currentWebview: { label: "main" },
    },
    invoke: async (cmd: string, args: Record<string, unknown>) =>
      handler(cmd, args ?? {}),
  };
  w.__TAURI_EVENT_PLUGIN_INTERNALS__ = {
    registerListener: () => {},
    unregisterListener: () => {},
  };
}