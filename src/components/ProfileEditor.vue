<script setup lang="ts">
import { ref, computed, onMounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { useProfilesStore, type ProfileSummary } from "../stores/profiles";
import { useTerminalStore } from "../stores/terminal";

const props = defineProps<{
  profile?: ProfileSummary;
  prefill?: { username: string; host: string; port: number };
}>();

const emit = defineEmits<{
  close: [];
  saved: [];
  connected: [sessionId: string];
}>();

const profilesStore = useProfilesStore();
const terminalStore = useTerminalStore();

const name = ref("");
const host = ref("");
const port = ref<number | undefined>(undefined);
const username = ref("");
const authType = ref<"password" | "key">("password");
const password = ref("");
const keyPath = ref("");
const passphrase = ref("");
const groupId = ref<string | null>(null);
const tags = ref("");
const saving = ref(false);
const connecting = ref(false);
const deleting = ref(false);
const confirmDelete = ref(false);
const error = ref("");

// Inline group creation & editing
const creatingGroup = ref(false);
const editingGroupId = ref<string | null>(null);
const confirmDeleteGroupId = ref<string | null>(null);
const newGroupName = ref("");
const newGroupColor = ref("#475569");
const groupColors = [
  "#475569", "#ef4444", "#f97316", "#eab308",
  "#22c55e", "#06b6d4", "#3b82f6", "#a855f7",
];

const isEditing = !!props.profile;

// Group combobox state
const groupDropdownOpen = ref(false);
const groupSearch = ref("");
const selectedGroupName = computed(() => {
  if (!groupId.value) return "";
  const g = profilesStore.sortedGroups.find((g) => g.id === groupId.value);
  return g?.name ?? "";
});
const filteredGroups = computed(() => {
  const q = groupSearch.value.toLowerCase();
  if (!q) return profilesStore.sortedGroups;
  return profilesStore.sortedGroups.filter((g) => g.name.toLowerCase().includes(q));
});
function selectGroup(id: string | null) {
  groupId.value = id;
  groupSearch.value = "";
  groupDropdownOpen.value = false;
}

async function createGroupInline() {
  if (!newGroupName.value.trim()) return;
  try {
    const group = await profilesStore.createGroup(newGroupName.value.trim(), newGroupColor.value);
    groupId.value = group.id;
    resetGroupForm();
  } catch (e) {
    error.value = String(e);
  }
}

function startEditGroup(group: { id: string; name: string; color: string }) {
  editingGroupId.value = group.id;
  newGroupName.value = group.name;
  newGroupColor.value = group.color;
  creatingGroup.value = false;
  confirmDeleteGroupId.value = null;
}

async function saveGroupEdit() {
  if (!editingGroupId.value || !newGroupName.value.trim()) return;
  try {
    await profilesStore.updateGroup(editingGroupId.value, newGroupName.value.trim(), newGroupColor.value);
    resetGroupForm();
  } catch (e) {
    error.value = String(e);
  }
}

async function deleteGroupConfirmed(id: string) {
  try {
    if (groupId.value === id) groupId.value = null;
    await profilesStore.deleteGroup(id);
    confirmDeleteGroupId.value = null;
  } catch (e) {
    error.value = String(e);
  }
}

function resetGroupForm() {
  newGroupName.value = "";
  newGroupColor.value = "#475569";
  creatingGroup.value = false;
  editingGroupId.value = null;
  confirmDeleteGroupId.value = null;
}

async function deleteServer() {
  if (!props.profile) return;
  deleting.value = true;
  try {
    await profilesStore.deleteProfile(props.profile.id);
    emit("close");
  } catch (e) {
    error.value = String(e);
  } finally {
    deleting.value = false;
  }
}

onMounted(() => {
  if (props.profile) {
    name.value = props.profile.name;
    host.value = props.profile.host;
    port.value = props.profile.port;
    username.value = props.profile.username;
    authType.value = props.profile.auth_type;
    groupId.value = props.profile.group_id;
    tags.value = props.profile.tags;
  } else if (props.prefill) {
    host.value = props.prefill.host;
    port.value = props.prefill.port;
    username.value = props.prefill.username;
  }
});

function buildProfileData() {
  return {
    name: name.value,
    host: host.value,
    port: port.value || 22,
    username: username.value,
    auth_type: authType.value,
    password: authType.value === "password" ? password.value || undefined : undefined,
    key_path: authType.value === "key" ? keyPath.value || undefined : undefined,
    passphrase: authType.value === "key" ? passphrase.value || undefined : undefined,
    group_id: groupId.value,
    tags: tags.value,
  };
}

async function save() {
  error.value = "";
  saving.value = true;

  try {
    if (isEditing) {
      await profilesStore.updateProfile(props.profile!.id, buildProfileData());
    } else {
      await profilesStore.createProfile(buildProfileData());
    }
    emit("saved");
    emit("close");
  } catch (e) {
    error.value = String(e);
  } finally {
    saving.value = false;
  }
}

async function saveAndConnect() {
  error.value = "";
  connecting.value = true;

  try {
    // Connect first — only save if connection succeeds
    const auth =
      authType.value === "password"
        ? { type: "password" as const, password: password.value }
        : {
            type: "key" as const,
            private_key_path: keyPath.value,
            passphrase: passphrase.value || null,
          };

    const sessionId = await invoke<string>("ssh_connect", {
      config: { host: host.value, port: port.value || 22, username: username.value, auth },
    });

    await invoke("ssh_open_pty", {
      sessionId,
      cols: 80,
      rows: 24,
    });

    // Connection succeeded — now save
    if (isEditing) {
      await profilesStore.updateProfile(props.profile!.id, buildProfileData());
    } else if (name.value) {
      await profilesStore.createProfile(buildProfileData());
    }

    terminalStore.addSession(sessionId, {
      id: sessionId,
      host: host.value,
      port: port.value || 22,
      username: username.value,
      connected_at: new Date().toISOString(),
      status: "connected",
    });

    emit("connected", sessionId);
    emit("close");
  } catch (e) {
    error.value = String(e);
  } finally {
    connecting.value = false;
  }
}
</script>

<template>
  <Transition name="panel">
    <div
      class="fixed inset-0 z-50 flex justify-end"
      @click.self="emit('close')"
    >
      <div class="absolute inset-0 bg-black/60" @click="emit('close')"></div>
      <div
        class="relative w-full max-w-md h-full bg-otter-card border-l border-otter-border
               overflow-y-auto animate-slide-in-right"
        @click="groupDropdownOpen = false"
      >
        <div class="p-6">
          <div class="flex items-center justify-between mb-6">
            <h2 class="text-sm font-semibold text-otter-muted uppercase tracking-wider">
              {{ isEditing ? "Edit Server" : "Add Server" }}
            </h2>
            <button
              class="text-otter-subtle hover:text-otter-text transition-colors text-lg"
              @click="emit('close')"
            >
              &times;
            </button>
          </div>

          <div class="flex flex-col gap-3">
            <input
              v-model="name"
              class="px-3 py-2 rounded-lg bg-otter-surface border border-otter-border
                     text-otter-text placeholder-otter-subtle text-sm
                     focus:outline-none focus:border-otter-teal-dim transition-colors"
              placeholder="Display name (e.g. Prod Server 1)"
            />

            <div class="flex gap-2">
              <input
                v-model="host"
                class="flex-1 px-3 py-2 rounded-lg bg-otter-surface border border-otter-border
                       text-otter-text placeholder-otter-subtle text-sm
                       focus:outline-none focus:border-otter-teal-dim transition-colors"
                placeholder="Host"
              />
              <input
                v-model.number="port"
                type="number"
                class="w-20 px-3 py-2 rounded-lg bg-otter-surface border border-otter-border
                       text-otter-text placeholder-otter-subtle text-sm port-input
                       focus:outline-none focus:border-otter-teal-dim transition-colors"
                placeholder="22"
              />
            </div>

            <input
              v-model="username"
              class="px-3 py-2 rounded-lg bg-otter-surface border border-otter-border
                     text-otter-text placeholder-otter-subtle text-sm
                     focus:outline-none focus:border-otter-teal-dim transition-colors"
              placeholder="Username"
            />

            <div class="flex gap-2">
              <button
                v-for="t in (['password', 'key'] as const)"
                :key="t"
                :class="[
                  'flex-1 py-1.5 rounded-lg text-xs transition-colors',
                  authType === t
                    ? 'bg-otter-teal text-otter-dark'
                    : 'bg-otter-surface text-otter-muted border border-otter-border',
                ]"
                @click="authType = t"
              >
                {{ t === "password" ? "Password" : "Key File" }}
              </button>
            </div>

            <input
              v-if="authType === 'password'"
              v-model="password"
              type="password"
              class="px-3 py-2 rounded-lg bg-otter-surface border border-otter-border
                     text-otter-text placeholder-otter-subtle text-sm
                     focus:outline-none focus:border-otter-teal-dim transition-colors"
              :placeholder="isEditing ? 'New password (leave empty to keep)' : 'Password'"
              @keyup.enter="saveAndConnect"
            />

            <template v-if="authType === 'key'">
              <input
                v-model="keyPath"
                class="px-3 py-2 rounded-lg bg-otter-surface border border-otter-border
                       text-otter-text placeholder-otter-subtle text-sm
                       focus:outline-none focus:border-otter-teal-dim transition-colors"
                placeholder="Path to private key (~/.ssh/id_ed25519)"
              />
              <input
                v-model="passphrase"
                type="password"
                class="px-3 py-2 rounded-lg bg-otter-surface border border-otter-border
                       text-otter-text placeholder-otter-subtle text-sm
                       focus:outline-none focus:border-otter-teal-dim transition-colors"
                placeholder="Passphrase (optional)"
                @keyup.enter="saveAndConnect"
              />
            </template>

            <!-- Group + Tags inline -->
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
                    v-if="groupId && selectedGroupName"
                    class="w-2 h-2 rounded-full flex-shrink-0"
                    :style="{ backgroundColor: profilesStore.sortedGroups.find(g => g.id === groupId)?.color }"
                  ></div>
                  <span :class="groupId ? 'text-otter-text' : 'text-otter-subtle'">
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
                      :class="!groupId ? 'text-otter-teal' : 'text-otter-muted'"
                      @click.stop="selectGroup(null)"
                    >
                      No group
                    </button>
                    <template v-for="group in filteredGroups" :key="group.id">
                      <!-- Edit form for this group -->
                      <div v-if="editingGroupId === group.id" class="px-3 py-2 border-t border-otter-border bg-otter-surface/30" @click.stop>
                        <input
                          v-model="newGroupName"
                          class="w-full px-2 py-1 mb-2 rounded bg-otter-surface border border-otter-border
                                 text-otter-text text-xs focus:outline-none focus:border-otter-teal-dim"
                          placeholder="Group name"
                          @keyup.enter="saveGroupEdit"
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
                            class="flex-1 py-1 rounded text-xs bg-otter-teal text-otter-dark font-medium disabled:opacity-50"
                            :disabled="!newGroupName.trim()"
                            @click="saveGroupEdit"
                          >Save</button>
                          <button
                            class="flex-1 py-1 rounded text-xs text-otter-muted hover:text-otter-text"
                            @click="resetGroupForm"
                          >Cancel</button>
                        </div>
                      </div>
                      <!-- Delete confirm for this group -->
                      <div v-else-if="confirmDeleteGroupId === group.id" class="px-3 py-2 border-t border-otter-border bg-otter-coral/5" @click.stop>
                        <p class="text-xs text-otter-coral mb-2">Delete "{{ group.name }}"?</p>
                        <div class="flex gap-1">
                          <button
                            class="flex-1 py-1 rounded text-xs bg-otter-coral text-white font-medium"
                            @click="deleteGroupConfirmed(group.id)"
                          >Delete</button>
                          <button
                            class="flex-1 py-1 rounded text-xs text-otter-muted hover:text-otter-text"
                            @click="confirmDeleteGroupId = null"
                          >Cancel</button>
                        </div>
                      </div>
                      <!-- Normal group row -->
                      <div v-else class="group/row flex items-center hover:bg-otter-surface/50 transition-colors">
                        <button
                          class="flex-1 px-3 py-2 text-left text-xs flex items-center gap-2"
                          :class="groupId === group.id ? 'text-otter-teal' : 'text-otter-text'"
                          @click.stop="selectGroup(group.id)"
                        >
                          <div class="w-2 h-2 rounded-full flex-shrink-0" :style="{ backgroundColor: group.color }"></div>
                          {{ group.name }}
                        </button>
                        <div class="hidden group-hover/row:flex items-center gap-0.5 pr-2">
                          <button
                            class="p-1 rounded text-otter-subtle hover:text-otter-text transition-colors"
                            title="Edit group"
                            @click.stop="startEditGroup(group)"
                          >
                            <svg class="w-3 h-3" viewBox="0 0 12 12" fill="none" stroke="currentColor" stroke-width="1.5">
                              <path d="M8.5 1.5l2 2M1 11l.7-2.8L9 1l2 2-7.2 7.2L1 11z" />
                            </svg>
                          </button>
                          <button
                            class="p-1 rounded text-otter-subtle hover:text-otter-coral transition-colors"
                            title="Delete group"
                            @click.stop="confirmDeleteGroupId = group.id"
                          >
                            <svg class="w-3 h-3" viewBox="0 0 12 12" fill="none" stroke="currentColor" stroke-width="1.5">
                              <path d="M2 3h8M4.5 3V2h3v1M3 3v7.5h6V3M5 5.5v3M7 5.5v3" />
                            </svg>
                          </button>
                        </div>
                      </div>
                    </template>

                    <!-- Inline create group -->
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
                      v-else-if="!editingGroupId"
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
                v-model="tags"
                class="flex-1 px-3 py-2 rounded-lg bg-otter-surface border border-otter-border
                       text-otter-text placeholder-otter-subtle text-sm
                       focus:outline-none focus:border-otter-teal-dim transition-colors"
                placeholder="Tags (prod, nginx)"
                @keyup.enter="saveAndConnect"
              />
            </div>

            <!-- Action buttons -->
            <div class="flex gap-2 pt-1">
              <button
                class="flex-1 py-2 rounded-lg bg-otter-teal text-otter-dark font-semibold
                       text-sm hover:opacity-90 transition-opacity disabled:opacity-50"
                :disabled="connecting || saving || !host || !username"
                @click="saveAndConnect"
              >
                {{ connecting ? "Connecting..." : isEditing ? "Connect" : "Save & Connect" }}
              </button>
              <button
                class="flex-1 py-2 rounded-lg bg-otter-surface border border-otter-border
                       text-otter-text text-sm hover:border-otter-subtle
                       transition-colors disabled:opacity-50"
                :disabled="connecting || saving || !name || !host || !username"
                @click="save"
              >
                {{ saving ? "Saving..." : isEditing ? "Update" : "Save" }}
              </button>
            </div>

            <!-- Delete server -->
            <template v-if="isEditing">
              <div v-if="confirmDelete" class="mt-4 p-3 rounded-lg border border-otter-coral/30 bg-otter-coral/5">
                <p class="text-xs text-otter-coral mb-2">Delete "{{ props.profile!.name || props.profile!.host }}"? This cannot be undone.</p>
                <div class="flex gap-2">
                  <button
                    class="flex-1 py-1.5 rounded-lg bg-otter-coral text-white text-xs font-medium
                           hover:opacity-90 transition-opacity disabled:opacity-50"
                    :disabled="deleting"
                    @click="deleteServer"
                  >
                    {{ deleting ? "Deleting..." : "Delete" }}
                  </button>
                  <button
                    class="flex-1 py-1.5 rounded-lg bg-otter-surface border border-otter-border
                           text-otter-text text-xs hover:border-otter-subtle transition-colors"
                    @click="confirmDelete = false"
                  >Cancel</button>
                </div>
              </div>
              <button
                v-else
                class="mt-4 w-full py-2 rounded-lg text-xs text-otter-coral/70 hover:text-otter-coral
                       hover:bg-otter-coral/5 transition-colors"
                @click="confirmDelete = true"
              >
                Delete server
              </button>
            </template>
          </div>

          <p v-if="error" class="mt-3 text-sm text-otter-coral break-all">
            {{ error }}
          </p>
        </div>
      </div>
    </div>
  </Transition>
</template>

<style scoped>
@keyframes slide-in-right {
  from { transform: translateX(100%); }
  to { transform: translateX(0); }
}
.animate-slide-in-right {
  animation: slide-in-right 0.25s ease-out;
}
.panel-leave-active .animate-slide-in-right {
  animation: slide-in-right 0.2s ease-in reverse;
}
.panel-enter-active {
  transition: opacity 0.25s ease-out;
}
.panel-leave-active {
  transition: opacity 0.2s ease-in;
}
.panel-enter-from,
.panel-leave-to {
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
