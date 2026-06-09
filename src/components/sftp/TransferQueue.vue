<script setup lang="ts">
import { computed } from "vue";
import { Icon } from "@iconify/vue";
import { useSftpStore, type Transfer } from "../../stores/sftp";

const store = useSftpStore();

const transfers = computed<Transfer[]>(() => store.transfers);
const hasFinished = computed(() =>
  transfers.value.some((t) => ["done", "failed", "cancelled"].includes(t.status)),
);

function pct(t: Transfer): number {
  if (t.total === 0) return t.status === "done" ? 100 : 0;
  return Math.min(100, Math.round((t.transferred / t.total) * 100));
}

function formatSpeed(bytesPerSec: number): string {
  if (bytesPerSec <= 0) return "";
  const units = ["B/s", "KB/s", "MB/s", "GB/s"];
  let v = bytesPerSec;
  let i = 0;
  while (v >= 1024 && i < units.length - 1) {
    v /= 1024;
    i++;
  }
  return `${v.toFixed(1)} ${units[i]}`;
}

function statusColor(t: Transfer): string {
  switch (t.status) {
    case "done":
      return "bg-otter-teal";
    case "failed":
      return "bg-red-500";
    case "cancelled":
      return "bg-otter-subtle";
    default:
      return "bg-otter-blue";
  }
}
</script>

<template>
  <div class="border-t border-otter-border bg-otter-card/60 flex flex-col max-h-44">
    <div class="flex items-center justify-between px-3 py-1 border-b border-otter-border">
      <span class="text-xs text-otter-muted">Transfers ({{ transfers.length }})</span>
      <button
        v-if="hasFinished"
        class="text-[11px] text-otter-subtle hover:text-otter-text"
        @click="store.clearFinished()"
      >
        Clear finished
      </button>
    </div>

    <div class="flex-1 overflow-y-auto">
      <div v-if="transfers.length === 0" class="px-3 py-3 text-center text-[11px] text-otter-subtle">
        No transfers
      </div>

      <div
        v-for="t in transfers"
        :key="t.id"
        class="flex items-center gap-2 px-3 py-1.5 border-b border-otter-border/50"
      >
        <Icon
          :icon="t.direction === 'download' ? 'mdi:download' : 'mdi:upload'"
          class="w-3.5 h-3.5 flex-shrink-0 text-otter-muted"
        />
        <span class="text-xs text-otter-text truncate w-40 flex-shrink-0" :title="t.name">{{ t.name }}</span>

        <div class="flex-1 h-1.5 rounded-full bg-otter-surface overflow-hidden">
          <div class="h-full transition-all" :class="statusColor(t)" :style="{ width: pct(t) + '%' }"></div>
        </div>

        <span class="text-[10px] text-otter-subtle w-9 text-right flex-shrink-0">{{ pct(t) }}%</span>
        <span class="text-[10px] text-otter-subtle w-16 text-right flex-shrink-0 hidden md:inline">
          {{ t.status === 'active' ? formatSpeed(t.speed) : t.status }}
        </span>

        <button
          v-if="t.status === 'active' || t.status === 'queued'"
          class="p-0.5 rounded hover:bg-otter-surface text-otter-muted hover:text-red-400 flex-shrink-0"
          title="Cancel"
          @click="store.cancel(t.id)"
        >
          <Icon icon="mdi:close" class="w-3.5 h-3.5" />
        </button>
        <Icon
          v-else-if="t.status === 'done'"
          icon="mdi:check"
          class="w-3.5 h-3.5 text-otter-teal flex-shrink-0"
        />
        <Icon
          v-else-if="t.status === 'failed'"
          icon="mdi:alert-circle-outline"
          class="w-3.5 h-3.5 text-red-400 flex-shrink-0"
          :title="t.error"
        />
      </div>
    </div>
  </div>
</template>
