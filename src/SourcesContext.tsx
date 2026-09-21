import { createContext, useContext, useEffect, useMemo, useCallback, useState } from "react";
import type { ReactNode } from "react";
import {
  getSourceRegistry,
  getSourceSelection,
  resetSourcesToDefaults,
  setSourceEnabled,
  setSourceSelected,
} from "./api";
import {
  filenameFor as computeFilename,
} from "./sources";
import type {
  FilenameRule,
  ModItem,
  SourceDescriptor,
  SourceSelection,
} from "./types";

interface SourcesContextValue {
  registry: SourceDescriptor[];
  selection: SourceSelection | null;
  loading: boolean;
  error: string | null;
  refresh: () => Promise<void>;
  setEnabled: (id: string, enabled: boolean) => Promise<void>;
  setSelected: (ids: string[] | null) => Promise<void>;
  resetDefaults: () => Promise<void>;
  descriptorOf: (id: string) => SourceDescriptor | undefined;
  labelOf: (id: string) => string;
  ruleOf: (id: string) => FilenameRule;
  filenameFor: (item: ModItem) => string;
}

const SourcesContext = createContext<SourcesContextValue | null>(null);

export function SourcesProvider({ children }: { children: ReactNode }) {
  const [registry, setRegistry] = useState<SourceDescriptor[]>([]);
  const [selection, setSelection] = useState<SourceSelection | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    setLoading(true);
    try {
      const [reg, sel] = await Promise.all([
        getSourceRegistry(),
        getSourceSelection(),
      ]);
      setRegistry(reg);
      setSelection(sel);
      setError(null);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void refresh();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [refresh]);

  const setEnabled = useCallback(
    async (id: string, enabled: boolean) => {
      await setSourceEnabled(id, enabled);
      await refresh();
    },
    [refresh],
  );

  const setSelected = useCallback(
    async (ids: string[] | null) => {
      await setSourceSelected(ids);
      await refresh();
    },
    [refresh],
  );

  const resetDefaults = useCallback(async () => {
    await resetSourcesToDefaults();
    await refresh();
  }, [refresh]);

  const value = useMemo<SourcesContextValue>(
    () => ({
      registry,
      selection,
      loading,
      error,
      refresh,
      setEnabled,
      setSelected,
      resetDefaults,
      descriptorOf: (id) => registry.find((d) => d.id === id),
      labelOf: (id) => registry.find((d) => d.id === id)?.label ?? id,
      ruleOf: (id) =>
        registry.find((d) => d.id === id)?.filenameRule ?? "key_stem",
      filenameFor: (item) =>
        computeFilename(
          item,
          registry.find((d) => d.id === item.source)?.filenameRule ?? "key_stem",
        ),
    }),
    [registry, selection, loading, error, refresh, setEnabled, setSelected, resetDefaults],
  );

  return (
    <SourcesContext.Provider value={value}>{children}</SourcesContext.Provider>
  );
}

export function useSources(): SourcesContextValue {
  const ctx = useContext(SourcesContext);
  if (!ctx) throw new Error("useSources() нужно вызывать внутри SourcesProvider");
  return ctx;
}