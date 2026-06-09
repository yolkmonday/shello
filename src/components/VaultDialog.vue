<script setup lang="ts">
import { ref, computed } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { useProfilesStore } from "../stores/profiles";

const props = defineProps<{
  mode: "setup" | "unlock" | "change";
}>();

const emit = defineEmits<{
  close: [];
  unlocked: [];
}>();

const profilesStore = useProfilesStore();

const password = ref("");
const confirmPassword = ref("");
const currentPassword = ref("");
const remember = ref(false);
const error = ref("");
const loading = ref(false);

const isSetup = computed(() => props.mode === "setup");
const isChange = computed(() => props.mode === "change");

const title = computed(() => {
  if (isSetup.value) return "Create Master Password";
  if (isChange.value) return "Change Master Password";
  return "Unlock Vault";
});

const buttonText = computed(() => {
  if (loading.value) return "Working...";
  if (isSetup.value) return "Set Password";
  if (isChange.value) return "Change Password";
  return "Unlock";
});

const canSubmit = computed(() => {
  if (loading.value) return false;
  if (!password.value) return false;
  if (isChange.value && !currentPassword.value) return false;
  if ((isSetup.value || isChange.value) && password.value !== confirmPassword.value) return false;
  if ((isSetup.value || isChange.value) && password.value.length < 8) return false;
  return true;
});

async function submit() {
  error.value = "";
  loading.value = true;

  try {
    if (isSetup.value) {
      await invoke("vault_setup", {
        masterPassword: password.value,
        remember: remember.value,
      });
    } else if (isChange.value) {
      await invoke("vault_change_password", {
        currentPassword: currentPassword.value,
        newPassword: password.value,
      });
    } else {
      await invoke("vault_unlock", {
        masterPassword: password.value,
        remember: remember.value,
      });
    }
    await profilesStore.checkVaultStatus();
    emit("unlocked");
    emit("close");
  } catch (e) {
    error.value = String(e);
  } finally {
    loading.value = false;
  }
}
</script>

<template>
  <div
    class="fixed inset-0 bg-black/60 flex items-center justify-center z-50"
    @click.self="emit('close')"
  >
    <div class="bg-otter-card border border-otter-border rounded-2xl p-6 w-full max-w-sm">
      <div class="flex items-center justify-between mb-4">
        <h2 class="text-sm font-semibold text-otter-muted uppercase tracking-wider">
          {{ title }}
        </h2>
        <button
          class="text-otter-subtle hover:text-otter-text transition-colors text-lg"
          @click="emit('close')"
        >
          &times;
        </button>
      </div>

      <p v-if="isSetup" class="text-xs text-otter-subtle mb-3">
        This password encrypts your saved credentials. Minimum 8 characters.
      </p>

      <div class="flex flex-col gap-3">
        <input
          v-if="isChange"
          v-model="currentPassword"
          type="password"
          class="px-3 py-2 rounded-lg bg-otter-surface border border-otter-border
                 text-otter-text placeholder-otter-subtle text-sm
                 focus:outline-none focus:border-otter-teal-dim transition-colors"
          placeholder="Current password"
        />

        <input
          v-model="password"
          type="password"
          class="px-3 py-2 rounded-lg bg-otter-surface border border-otter-border
                 text-otter-text placeholder-otter-subtle text-sm
                 focus:outline-none focus:border-otter-teal-dim transition-colors"
          :placeholder="isSetup || isChange ? 'New password (min 8 chars)' : 'Master password'"
          @keyup.enter="canSubmit && submit()"
        />

        <input
          v-if="isSetup || isChange"
          v-model="confirmPassword"
          type="password"
          class="px-3 py-2 rounded-lg bg-otter-surface border border-otter-border
                 text-otter-text placeholder-otter-subtle text-sm
                 focus:outline-none focus:border-otter-teal-dim transition-colors"
          placeholder="Confirm password"
          @keyup.enter="canSubmit && submit()"
        />

        <label v-if="!isChange" class="flex items-center gap-2 cursor-pointer">
          <input
            v-model="remember"
            type="checkbox"
            class="accent-otter-teal"
          />
          <span class="text-xs text-otter-muted">Remember on this device</span>
        </label>

        <button
          class="w-full py-2 rounded-lg bg-otter-teal text-otter-dark font-semibold
                 text-sm hover:opacity-90 transition-opacity disabled:opacity-50"
          :disabled="!canSubmit"
          @click="submit"
        >
          {{ buttonText }}
        </button>
      </div>

      <p v-if="error" class="mt-3 text-sm text-otter-coral break-all">
        {{ error }}
      </p>

      <p
        v-if="(isSetup || isChange) && password && confirmPassword && password !== confirmPassword"
        class="mt-2 text-xs text-otter-coral"
      >
        Passwords don't match
      </p>
    </div>
  </div>
</template>
