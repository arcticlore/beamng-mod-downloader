/**
 * Файловый диалог выбора .zip для импорта.
 *
 * Плагин tauri-plugin-dialog импортируется ДИНАМИЧЕСКИ: он нужен только в
 * единственном экране импорта, а статический импорт тянул бы его во все бандлы.
 * В node-тестах модуль резолвится как обычный JS-модуль без Tauri-рантайма,
 * поэтому вызов безопасно отдаёт `null` через пустой invoke.
 */
export async function pickZipFile(): Promise<string | null> {
  const dialog = await import("@tauri-apps/plugin-dialog");
  const picked = await dialog.open({
    title: "Выберите архив мода",
    multiple: false,
    directory: false,
    filters: [{ name: "ZIP archive", extensions: ["zip"] }],
  });
  return typeof picked === "string" ? picked : null;
}