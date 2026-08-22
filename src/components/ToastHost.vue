<template>
  <div class="toast-host" aria-live="polite">
    <TransitionGroup name="toast">
      <div
        v-for="t in toast.items"
        :key="t.id"
        class="toast-item"
        :class="'toast-' + t.type"
        role="status"
        @click="toast.dismiss(t.id)"
      >
        <i class="bi mx-2" :class="icons[t.type]"></i>
        <span>{{ t.message }}</span>
      </div>
    </TransitionGroup>
  </div>
</template>

<script setup lang="ts">
import { useToast } from "../composables/useToast";

const toast = useToast();
const icons: Record<string, string> = {
  success: "bi-check-circle-fill",
  error: "bi-exclamation-triangle-fill",
  info: "bi-info-circle-fill",
};
</script>
