<template>
  <div ref="root" class="app-select">
    <button
      type="button"
      class="form-select text-start d-flex align-items-center justify-content-between"
      :class="{ 'form-select-sm': sm }"
      :disabled="disabled"
      :aria-expanded="open"
      aria-haspopup="listbox"
      @click="togglePanel"
      @keydown="onKeydown"
    >
      <span class="text-truncate" :class="{ 'text-muted': modelValue == null }">
        {{
          modelValue != null ? selectedLabel : placeholder || t("common.select")
        }}
      </span>
      <span
        v-if="modelValue != null && !disabled"
        class="app-select-clear"
        role="button"
        :aria-label="t('common.clear')"
        @click.stop="clearSelection"
        @keydown.stop
      >
        ×
      </span>
    </button>

    <div v-if="open" class="app-select-panel shadow" :style="panelStyle">
      <div class="app-select-search border-bottom">
        <input
          class="form-control form-control-sm border-0"
          type="text"
          v-model="search"
          :placeholder="t('common.typeToSearch')"
          @keydown="onKeydown"
        />
      </div>
      <div ref="listEl" class="app-select-list" @scroll="onListScroll">
        <div v-if="initialLoading" class="text-muted small px-3 py-2">
          {{ t("common.loading") }}
        </div>
        <template v-else-if="options.length">
          <button
            v-for="(item, i) in options"
            :key="`${i}-${String(optionValue(item))}`"
            type="button"
            class="app-select-option"
            :class="{
              active: optionValue(item) === modelValue,
              hovered: i === highlighted,
            }"
            :data-index="i"
            @click="select(item)"
            @mousemove="highlighted = i"
          >
            {{ optionLabel(item) }}
          </button>
          <div v-if="hasMore" class="text-center py-2 border-top">
            <span
              v-if="loadingMore"
              class="spinner-border spinner-border-sm text-muted"
            ></span>
            <button
              v-else
              type="button"
              class="btn btn-sm btn-link text-decoration-none"
              @click="showMore"
            >
              {{ t("common.moreResults") }}
            </button>
          </div>
        </template>
        <div v-else class="text-muted small px-3 py-2">
          {{ t("common.noResults") }}
        </div>
      </div>
    </div>
  </div>
</template>
<script setup lang="ts" generic="T">
import { computed, nextTick, onBeforeUnmount, ref, watch, type Ref } from "vue";
import { useI18n } from "vue-i18n";
import { useSettingsStore } from "../stores/settings";

/**
 * Searchable, paginated dropdown with a fixed-height option panel — replaces
 * native <select> elements fed from database lists that may hold thousands of
 * rows. Two data modes:
 *  - Local: pass `items` (e.g. the shared catalog store) — searching and
 *    paging happen client-side.
 *  - Async: pass `fetchPage(limit, offset, search)` — options are paged and
 *    searched server-side; more pages load while scrolling.
 */
const props = withDefaults(
  defineProps<{
    modelValue: string | number | null | undefined;
    items?: T[] | null;
    fetchPage?: (limit: number, offset: number, search: string) => Promise<T[]>;
    optionLabel: (item: T) => string;
    optionValue: (item: T) => string | number | null;
    placeholder?: string;
    disabled?: boolean;
    sm?: boolean;
    pageSize?: number;
  }>(),
  {
    items: null,
    fetchPage: undefined,
    placeholder: "",
    disabled: false,
    sm: false,
    pageSize: undefined,
  },
);

const emit = defineEmits<{
  "update:modelValue": [value: string | number | null];
  change: [item: T];
}>();

const { t } = useI18n();
const settings = useSettingsStore();

/** Options per panel page — defaults to the user's "rows per page" preference
 *  from Settings so the dropdown pages exactly like the list screens. */
const pageSizeValue = computed(() => {
  const n = props.pageSize ?? settings.pageSize;
  return n > 0 ? n : settings.pageSize;
});

const open = ref(false);
const search = ref("");
const asyncOptions = ref([]) as Ref<T[]>;
const visibleCount = ref(pageSizeValue.value);
const loadingMore = ref(false);
const initialLoading = ref(false);
const highlighted = ref(0);
const root = ref<HTMLElement | null>(null);
const listEl = ref<HTMLElement | null>(null);

watch(pageSizeValue, (n) => {
  visibleCount.value = n;
});

const labelCache = new Map<string | number | null, string>();

let seq = 0;
let debounceTimer: ReturnType<typeof setTimeout> | null = null;

const isAsync = computed(() => !props.items && !!props.fetchPage);

/* ----- Option source ----- */

const filteredLocal = computed(() => {
  if (!props.items) return [] as T[];
  const q = search.value.trim().toLowerCase();
  if (!q) return props.items;
  return props.items.filter((item) =>
    props.optionLabel(item).toLowerCase().includes(q),
  );
});

const visibleLocal = computed(() =>
  filteredLocal.value.slice(0, visibleCount.value),
);

const hasMoreLocal = computed(
  () => filteredLocal.value.length > visibleCount.value,
);

const options = computed(() =>
  isAsync.value ? (asyncOptions.value as T[]) : visibleLocal.value,
);

const hasMore = computed(() =>
  isAsync.value ? asyncHasMore.value : hasMoreLocal.value,
);

/* ----- Selected label ----- */

function cacheLabels(items: T[]) {
  for (const item of items) {
    labelCache.set(props.optionValue(item), props.optionLabel(item));
  }
}

watch(
  () => options.value,
  (opts) => cacheLabels(opts),
  { immediate: true },
);

const selectedLabel = computed(() => {
  if (props.modelValue == null) return "";
  return labelCache.get(props.modelValue) ?? String(props.modelValue);
});

/* ----- Async paging ----- */

const asyncHasMore = ref(false);

async function loadAsyncPage(reset: boolean) {
  const fetch = props.fetchPage;
  if (!fetch || loadingMore.value) return;
  const id = ++seq;
  if (reset) initialLoading.value = true;
  else loadingMore.value = true;
  try {
    const offset = reset ? 0 : asyncOptions.value.length;
    const rows = await fetch(pageSizeValue.value, offset, search.value.trim());
    if (id !== seq) return;
    asyncOptions.value = reset ? rows : [...asyncOptions.value, ...rows];
    asyncHasMore.value = rows.length >= pageSizeValue.value;
    cacheLabels(rows);
  } finally {
    if (id === seq) {
      initialLoading.value = false;
      loadingMore.value = false;
    }
  }
}

/* ----- Open / close / search ----- */

const panelStyle = ref<Record<string, string>>({});

/** Position the panel with `position: fixed` anchored to the trigger so it
 *  escapes scrollable/modal containers, flipping upward when space below is
 *  tight. */
function updatePanelPosition() {
  const trigger = root.value?.querySelector("button");
  if (!trigger) return;
  const r = trigger.getBoundingClientRect();
  const style: Record<string, string> = {
    left: `${r.left}px`,
    width: `${r.width}px`,
    top: "auto",
    bottom: "auto",
  };
  const spaceBelow = window.innerHeight - r.bottom;
  if (spaceBelow < 220 && r.top > 260) {
    style.bottom = `${window.innerHeight - r.top + 2}px`;
  } else {
    style.top = `${r.bottom + 2}px`;
  }
  panelStyle.value = style;
}

function onWindowChange() {
  if (open.value) updatePanelPosition();
}

function openPanel() {
  if (props.disabled || open.value) return;
  open.value = true;
  search.value = "";
  visibleCount.value = pageSizeValue.value;
  highlighted.value = 0;
  updatePanelPosition();
  void nextTick(updatePanelPosition);
  if (isAsync.value && !asyncOptions.value.length) {
    void loadAsyncPage(true);
  }
}

function closePanel() {
  open.value = false;
}

function togglePanel() {
  open.value ? closePanel() : openPanel();
}

watch(search, () => {
  visibleCount.value = pageSizeValue.value;
  highlighted.value = 0;
  if (isAsync.value) {
    if (debounceTimer) clearTimeout(debounceTimer);
    debounceTimer = setTimeout(() => void loadAsyncPage(true), 250);
  }
});

function onDocumentPointerDown(e: MouseEvent) {
  if (open.value && root.value && !root.value.contains(e.target as Node)) {
    closePanel();
  }
}

document.addEventListener("mousedown", onDocumentPointerDown);
document.addEventListener("scroll", onWindowChange, true);
window.addEventListener("resize", onWindowChange);
onBeforeUnmount(() => {
  document.removeEventListener("mousedown", onDocumentPointerDown);
  document.removeEventListener("scroll", onWindowChange, true);
  window.removeEventListener("resize", onWindowChange);
});

/* ----- Selection ----- */

function select(item: T) {
  const value = props.optionValue(item);
  emit("update:modelValue", value);
  emit("change", item);
  closePanel();
}

function clearSelection() {
  emit("update:modelValue", null);
}

/* ----- Keyboard support ----- */

function moveHighlight(delta: number) {
  const max = options.value.length - 1;
  if (max < 0) return;
  highlighted.value = Math.min(Math.max(highlighted.value + delta, 0), max);
  void nextTick(() => {
    listEl.value
      ?.querySelector(`[data-index="${highlighted.value}"]`)
      ?.scrollIntoView({ block: "nearest" });
  });
}

function onKeydown(e: KeyboardEvent) {
  if (e.key === "Escape") {
    e.stopPropagation();
    closePanel();
  } else if (e.key === "ArrowDown") {
    e.preventDefault();
    moveHighlight(1);
  } else if (e.key === "ArrowUp") {
    e.preventDefault();
    moveHighlight(-1);
  } else if (e.key === "Enter") {
    e.preventDefault();
    const item = options.value[highlighted.value];
    if (item) select(item);
  }
}

/* ----- Infinite scroll ----- */

/** Reveals exactly one more page of options (next `pageSizeValue` rows). */
function showMore() {
  if (isAsync.value) void loadAsyncPage(false);
  else visibleCount.value += pageSizeValue.value;
}

function onListScroll() {
  const el = listEl.value;
  if (!el || !hasMore.value || loadingMore.value || initialLoading.value)
    return;
  // Local lists advance strictly one page per "Show more" click — otherwise
  // pinned-at-bottom scrolling chains loads back-to-back and dumps every
  // remaining option at once. Server-paged lists keep smooth infinite scroll.
  if (!isAsync.value) return;
  if (el.scrollTop + el.clientHeight >= el.scrollHeight - 40) {
    showMore();
  }
}
</script>

<style scoped>
.app-select {
  position: relative;
}

.app-select-panel {
  position: fixed;
  z-index: 1200;
  background: var(--bs-body-bg);
  border: 1px solid var(--bs-border-color);
  border-radius: 0.375rem;
}

/* Fixed height keeps long DB lists friendly — the panel never grows with data. */
.app-select-list {
  max-height: 264px;
  min-height: 36px;
  overflow-y: auto;
  overscroll-behavior: contain;
  border-radius: 0 0 0.35rem 0.35rem;
}

.app-select-option {
  display: block;
  width: 100%;
  text-align: start;
  padding: 0.4rem 0.75rem;
  background: transparent;
  border: 0;
  color: var(--bs-body-color);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.app-select-option.hovered,
.app-select-option.active {
  background: var(--bs-tertiary-bg);
}

.app-select-option.active {
  font-weight: 600;
}

.app-select-clear {
  line-height: 1;
  padding: 0 0.15rem;
  color: var(--bs-secondary-color);
  font-size: 1rem;
}

.app-select-clear:hover {
  color: var(--bs-danger);
}
</style>
