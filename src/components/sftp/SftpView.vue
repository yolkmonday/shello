<script setup lang="ts">
import { onMounted, ref } from "vue";
import { Icon } from "@iconify/vue";
import { useSftpStore } from "../../stores/sftp";
import FilePane from "./FilePane.vue";
import TransferQueue from "./TransferQueue.vue";

const props = defineProps<{ sessionId: string; title?: string }>();
defineEmits<{ close: [] }>();

const store = useSftpStore();
const loading = ref(true);
const error = ref("");

onMounted(async () => {
  try {
    await store.open(props.sessionId);
  } catch (e) {
    error.value = String(e);
  } finally {
    loading.value = false;
  }
});
</script>

<template>
  <div class="flex flex-col h-full bg-otter-dark">
    <!-- Header -->
    <div class="flex items-center gap-2 px-3 py-2 border-b border-otter-border bg-otter-card/50 flex-shrink-0">
      <Icon icon="mdi:folder-network-outline" class="w-4 h-4 text-otter-teal" />
      <span class="text-sm font-medium text-otter-text truncate">SFTP — {{ title || 'Server' }}</span>
      <button
        class="ml-auto p-1 rounded hover:bg-otter-surface text-otter-muted hover:text-otter-text"
        title="Close SFTP"
        @click="$emit('close')"
      >
        <Icon icon="mdi:close" class="w-4 h-4" />
      </button>
    </div>

    <div v-if="loading" class="flex-1 flex items-center justify-center text-otter-subtle text-sm">
      <span class="inline-block w-5 h-5 border-2 border-otter-teal border-t-transparent rounded-full animate-spin mr-2"></span>
      Opening SFTP…
    </div>

    <div v-else-if="error" class="flex-1 flex flex-col items-center justify-center text-sm text-red-400 gap-2">
      <span>Failed to open SFTP</span>
      <span class="text-xs text-otter-subtle max-w-md text-center">{{ error }}</span>
    </div>

    <template v-else>
      <div class="flex-1 grid grid-cols-2 gap-2 p-2 min-h-0">
        <FilePane side="local" :session-id="sessionId" />
        <FilePane side="remote" :session-id="sessionId" />
      </div>
      <TransferQueue />
    </template>
  </div>
</template>
