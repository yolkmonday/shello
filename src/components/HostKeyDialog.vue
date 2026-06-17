<script setup lang="ts">
import { ref, onMounted } from "vue";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { Icon } from "@iconify/vue";

interface Prompt {
  id: string;
  host: string;
  port: number;
  fingerprint: string;
}

const current = ref<Prompt | null>(null);

onMounted(async () => {
  await listen<Prompt>("host-key-prompt", (e) => {
    current.value = e.payload;
  });
});

async function respond(accept: boolean) {
  if (!current.value) return;
  const id = current.value.id;
  current.value = null;
  await invoke("host_key_respond", { id, accept });
}
</script>

<template>
  <div
    v-if="current"
    class="fixed inset-0 bg-black/60 flex items-center justify-center z-[70] p-4"
  >
    <div class="bg-otter-card border border-otter-border rounded-2xl w-full max-w-md p-6">
      <div class="flex items-center gap-2 mb-3">
        <Icon icon="mdi:shield-key-outline" class="w-5 h-5 text-otter-amber" />
        <h2 class="text-base font-semibold text-otter-text">Unknown host key</h2>
      </div>
      <p class="text-sm text-otter-muted">
        The authenticity of
        <span class="text-otter-text font-mono">{{ current.host }}:{{ current.port }}</span>
        can't be established.
      </p>
      <p class="text-xs text-otter-subtle mt-3">SHA256 fingerprint</p>
      <p class="text-xs font-mono text-otter-text bg-otter-surface rounded-lg px-3 py-2 mt-1 break-all">
        {{ current.fingerprint }}
      </p>
      <p class="text-[11px] text-otter-subtle mt-3">
        Only accept if this matches the server's key. Accepting saves it to
        <span class="font-mono">~/.ssh/known_hosts</span>.
      </p>
      <div class="flex items-center justify-end gap-2 mt-5">
        <button
          class="px-3 py-1.5 rounded-lg text-sm text-otter-muted hover:text-otter-text hover:bg-otter-surface"
          @click="respond(false)"
        >
          Reject
        </button>
        <button
          class="px-3 py-1.5 rounded-lg bg-otter-teal text-otter-dark text-sm font-medium hover:opacity-90"
          @click="respond(true)"
        >
          Accept &amp; connect
        </button>
      </div>
    </div>
  </div>
</template>
