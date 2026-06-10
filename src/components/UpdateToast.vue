<script setup lang="ts">
import { computed } from "vue";
import { Icon } from "@iconify/vue";
import { useUpdaterStore } from "../stores/updater";

const store = useUpdaterStore();

const visible = computed(() => store.available && !store.dismissed);
const pct = computed(() =>
  store.total > 0 ? Math.min(100, Math.round((store.downloaded / store.total) * 100)) : 0,
);
</script>

<template>
  <Transition name="toast">
    <div
      v-if="visible"
      class="fixed bottom-4 right-4 z-50 w-80 bg-otter-card border border-otter-border rounded-xl shadow-lg overflow-hidden"
    >
      <div class="flex items-start gap-3 p-4">
        <Icon icon="mdi:rocket-launch-outline" class="w-5 h-5 text-otter-teal flex-shrink-0 mt-0.5" />
        <div class="min-w-0 flex-1">
          <p class="text-sm font-semibold text-otter-text">Update available</p>
          <p class="text-xs text-otter-muted mt-0.5">Version {{ store.newVersion }} is ready to install.</p>

          <div v-if="store.downloading" class="mt-3">
            <div class="h-1.5 rounded-full bg-otter-surface overflow-hidden">
              <div class="h-full bg-otter-teal transition-all" :style="{ width: pct + '%' }"></div>
            </div>
            <p class="text-[10px] text-otter-subtle mt-1">Downloading… {{ pct }}%</p>
          </div>

          <p v-else-if="store.error" class="text-[11px] text-red-400 mt-2">{{ store.error }}</p>

          <div v-else class="flex items-center gap-2 mt-3">
            <button
              class="px-3 py-1.5 rounded-lg bg-otter-teal text-otter-dark text-xs font-medium hover:opacity-90"
              @click="store.install()"
            >
              Install &amp; restart
            </button>
            <button
              class="px-3 py-1.5 rounded-lg text-xs text-otter-muted hover:text-otter-text hover:bg-otter-surface"
              @click="store.dismiss()"
            >
              Later
            </button>
          </div>
        </div>

        <button
          v-if="!store.downloading"
          class="p-0.5 rounded hover:bg-otter-surface text-otter-subtle hover:text-otter-text flex-shrink-0"
          @click="store.dismiss()"
        >
          <Icon icon="mdi:close" class="w-4 h-4" />
        </button>
      </div>
    </div>
  </Transition>
</template>

<style scoped>
.toast-enter-active,
.toast-leave-active {
  transition: all 0.2s ease;
}
.toast-enter-from,
.toast-leave-to {
  opacity: 0;
  transform: translateY(8px);
}
</style>
