<script setup lang="ts">
import { computed } from "vue";

const props = withDefaults(
  defineProps<{
    loading?: boolean;
    disabled?: boolean;
    variant?: string;
    size?: "sm" | "lg" | undefined;
    type?: "button" | "submit";
    block?: boolean;
  }>(),
  {
    loading: false,
    disabled: false,
    variant: "primary",
    size: undefined,
    type: "button",
    block: false,
  },
);

const emit = defineEmits<{ click: [] }>();

const classes = computed(() => [
  "btn",
  `btn-${props.variant}`,
  props.size ? `btn-${props.size}` : "",
  props.block ? "w-100" : "",
]);

function onClick() {
  if (props.loading || props.disabled) return;
  emit("click");
}
</script>

<template>
  <button
    :class="classes"
    :type="type"
    :disabled="loading || disabled"
    @click="onClick"
  >
    <span
      v-if="loading"
      class="spinner-border spinner-border-sm mx-2"
      role="status"
      aria-hidden="true"
    ></span>
    <slot />
  </button>
</template>
