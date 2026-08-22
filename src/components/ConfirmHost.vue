<template>
  <div v-if="state.show" class="modal-backdrop show"></div>
  <div
    v-if="state.show"
    class="modal d-block"
    tabindex="-1"
    role="dialog"
    aria-modal="true"
    @keydown.esc="settle(false)"
  >
    <div class="modal-dialog modal-dialog-centered modal-sm">
      <div class="modal-content">
        <div class="modal-header">
          <h5 class="modal-title">{{ state.title }}</h5>
          <button
            type="button"
            class="btn-close"
            :aria-label="t('common.close')"
            @click="settle(false)"
          ></button>
        </div>
        <div class="modal-body">{{ state.message }}</div>
        <div class="modal-footer">
          <button
            type="button"
            class="btn btn-outline-secondary"
            @click="settle(false)"
          >
            {{ state.cancelLabel }}
          </button>
          <button
            type="button"
            class="btn"
            :class="state.danger ? 'btn-danger' : 'btn-primary'"
            @click="settle(true)"
          >
            {{ state.confirmLabel }}
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { useI18n } from "vue-i18n";
import { useConfirm } from "../composables/useConfirm";

const { t } = useI18n();
const { settle, state } = useConfirm();
</script>
