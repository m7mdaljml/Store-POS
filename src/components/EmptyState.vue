<script setup lang="ts">
defineProps<{
  image: string;
  message: string;
}>();
</script>

<template>
  <div class="empty-state text-center py-5">
    <img :src="image" alt="" class="empty-illustration mb-3" draggable="false" />
    <p class="text-muted mb-0">{{ message }}</p>
  </div>
</template>

<style scoped>
.empty-illustration {
  width: 150px;
  max-width: 60%;
  height: auto;
  opacity: 0.9;
  user-select: none;
}
.empty-state p {
  font-size: 0.95rem;
}
</style>
