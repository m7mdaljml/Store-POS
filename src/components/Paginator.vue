<template>
  <nav
    v-if="totalPages > 1"
    class="py-2 border-top d-flex justify-content-center"
    :aria-label="t('common.pagination')"
  >
    <ul class="pagination pagination-sm mb-0">
      <li class="page-item" :class="{ disabled: page <= 1 || disabled }">
        <button
          class="page-link"
          type="button"
          :aria-label="t('common.prevPage')"
          @click="go(page - 1)"
        >
          <i
            class="bi"
            :class="rtl ? 'bi-chevron-right' : 'bi-chevron-left'"
          ></i>
        </button>
      </li>
      <template v-for="(item, i) in items" :key="`${item}-${i}`">
        <li
          v-if="typeof item === 'number'"
          class="page-item"
          :class="{ active: item === page, disabled }"
        >
          <button
            class="page-link"
            type="button"
            :aria-current="item === page ? 'page' : undefined"
            @click="go(item)"
          >
            {{ item }}
          </button>
        </li>
        <li v-else class="page-item">
          <button
            class="page-link"
            type="button"
            :disabled="disabled"
            :title="t('common.morePages')"
            @click="goDots(item)"
          >
            …
          </button>
        </li>
      </template>
      <li
        class="page-item"
        :class="{ disabled: page >= totalPages || disabled }"
      >
        <button
          class="page-link"
          type="button"
          :aria-label="t('common.nextPage')"
          @click="go(page + 1)"
        >
          <i
            class="bi"
            :class="rtl ? 'bi-chevron-left' : 'bi-chevron-right'"
          ></i>
        </button>
      </li>
    </ul>
  </nav>
</template>
<script setup lang="ts">
import { computed } from "vue";
import { useI18n } from "vue-i18n";

/**
 * Page-number pagination bar with a sliding window:
 * 1 2 3 … 8 9 10 — clicking an ellipsis slides the window toward the hidden
 * pages. Works in two modes:
 *  - Known totals: pass `total-items` (+ `page-size`) and every page is
 *    reachable.
 *  - Unknown totals (backend "has more" style): pass `has-more` instead;
 *    pages beyond the last known one stay disabled.
 */
const props = withDefaults(
  defineProps<{
    page: number;
    pageSize?: number;
    /** Total record count when known (client-side lists). */
    totalItems?: number | null;
    /** Whether a page after the current one exists (server-side lists). */
    hasMore?: boolean;
    disabled?: boolean;
  }>(),
  {
    pageSize: 20,
    totalItems: null,
    hasMore: false,
    disabled: false,
  },
);

const emit = defineEmits<{ "update:page": [page: number] }>();

const { t, locale } = useI18n();

const totalPages = computed(() => {
  if (props.totalItems != null) {
    return Math.max(1, Math.ceil(props.totalItems / props.pageSize));
  }
  return props.page + (props.hasMore ? 1 : 0);
});

type PageItem = number | "dots-left" | "dots-right";

const items = computed<PageItem[]>(() => {
  const total = totalPages.value;
  if (total <= 7) return Array.from({ length: total }, (_, i) => i + 1);
  const start = Math.max(2, props.page - 2);
  const end = Math.min(total - 1, props.page + 2);
  const list: PageItem[] = [1];
  if (start > 2) list.push("dots-left");
  for (let p = start; p <= end; p++) list.push(p);
  if (end < total - 1) list.push("dots-right");
  list.push(total);
  return list;
});

function go(target: number) {
  if (props.disabled) return;
  const clamped = Math.min(Math.max(1, target), totalPages.value);
  if (clamped !== props.page) emit("update:page", clamped);
}

/** Ellipsis jumps three pages toward the hidden range. */
function goDots(item: PageItem) {
  go(item === "dots-left" ? props.page - 3 : props.page + 3);
}

const rtl = computed(() => locale.value === "ar");
</script>
