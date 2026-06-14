<script setup lang="ts">
import { ref, computed, watch } from "vue";
import { Icon } from "@iconify/vue";
import { useTunnelsStore, type ActiveTunnel } from "../stores/tunnels";

const props = defineProps<{ open: boolean; sessionId: string }>();
const emit = defineEmits<{ close: [] }>();

const store = useTunnelsStore();

const tunnels = computed<ActiveTunnel[]>(() => store.active[props.sessionId] ?? []);

const adding = ref(false);
const localPort = ref<number | null>(null);
const remoteHost = ref("");
const remotePort = ref<number | null>(null);
const formError = ref("");

watch(
  () => props.open,
  (isOpen) => {
    if (isOpen) store.refreshActive(props.sessionId);
  },
);

function statusColor(s: ActiveTunnel["status"]): string {
  if (s === "active") return "bg-otter-teal";
  if (s === "error") return "bg-red-500";
  if (s === "starting") return "bg-amber-400";
  return "bg-otter-subtle";
}

async function addTunnel() {
  formError.value = "";
  if (!localPort.value || !remoteHost.value.trim() || !remotePort.value) {
    formError.value = "All fields are required";
    return;
  }
  await store.start(props.sessionId, {
    tunnel_id: crypto.randomUUID(),
    local_host: "127.0.0.1",
    local_port: localPort.value,
    remote_host: remoteHost.value.trim(),
    remote_port: remotePort.value,
  });
  localPort.value = null;
  remoteHost.value = "";
  remotePort.value = null;
  adding.value = false;
}
</script>

<template>
  <div
    v-if="open"
    class="fixed inset-0 bg-black/60 flex items-center justify-center z-50 p-4"
    @click.self="emit('close')"
  >
    <div class="bg-otter-card border border-otter-border rounded-2xl w-full max-w-md flex flex-col max-h-[80vh]">
      <!-- Header -->
      <div class="flex items-center gap-2 px-5 py-4 border-b border-otter-border">
        <Icon icon="mdi:transit-connection-variant" class="w-5 h-5 text-otter-teal" />
        <h2 class="text-base font-semibold text-otter-text">Port forwarding</h2>
        <button class="ml-auto p-1 rounded hover:bg-otter-surface text-otter-muted hover:text-otter-text" @click="emit('close')">
          <Icon icon="mdi:close" class="w-4 h-4" />
        </button>
      </div>

      <!-- Active tunnels -->
      <div class="flex-1 overflow-y-auto px-2 py-2">
        <div v-if="tunnels.length === 0" class="px-3 py-6 text-center text-sm text-otter-subtle">
          No active tunnels
        </div>
        <div
          v-for="t in tunnels"
          :key="t.tunnel_id"
          class="flex items-center gap-3 px-3 py-2 rounded-lg hover:bg-otter-surface/40"
        >
          <span class="w-2 h-2 rounded-full flex-shrink-0" :class="statusColor(t.status)"></span>
          <div class="min-w-0 flex-1">
            <p class="text-sm text-otter-text font-mono truncate">
              localhost:{{ t.local_port }} → {{ t.remote_host }}:{{ t.remote_port }}
            </p>
            <p v-if="t.status === 'error'" class="text-[11px] text-red-400 truncate" :title="t.error">{{ t.error }}</p>
            <p v-else class="text-[11px] text-otter-subtle capitalize">{{ t.status }}</p>
          </div>
          <button
            v-if="t.status === 'error'"
            class="p-1 rounded hover:bg-otter-surface text-otter-muted hover:text-otter-teal"
            title="Retry"
            @click="store.retry(sessionId, t.tunnel_id)"
          >
            <Icon icon="mdi:refresh" class="w-4 h-4" />
          </button>
          <button
            class="p-1 rounded hover:bg-otter-surface text-otter-muted hover:text-red-400"
            title="Stop"
            @click="store.stop(sessionId, t.tunnel_id)"
          >
            <Icon icon="mdi:stop-circle-outline" class="w-4 h-4" />
          </button>
        </div>
      </div>

      <!-- Add ad-hoc -->
      <div class="border-t border-otter-border px-5 py-3">
        <button
          v-if="!adding"
          class="text-sm text-otter-teal hover:underline flex items-center gap-1"
          @click="adding = true"
        >
          <Icon icon="mdi:plus" class="w-4 h-4" /> Add tunnel
        </button>
        <div v-else class="flex flex-col gap-2">
          <div class="flex items-center gap-2 text-xs text-otter-muted font-mono">
            <span>localhost:</span>
            <input v-model.number="localPort" type="number" placeholder="5432" class="w-20 bg-otter-dark border border-otter-border rounded px-2 py-1 text-otter-text focus:outline-none focus:border-otter-teal-dim" />
            <span>→</span>
            <input v-model="remoteHost" placeholder="db.internal" class="flex-1 bg-otter-dark border border-otter-border rounded px-2 py-1 text-otter-text focus:outline-none focus:border-otter-teal-dim" />
            <span>:</span>
            <input v-model.number="remotePort" type="number" placeholder="5432" class="w-20 bg-otter-dark border border-otter-border rounded px-2 py-1 text-otter-text focus:outline-none focus:border-otter-teal-dim" />
          </div>
          <p v-if="formError" class="text-[11px] text-red-400">{{ formError }}</p>
          <div class="flex items-center gap-2">
            <button class="px-3 py-1.5 rounded-lg bg-otter-teal text-otter-dark text-xs font-medium hover:opacity-90" @click="addTunnel">Start</button>
            <button class="px-3 py-1.5 rounded-lg text-xs text-otter-muted hover:text-otter-text hover:bg-otter-surface" @click="adding = false">Cancel</button>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>
