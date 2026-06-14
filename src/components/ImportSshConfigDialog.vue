<script setup lang="ts">
import { ref, watch, computed } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { Icon } from "@iconify/vue";
import { useProfilesStore, type CreateProfileInput } from "../stores/profiles";

interface ParsedHost {
  alias: string;
  hostname: string | null;
  user: string | null;
  port: number | null;
  identity_file: string | null;
  proxy_jump: string | null;
}

interface Row {
  host: ParsedHost;
  mappedHost: string;
  mappedPort: number;
  mappedUser: string;
  exists: boolean;
  selected: boolean;
}

const props = defineProps<{ open: boolean }>();
const emit = defineEmits<{ close: []; imported: [count: number] }>();

const store = useProfilesStore();
const rows = ref<Row[]>([]);
const loading = ref(false);
const importing = ref(false);
const error = ref("");

const selectedCount = computed(() => rows.value.filter((r) => r.selected && !r.exists).length);

watch(
  () => props.open,
  async (isOpen) => {
    if (!isOpen) return;
    error.value = "";
    rows.value = [];
    loading.value = true;
    try {
      const hosts = await invoke<ParsedHost[]>("ssh_config_parse");
      rows.value = hosts.map((host) => {
        const mappedHost = host.hostname || host.alias;
        const mappedPort = host.port ?? 22;
        const mappedUser = host.user ?? "";
        const exists = Object.values(store.profiles).some(
          (p) =>
            p.host.toLowerCase() === mappedHost.toLowerCase() &&
            p.port === mappedPort &&
            p.username === mappedUser,
        );
        return { host, mappedHost, mappedPort, mappedUser, exists, selected: !exists };
      });
    } catch (e) {
      error.value = String(e);
    } finally {
      loading.value = false;
    }
  },
);

async function runImport() {
  const toImport = rows.value.filter((r) => r.selected && !r.exists);
  if (toImport.length === 0) return;
  importing.value = true;
  error.value = "";
  let ok = 0;
  for (const r of toImport) {
    const input: CreateProfileInput = {
      name: r.host.alias,
      host: r.mappedHost,
      port: r.mappedPort,
      username: r.mappedUser,
      auth_type: r.host.identity_file ? "key" : "password",
      key_path: r.host.identity_file ?? undefined,
      tags: r.host.proxy_jump ? `via:${r.host.proxy_jump}` : undefined,
      group_id: null,
    };
    try {
      await store.createProfile(input);
      ok++;
    } catch {
      // Skip a failing host; keep importing the rest.
    }
  }
  importing.value = false;
  emit("imported", ok);
  emit("close");
}
</script>

<template>
  <div
    v-if="open"
    class="fixed inset-0 bg-black/60 flex items-center justify-center z-50 p-4"
    @click.self="emit('close')"
  >
    <div class="bg-otter-card border border-otter-border rounded-2xl w-full max-w-lg flex flex-col max-h-[80vh]">
      <!-- Header -->
      <div class="flex items-center gap-2 px-5 py-4 border-b border-otter-border">
        <Icon icon="mdi:file-import-outline" class="w-5 h-5 text-otter-teal" />
        <h2 class="text-base font-semibold text-otter-text">Import from ~/.ssh/config</h2>
        <button class="ml-auto p-1 rounded hover:bg-otter-surface text-otter-muted hover:text-otter-text" @click="emit('close')">
          <Icon icon="mdi:close" class="w-4 h-4" />
        </button>
      </div>

      <!-- Body -->
      <div class="flex-1 overflow-y-auto px-2 py-2">
        <div v-if="loading" class="px-3 py-8 text-center text-sm text-otter-subtle">Reading config…</div>
        <div v-else-if="error" class="px-3 py-6 text-center text-sm text-red-400">{{ error }}</div>
        <div v-else-if="rows.length === 0" class="px-3 py-8 text-center text-sm text-otter-subtle">
          No hosts found in ~/.ssh/config
        </div>

        <label
          v-for="r in rows"
          :key="r.host.alias + r.mappedHost"
          class="flex items-center gap-3 px-3 py-2 rounded-lg"
          :class="r.exists ? 'opacity-50 cursor-default' : 'cursor-pointer hover:bg-otter-surface/50'"
        >
          <input
            type="checkbox"
            v-model="r.selected"
            :disabled="r.exists"
            class="accent-otter-teal"
          />
          <Icon
            :icon="r.host.identity_file ? 'mdi:key-variant' : 'mdi:lock'"
            class="w-4 h-4 flex-shrink-0"
            :class="r.host.identity_file ? 'text-otter-teal' : 'text-amber-400'"
          />
          <div class="min-w-0 flex-1">
            <p class="text-sm text-otter-text truncate">{{ r.host.alias }}</p>
            <p class="text-xs text-otter-subtle truncate font-mono">
              {{ r.mappedUser ? r.mappedUser + '@' : '' }}{{ r.mappedHost }}:{{ r.mappedPort }}
              <span v-if="r.host.proxy_jump" class="text-otter-muted"> · via {{ r.host.proxy_jump }}</span>
            </p>
          </div>
          <span v-if="r.exists" class="text-[10px] text-otter-subtle flex-shrink-0">already exists</span>
        </label>
      </div>

      <!-- Footer -->
      <div class="flex items-center justify-end gap-2 px-5 py-3 border-t border-otter-border">
        <button
          class="px-3 py-1.5 rounded-lg text-sm text-otter-muted hover:text-otter-text hover:bg-otter-surface"
          @click="emit('close')"
        >
          Cancel
        </button>
        <button
          class="px-3 py-1.5 rounded-lg bg-otter-teal text-otter-dark text-sm font-medium hover:opacity-90 disabled:opacity-50"
          :disabled="selectedCount === 0 || importing"
          @click="runImport"
        >
          {{ importing ? 'Importing…' : `Import ${selectedCount} selected` }}
        </button>
      </div>
    </div>
  </div>
</template>
