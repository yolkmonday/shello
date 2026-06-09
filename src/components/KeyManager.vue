<script setup lang="ts">
import { ref, onMounted } from "vue";
import { invoke } from "@tauri-apps/api/core";

interface SshKeyInfo {
  name: string;
  path: string;
  key_type: string;
  has_public: boolean;
  public_key: string | null;
}

const emit = defineEmits<{ close: [] }>();

const keys = ref<SshKeyInfo[]>([]);
const generating = ref(false);
const newKeyName = ref("");
const newKeyType = ref("ed25519");
const newKeyPassphrase = ref("");
const copiedKey = ref<string | null>(null);
const error = ref("");

onMounted(loadKeys);

async function loadKeys() {
  try {
    keys.value = await invoke<SshKeyInfo[]>("ssh_list_keys");
  } catch (e) {
    error.value = String(e);
  }
}

async function generateKey() {
  if (!newKeyName.value.trim()) return;
  error.value = "";

  try {
    await invoke("ssh_generate_key", {
      name: newKeyName.value.trim(),
      keyType: newKeyType.value,
      passphrase: newKeyPassphrase.value || null,
    });
    newKeyName.value = "";
    newKeyPassphrase.value = "";
    generating.value = false;
    await loadKeys();
  } catch (e) {
    error.value = String(e);
  }
}

async function copyPublicKey(key: SshKeyInfo) {
  if (!key.public_key) return;
  await navigator.clipboard.writeText(key.public_key);
  copiedKey.value = key.name;
  setTimeout(() => { copiedKey.value = null; }, 2000);
}
</script>

<template>
  <div
    class="fixed inset-0 z-50 flex justify-end"
    @click.self="emit('close')"
  >
    <div class="absolute inset-0 bg-black/60" @click="emit('close')"></div>
    <div
      class="relative w-full max-w-md h-full bg-otter-card border-l border-otter-border
             overflow-y-auto animate-slide-in-right"
    >
      <div class="p-5">
        <div class="flex items-center justify-between mb-4">
          <h2 class="text-sm font-semibold text-otter-muted uppercase tracking-wider">
            SSH Keys
          </h2>
          <button
            class="text-otter-subtle hover:text-otter-text transition-colors text-lg"
            @click="emit('close')"
          >
            &times;
          </button>
        </div>

        <!-- Generate button -->
        <button
          v-if="!generating"
          class="w-full py-2 mb-4 rounded-lg border border-dashed border-otter-border
                 text-otter-muted text-xs hover:border-otter-teal-dim hover:text-otter-text
                 transition-colors"
          @click="generating = true"
        >
          + Generate New Key
        </button>

        <!-- Generate form -->
        <div v-if="generating" class="mb-4 p-3 rounded-lg bg-otter-surface border border-otter-border">
          <input
            v-model="newKeyName"
            class="w-full px-2 py-1.5 mb-2 rounded bg-otter-dark border border-otter-border
                   text-otter-text placeholder-otter-subtle text-xs font-mono
                   focus:outline-none focus:border-otter-teal-dim"
            placeholder="Key name (e.g. id_ed25519_work)"
          />
          <div class="flex gap-2 mb-2">
            <button
              v-for="t in ['ed25519', 'rsa', 'ecdsa']"
              :key="t"
              class="flex-1 py-1.5 rounded text-xs transition-colors"
              :class="newKeyType === t
                ? 'bg-otter-teal text-otter-dark'
                : 'bg-otter-dark border border-otter-border text-otter-muted'"
              @click="newKeyType = t"
            >
              {{ t.toUpperCase() }}
            </button>
          </div>
          <input
            v-model="newKeyPassphrase"
            type="password"
            class="w-full px-2 py-1.5 mb-2 rounded bg-otter-dark border border-otter-border
                   text-otter-text placeholder-otter-subtle text-xs
                   focus:outline-none focus:border-otter-teal-dim"
            placeholder="Passphrase (optional)"
          />
          <div class="flex gap-2">
            <button
              class="flex-1 py-1.5 rounded bg-otter-teal text-otter-dark text-xs font-semibold
                     hover:opacity-90 disabled:opacity-50"
              :disabled="!newKeyName.trim()"
              @click="generateKey"
            >
              Generate
            </button>
            <button
              class="flex-1 py-1.5 rounded bg-otter-dark border border-otter-border text-otter-text
                     text-xs hover:border-otter-subtle"
              @click="generating = false"
            >
              Cancel
            </button>
          </div>
        </div>

        <p v-if="error" class="text-xs text-otter-coral mb-3 break-all">{{ error }}</p>

        <!-- Key list -->
        <div class="flex flex-col gap-2">
          <div
            v-for="key in keys"
            :key="key.name"
            class="p-3 rounded-lg bg-otter-surface border border-otter-border"
          >
            <div class="flex items-center gap-2 mb-1">
              <svg class="w-4 h-4 text-otter-teal flex-shrink-0" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5"><circle cx="5" cy="11" r="3"/><path d="M7.5 8.5L14 2M12 2h2v2"/></svg>
              <span class="text-xs font-semibold text-otter-text">{{ key.name }}</span>
              <span class="ml-auto px-1.5 py-0.5 rounded bg-otter-dark text-[10px] text-otter-muted">
                {{ key.key_type }}
              </span>
            </div>
            <p class="text-[10px] text-otter-subtle font-mono truncate mb-2">{{ key.path }}</p>

            <div v-if="key.has_public" class="flex items-center gap-2">
              <div class="flex-1 px-2 py-1 rounded bg-otter-dark text-[10px] text-otter-muted font-mono truncate">
                {{ key.public_key }}
              </div>
              <button
                class="px-2 py-1 rounded text-[10px] transition-colors flex-shrink-0"
                :class="copiedKey === key.name
                  ? 'bg-otter-teal text-otter-dark'
                  : 'bg-otter-dark border border-otter-border text-otter-muted hover:text-otter-text'"
                @click="copyPublicKey(key)"
              >
                {{ copiedKey === key.name ? 'Copied!' : 'Copy' }}
              </button>
            </div>
          </div>

          <p
            v-if="keys.length === 0"
            class="text-center text-xs text-otter-subtle py-6"
          >
            No SSH keys found in ~/.ssh/
          </p>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
@keyframes slide-in-right {
  from { transform: translateX(100%); }
  to { transform: translateX(0); }
}
.animate-slide-in-right {
  animation: slide-in-right 0.25s ease-out;
}
</style>
