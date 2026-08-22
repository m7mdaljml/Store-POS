import { computed, ref, watch, type Ref, type WatchSource } from "vue";

export const PAGE_SIZE = 20;

/**
 * Shared "load more" pagination for list pages: every fetch pulls one page of
 * `pageSize` records and `loadMore()` appends the next page. When any of the
 * `resetSources` change (search text, filters...) the list restarts from the
 * first page. A stale-response guard keeps out-of-order replies from clobbering
 * newer results.
 */
export function usePagedList<T>(
  fetchPage: (limit: number, offset: number) => Promise<T[]>,
  resetSources: WatchSource[] = [],
  onError: (e: unknown) => void = () => {},
  pageSize = PAGE_SIZE,
) {
  const items = ref([]) as Ref<T[]>;
  const loading = ref(false);
  const loadingMore = ref(false);
  const lastPageCount = ref(0);

  let seq = 0;
  let debounceTimer: ReturnType<typeof setTimeout> | null = null;

  // The button shows whenever the last fetch returned a full page, i.e. there
  // may be more records beyond what is currently rendered.
  const hasMore = computed(() => lastPageCount.value >= pageSize);

  async function reload() {
    const id = ++seq;
    loading.value = true;
    try {
      const rows = await fetchPage(pageSize, 0);
      if (id !== seq) return;
      items.value = rows;
      lastPageCount.value = rows.length;
    } catch (e) {
      if (id === seq) onError(e);
    } finally {
      if (id === seq) loading.value = false;
    }
  }

  async function loadMore() {
    if (loading.value || loadingMore.value || !hasMore.value) return;
    const id = seq;
    loadingMore.value = true;
    try {
      const rows = await fetchPage(pageSize, items.value.length);
      // Ignore stale pages that raced with a filter/search change.
      if (id !== seq) return;
      items.value.push(...rows);
      lastPageCount.value = rows.length;
    } catch (e) {
      if (id === seq) onError(e);
    } finally {
      if (id === seq) loadingMore.value = false;
    }
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

  return { items, loading, loadingMore, hasMore, reload, loadMore };
}
