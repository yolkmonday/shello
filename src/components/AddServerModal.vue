<script setup lang="ts">
import { ref, computed } from "vue";
import { open } from "@tauri-apps/plugin-dialog";
import { useProfilesStore, type Group } from "../stores/profiles";
import {
  useProfileForm,
  type ProfileFormEmits,
  type ProfileFormPrefill,
} from "../lib/use-profile-form";

const props = defineProps<{
  prefill?: ProfileFormPrefill;
}>();

const emit = defineEmits<{
  close: [];
  saved: [];
  connected: [sessionId: string];
}>();

const emitFns: ProfileFormEmits = {
  close: () => emit("close"),
  saved: () => emit("saved"),
  connected: (id: string) => emit("connected", id),
};

const form = useProfileForm(emitFns, undefined, props.prefill);
const profilesStore = useProfilesStore();

// Group combobox (simple version)
const groupDropdownOpen = ref(false);
const groupSearch = ref("");
const selectedGroupName = computed(() => {
  if (!form.groupId.value) return "";
  const g = profilesStore.sortedGroups.find((g: Group) => g.id === form.groupId.value);
  return g?.name ?? "";
});
const filteredGroups = computed(() => {
  const q = groupSearch.value.toLowerCase();
  if (!q) return profilesStore.sortedGroups;
  return profilesStore.sortedGroups.filter((g: Group) =>
    g.name.toLowerCase().includes(q),
  );
});

// Inline group creation
const creatingGroup = ref(false);
const newGroupName = ref("");
const newGroupColor = ref("#475569");
const groupColors = [
  "#475569", "#ef4444", "#f97316", "#eab308",
  "#22c55e", "#06b6d4", "#3b82f6", "#a855f7",
];

function selectGroup(id: string | null) {
  form.groupId.value = id;
  groupSearch.value = "";
  groupDropdownOpen.value = false;
}

async function createGroupInline() {
  if (!newGroupName.value.trim()) return;
  try {
    const group = await profilesStore.createGroup(
      newGroupName.value.trim(),
      newGroupColor.value,
    );
    form.groupId.value = group.id;
    resetGroupForm();
  } catch (e) {
    form.error.value = String(e);
  }
}

function resetGroupForm() {
  newGroupName.value = "";
  newGroupColor.value = "#475569";
  creatingGroup.value = false;
}

async function browseKeyFile() {
  const selected = await open({
    title: "Select SSH Key",
    multiple: false,
    filters: [{ name: "SSH Keys", extensions: ["pem", "key", "pub", ""] }],
  });
  if (selected) form.keyPath.value = selected;
}
</script>

<template>
  <Transition name="modal">
    <div
      class="fixed inset-0 z-50 flex items-center justify-center"
      @click.self="emit('close')"
      @keydown.escape="emit('close')"
    >
      <div class="absolute inset-0 bg-black/60 backdrop-blur-sm" @click="emit('close')"></div>
      <div
        class="relative w-full max-w-md bg-otter-card border border-otter-border
               rounded-xl shadow-2xl overflow-hidden animate-modal-in"
        @click="groupDropdownOpen = false"
      >
        <!-- Header -->
        <div class="flex items-center justify-between px-5 pt-5 pb-3">
          <h2 class="text-sm font-semibold text-otter-text">Add Server</h2>
          <button
            class="text-otter-subtle hover:text-otter-text transition-colors text-lg leading-none"
            @click="emit('close')"
          >
            &times;
          </button>
        </div>

        <!-- Form -->
        <div class="px-5 pb-5 flex flex-col gap-3">
          <!-- Name -->
          <input
            v-model="form.name.value"
            class="px-3 py-2 rounded-lg bg-otter-surface border border-otter-border
                   text-otter-text placeholder-otter-subtle text-sm
                   focus:outline-none focus:border-otter-teal-dim transition-colors"
            placeholder="Display name (e.g. Prod Server 1)"
          />

          <!-- Host + Port -->
          <div class="flex gap-2">
            <input
              v-model="form.host.value"
              class="flex-1 px-3 py-2 rounded-lg bg-otter-surface border border-otter-border
                     text-otter-text placeholder-otter-subtle text-sm
                     focus:outline-none focus:border-otter-teal-dim transition-colors"
              placeholder="Host"
            />
            <input
              v-model.number="form.port.value"
              type="number"
              class="w-20 px-3 py-2 rounded-lg bg-otter-surface border border-otter-border
                     text-otter-text placeholder-otter-subtle text-sm port-input
                     focus:outline-none focus:border-otter-teal-dim transition-colors"
              placeholder="22"
            />
          </div>

          <!-- Username -->
          <input
            v-model="form.username.value"
            class="px-3 py-2 rounded-lg bg-otter-surface border border-otter-border
                   text-otter-text placeholder-otter-subtle text-sm
                   focus:outline-none focus:border-otter-teal-dim transition-colors"
            placeholder="Username"
          />

          <!-- Auth type toggle -->
          <div class="flex gap-2">
            <button
              v-for="t in (['password', 'key'] as const)"
              :key="t"
              :class="[
                'flex-1 py-1.5 rounded-lg text-xs transition-colors',
                form.authType.value === t
                  ? 'bg-otter-teal text-otter-dark'
                  : 'bg-otter-surface text-otter-muted border border-otter-border',
              ]"
              @click="form.authType.value = t"
            >
              {{ t === "password" ? "Password" : "Key File" }}
            </button>
          </div>

          <!-- Password field -->
          <input
            v-if="form.authType.value === 'password'"
            v-model="form.password.value"
            type="password"
            class="px-3 py-2 rounded-lg bg-otter-surface border border-otter-border
                   text-otter-text placeholder-otter-subtle text-sm
                   focus:outline-none focus:border-otter-teal-dim transition-colors"
            placeholder="Password"
            @keyup.enter="form.saveAndConnect()"
          />

          <!-- Key fields -->
          <template v-if="form.authType.value === 'key'">
            <div class="flex gap-2">
              <input
                v-model="form.keyPath.value"
                class="flex-1 px-3 py-2 rounded-lg bg-otter-surface border border-otter-border
                       text-otter-text placeholder-otter-subtle text-sm
                       focus:outline-none focus:border-otter-teal-dim transition-colors"
                placeholder="Path to private key (~/.ssh/id_ed25519)"
              />
              <button
                type="button"
                class="px-3 py-2 rounded-lg bg-otter-surface border border-otter-border
                       text-otter-muted text-xs hover:text-otter-text hover:border-otter-teal-dim
                       transition-colors flex-shrink-0"
                @click="browseKeyFile"
              >
                Browse
              </button>
            </div>
            <input
              v-model="form.passphrase.value"
              type="password"
              class="px-3 py-2 rounded-lg bg-otter-surface border border-otter-border
                     text-otter-text placeholder-otter-subtle text-sm
                     focus:outline-none focus:border-otter-teal-dim transition-colors"
              placeholder="Passphrase (optional)"
              @keyup.enter="form.saveAndConnect()"
            />
          </template>

          <!-- Group + Tags -->
          <div class="flex gap-2">
            <!-- Group combobox -->
            <div class="relative flex-1">
              <div
                class="flex items-center gap-2 px-3 py-2 rounded-lg bg-otter-surface border
                       text-sm cursor-pointer transition-colors"
                :class="groupDropdownOpen ? 'border-otter-teal-dim' : 'border-otter-border'"
                @click.stop="groupDropdownOpen = !groupDropdownOpen"
              >
                <div
                  v-if="form.groupId.value && selectedGroupName"
                  class="w-2 h-2 rounded-full flex-shrink-0"
                  :style="{ backgroundColor: profilesStore.sortedGroups.find((g: Group) => g.id === form.groupId.value)?.color }"
                ></div>
                <span :class="form.groupId.value ? 'text-otter-text' : 'text-otter-subtle'">
                  {{ selectedGroupName || 'Group' }}
                </span>
                <svg class="w-3 h-3 ml-auto text-otter-subtle transition-transform" :class="{ 'rotate-180': groupDropdownOpen }" viewBox="0 0 12 12" fill="none" stroke="currentColor" stroke-width="1.5">
                  <path d="M3 4.5l3 3 3-3" />
                </svg>
              </div>

              <!-- Dropdown -->
              <div
                v-if="groupDropdownOpen"
                class="absolute left-0 right-0 top-full mt-1 z-10 rounded-lg bg-otter-card border border-otter-border
                       shadow-lg overflow-hidden"
              >
                <input
                  v-model="groupSearch"
                  class="w-full px-3 py-2 bg-otter-surface border-b border-otter-border
                         text-otter-text placeholder-otter-subtle text-xs
                         focus:outline-none"
                  placeholder="Search groups..."
                  @click.stop
                />
                <div class="max-h-40 overflow-y-auto">
                  <button
                    class="w-full px-3 py-2 text-left text-xs transition-colors
                           hover:bg-otter-surface/50"
                    :class="!form.groupId.value ? 'text-otter-teal' : 'text-otter-muted'"
                    @click.stop="selectGroup(null)"
                  >
                    No group
                  </button>
                  <button
                    v-for="group in filteredGroups"
                    :key="group.id"
                    class="w-full px-3 py-2 text-left text-xs flex items-center gap-2
                           hover:bg-otter-surface/50 transition-colors"
                    :class="form.groupId.value === group.id ? 'text-otter-teal' : 'text-otter-text'"
                    @click.stop="selectGroup(group.id)"
                  >
                    <div class="w-2 h-2 rounded-full flex-shrink-0" :style="{ backgroundColor: group.color }"></div>
                    {{ group.name }}
                  </button>

                  <!-- Inline create -->
                  <div v-if="creatingGroup" class="px-3 py-2 border-t border-otter-border" @click.stop>
                    <input
                      v-model="newGroupName"
                      class="w-full px-2 py-1 mb-2 rounded bg-otter-surface border border-otter-border
                             text-otter-text placeholder-otter-subtle text-xs
                             focus:outline-none focus:border-otter-teal-dim"
                      placeholder="Group name"
                      @keyup.enter="createGroupInline"
                    />
                    <div class="flex gap-1 mb-2">
                      <button
                        v-for="c in groupColors"
                        :key="c"
                        class="w-4 h-4 rounded-full border-2 transition-colors"
                        :class="newGroupColor === c ? 'border-white' : 'border-transparent'"
                        :style="{ backgroundColor: c }"
                        @click="newGroupColor = c"
                      ></button>
                    </div>
                    <div class="flex gap-1">
                      <button
                        class="flex-1 py-1 rounded text-xs bg-otter-teal text-otter-dark font-medium
                               disabled:opacity-50"
                        :disabled="!newGroupName.trim()"
                        @click="createGroupInline"
                      >Add</button>
                      <button
                        class="flex-1 py-1 rounded text-xs text-otter-muted hover:text-otter-text"
                        @click="resetGroupForm"
                      >Cancel</button>
                    </div>
                  </div>
                  <button
                    v-else
                    class="w-full px-3 py-2 text-left text-xs text-otter-teal-dim
                           hover:bg-otter-surface/50 transition-colors border-t border-otter-border"
                    @click.stop="creatingGroup = true"
                  >
                    + New group
                  </button>
                </div>
              </div>
            </div>

            <!-- Tags -->
            <input
              v-model="form.tags.value"
              class="flex-1 px-3 py-2 rounded-lg bg-otter-surface border border-otter-border
                     text-otter-text placeholder-otter-subtle text-sm
                     focus:outline-none focus:border-otter-teal-dim transition-colors"
              placeholder="Tags (prod, nginx)"
              @keyup.enter="form.saveAndConnect()"
            />
          </div>

          <!-- Action buttons -->
          <div class="flex gap-2 pt-1">
            <button
              class="flex-1 py-2 rounded-lg bg-otter-teal text-otter-dark font-semibold
                     text-sm hover:opacity-90 transition-opacity disabled:opacity-50"
              :disabled="form.connecting.value || form.saving.value || !form.host.value || !form.username.value"
              @click="form.saveAndConnect()"
            >
              {{ form.connecting.value ? "Connecting..." : "Save & Connect" }}
            </button>
            <button
              class="flex-1 py-2 rounded-lg bg-otter-surface border border-otter-border
                     text-otter-text text-sm hover:border-otter-subtle
                     transition-colors disabled:opacity-50"
              :disabled="form.connecting.value || form.saving.value || !form.name.value || !form.host.value || !form.username.value"
              @click="form.save()"
            >
              {{ form.saving.value ? "Saving..." : "Save" }}
            </button>
          </div>

          <!-- Error -->
          <p v-if="form.error.value" class="text-sm text-otter-coral break-all">
            {{ form.error.value }}
          </p>
        </div>
      </div>
    </div>
  </Transition>
</template>

<style scoped>
@keyframes modal-in {
  from {
    opacity: 0;
    transform: scale(0.95) translateY(8px);
  }
  to {
    opacity: 1;
    transform: scale(1) translateY(0);
  }
}
.animate-modal-in {
  animation: modal-in 0.2s ease-out;
}

.modal-enter-active {
  transition: opacity 0.2s ease-out;
}
.modal-leave-active {
  transition: opacity 0.15s ease-in;
}
.modal-enter-from,
.modal-leave-to {
  opacity: 0;
}

/* Hide number input spinner arrows */
.port-input::-webkit-outer-spin-button,
.port-input::-webkit-inner-spin-button {
  -webkit-appearance: none;
  margin: 0;
}
.port-input {
  -moz-appearance: textfield;
}
</style>
