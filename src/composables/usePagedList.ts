import {
  computed,
  ref,
  toValue,
  watch,
  type MaybeRefOrGetter,
  type Ref,
  type WatchSource,
} from "vue";
import { useSettingsStore } from "../stores/settings";

/** Fallback page size when no user preference is available. */
export const PAGE_SIZE = 20;

/** One page of records plus the exact filtered row count. */
export interface Paged<T> {
  items: T[];
  total: number;
}

/**
 * Shared page-based pagination for list pages backed by exact row counts:
 * every fetch pulls one page of `pageSize` records together with the total,
 * so the Paginator can render true page numbers exactly like the client-side
 * lists. `goToPage(n)` replaces the rendered slice. When any of the
 * `resetSources` change (search text, filters...) or the preferred page size
 * changes, the list restarts from the first page. A stale-response guard
 * keeps out-of-order replies from clobbering newer results.
 *
 * `pageSize` may be omitted to follow the user's "rows per page" preference
 * from Settings reactively.
 */
export function usePagedList<T>(
  fetchPage: (limit: number, offset: number) => Promise<Paged<T>>,
  resetSources: WatchSource[] = [],
  onError: (e: unknown) => void = () => {},
  pageSize?: MaybeRefOrGetter<number>,
) {
  const settings = useSettingsStore();

  const items = ref([]) as Ref<T[]>;
  const loading = ref(false);
  const page = ref(1);
  const totalItems = ref<number | null>(null);

  let seq = 0;
  let debounceTimer: ReturnType<typeof setTimeout> | null = null;

  const size = computed(() => {
    const explicit = pageSize !== undefined ? toValue(pageSize) : undefined;
    const n = explicit ?? settings.pageSize;
    return n > 0 ? Math.floor(n) : PAGE_SIZE;
  });

  const totalPages = computed(() =>
    Math.max(1, Math.ceil((totalItems.value ?? 0) / size.value)),
  );

  async function load(target: number): Promise<void> {
    const id = ++seq;
    loading.value = true;
    try {
      let res = await fetchPage(size.value, (target - 1) * size.value);
      if (id !== seq) return;
      // Records may have been deleted while browsing past the end.
      if (!res.items.length && target > 1 && res.total > 0) {
        target = Math.max(1, Math.min(target - 1, totalPages.value));
        res = await fetchPage(size.value, (target - 1) * size.value);
        if (id !== seq) return;
      }
      items.value = res.items;
      totalItems.value = res.total;
      page.value = target;
    } catch (e) {
      if (id === seq) onError(e);
    } finally {
      if (id === seq) loading.value = false;
    }
  }

  function reload(): Promise<void> {
    return load(1);
  }

  async function goToPage(n: number): Promise<void> {
    if (loading.value || n < 1 || n > totalPages.value) return;
    const target = Math.max(1, Math.min(n, totalPages.value));
    if (target === page.value) return;
    await load(target);
  }

  function debouncedReload() {
    if (debounceTimer) clearTimeout(debounceTimer);
    debounceTimer = setTimeout(() => {
      debounceTimer = null;
      reload();
    }, 250);
  }

  if (resetSources.length) {
    watch(resetSources, debouncedReload, { deep: true });
  }
  watch(size, debouncedReload);

  return { items, loading, page, size, totalItems, reload, goToPage };
}
