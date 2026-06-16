<script setup lang="ts">
import { ref, computed } from "vue";
import { open } from "@tauri-apps/plugin-dialog";
import { useProfilesStore, type ProfileSummary, type Group } from "../stores/profiles";
import { useTunnelsStore, type Tunnel } from "../stores/tunnels";
import {
  useProfileForm,
  type ProfileFormEmits,
  type ProfileFormPrefill,
} from "../lib/use-profile-form";
import SectionHeader from "./SectionHeader.vue";

const props = defineProps<{
  profile?: ProfileSummary;
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

const form = useProfileForm(emitFns, props.profile, props.prefill);
const profilesStore = useProfilesStore();
const tunnelsStore = useTunnelsStore();

// Section collapse state (all open by default)
const sections = ref({
  connection: true,
  auth: true,
  organization: true,
  tunnels: true,
});

// Port forwarding (edit-only)
const savedTunnels = computed<Tunnel[]>(() =>
  props.profile ? tunnelsStore.saved[props.profile.id] ?? [] : [],
);
const newTun = ref<{
  tunnel_type: "local" | "dynamic" | "remote";
  local_port: number | null;
  remote_host: string;
  remote_port: number | null;
}>({
  tunnel_type: "local",
  local_port: null,
  remote_host: "",
  remote_port: null,
});
const tunnelError = ref("");

async function addSavedTunnel() {
  tunnelError.value = "";
  if (!props.profile) return;
  const type = newTun.value.tunnel_type;
  if (type === "dynamic") {
    if (!newTun.value.local_port) {
      tunnelError.value = "Local port is required";
      return;
    }
  } else if (type === "remote") {
    if (!newTun.value.remote_port || !newTun.value.local_port) {
      tunnelError.value = "Server port and local target port are required";
      return;
    }
  } else if (!newTun.value.local_port || !newTun.value.remote_host.trim() || !newTun.value.remote_port) {
    tunnelError.value = "All tunnel fields are required";
    return;
  }
  try {
    await tunnelsStore.createSaved({
      profile_id: props.profile.id,
      tunnel_type: type,
      local_port: newTun.value.local_port!,
      remote_host: type === "local" ? newTun.value.remote_host.trim() : "",
      remote_port: type === "dynamic" ? 0 : newTun.value.remote_port!,
    });
    newTun.value = { tunnel_type: "local", local_port: null, remote_host: "", remote_port: null };
  } catch (e) {
    tunnelError.value = String(e);
  }
}

async function removeSavedTunnel(id: string) {
  if (props.profile) await tunnelsStore.removeSaved(props.profile.id, id);
}

async function toggleTunnelEnabled(t: Tunnel) {
  if (props.profile) await tunnelsStore.updateSaved(props.profile.id, t.id, { enabled: !t.enabled });
}

// Delete server (edit-only)
const deleting = ref(false);
const confirmDelete = ref(false);

async function deleteServer() {
  if (!props.profile) return;
  deleting.value = true;
  try {
    await profilesStore.deleteProfile(props.profile.id);
    emit("close");
  } catch (e) {
    form.error.value = String(e);
  } finally {
    deleting.value = false;
  }
}

// Group combobox (full CRUD version for edit)
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
  return profilesStore.sortedGroups.filter((g: Group) => g.name.toLowerCase().includes(q));
});
function selectGroup(id: string | null) {
  form.groupId.value = id;
  groupSearch.value = "";
  groupDropdownOpen.value = false;
}

const creatingGroup = ref(false);
const editingGroupId = ref<string | null>(null);
const confirmDeleteGroupId = ref<string | null>(null);
const newGroupName = ref("");
const newGroupColor = ref("#475569");
const groupColors = [
  "#475569", "#ef4444", "#f97316", "#eab308",
  "#22c55e", "#06b6d4", "#3b82f6", "#a855f7",
];

async function createGroupInline() {
  if (!newGroupName.value.trim()) return;
  try {
    const group = await profilesStore.createGroup(newGroupName.value.trim(), newGroupColor.value);
    form.groupId.value = group.id;
    resetGroupForm();
  } catch (e) {
    form.error.value = String(e);
  }
}

async function browseKeyFile() {
  const selected = await open({
    title: "Select SSH Key",
    multiple: false,
    filters: [{ name: "SSH Keys", extensions: ["pem", "key", "pub", ""] }],
  });
  if (selected) form.keyPath.value = selected;
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
    form.error.value = String(e);
  }
}

async function deleteGroupConfirmed(id: string) {
  try {
    if (form.groupId.value === id) form.groupId.value = null;
    await profilesStore.deleteGroup(id);
    confirmDeleteGroupId.value = null;
  } catch (e) {
    form.error.value = String(e);
  }
}

function resetGroupForm() {
  newGroupName.value = "";
  newGroupColor.value = "#475569";
  creatingGroup.value = false;
  editingGroupId.value = null;
  confirmDeleteGroupId.value = null;
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
          <!-- Header -->
          <div class="flex items-center justify-between mb-5">
            <h2 class="text-sm font-semibold text-otter-muted uppercase tracking-wider">
              {{ form.isEditing ? "Edit Server" : "Add Server" }}
            </h2>
            <button
              class="text-otter-subtle hover:text-otter-text transition-colors text-lg"
              @click="emit('close')"
            >
              &times;
            </button>
          </div>

          <div class="flex flex-col gap-1">
            <!-- ── Section: Connection ── -->
            <SectionHeader
              title="Connection"
              :open="sections.connection"
              @toggle="sections.connection = !sections.connection"
            />
            <Transition name="section">
              <div v-show="sections.connection" class="flex flex-col gap-3 pb-4">
                <input
                  v-model="form.name.value"
                  class="px-3 py-2 rounded-lg bg-otter-surface border border-otter-border
                         text-otter-text placeholder-otter-subtle text-sm
                         focus:outline-none focus:border-otter-teal-dim transition-colors"
                  placeholder="Display name (e.g. Prod Server 1)"
                />
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
                <input
                  v-model="form.username.value"
                  class="px-3 py-2 rounded-lg bg-otter-surface border border-otter-border
                         text-otter-text placeholder-otter-subtle text-sm
                         focus:outline-none focus:border-otter-teal-dim transition-colors"
                  placeholder="Username"
                />
              </div>
            </Transition>

            <!-- ── Section: Authentication ── -->
            <SectionHeader
              title="Authentication"
              :open="sections.auth"
              @toggle="sections.auth = !sections.auth"
            />
            <Transition name="section">
              <div v-show="sections.auth" class="flex flex-col gap-3 pb-4">
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

                <input
                  v-if="form.authType.value === 'password'"
                  v-model="form.password.value"
                  type="password"
                  class="px-3 py-2 rounded-lg bg-otter-surface border border-otter-border
                         text-otter-text placeholder-otter-subtle text-sm
                         focus:outline-none focus:border-otter-teal-dim transition-colors"
                  :placeholder="form.isEditing ? 'New password (leave empty to keep)' : 'Password'"
                  @keyup.enter="form.saveAndConnect()"
                />

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
              </div>
            </Transition>

            <!-- ── Section: Organization ── -->
            <SectionHeader
              title="Organization"
              :open="sections.organization"
              @toggle="sections.organization = !sections.organization"
            />
            <Transition name="section">
              <div v-show="sections.organization" class="flex flex-col gap-3 pb-4">
                <div class="flex gap-2">
                  <!-- Group combobox (full CRUD) -->
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
                              :class="form.groupId.value === group.id ? 'text-otter-teal' : 'text-otter-text'"
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
                    v-model="form.tags.value"
                    class="flex-1 px-3 py-2 rounded-lg bg-otter-surface border border-otter-border
                           text-otter-text placeholder-otter-subtle text-sm
                           focus:outline-none focus:border-otter-teal-dim transition-colors"
                    placeholder="Tags (prod, nginx)"
                    @keyup.enter="form.saveAndConnect()"
                  />
                </div>
              </div>
            </Transition>

            <!-- ── Section: Port Forwarding (edit-only) ── -->
            <template v-if="form.isEditing">
              <SectionHeader
                title="Port Forwarding"
                :open="sections.tunnels"
                :count="savedTunnels.length"
                @toggle="sections.tunnels = !sections.tunnels"
              />
              <Transition name="section">
                <div v-show="sections.tunnels" class="flex flex-col gap-2 pb-4">
                  <label class="text-xs text-otter-muted">Auto-start on connect</label>
                  <div
                    v-for="t in savedTunnels"
                    :key="t.id"
                    class="flex items-center gap-2 px-3 py-2 rounded-lg bg-otter-surface/50 border border-otter-border"
                  >
                    <input type="checkbox" :checked="t.enabled" class="accent-otter-teal" @change="toggleTunnelEnabled(t)" />
                    <span class="text-xs font-mono text-otter-text flex-1 truncate">
                      <template v-if="t.tunnel_type === 'dynamic'">SOCKS5 localhost:{{ t.local_port }}</template>
                      <template v-else-if="t.tunnel_type === 'remote'">remote:{{ t.remote_port }} → localhost:{{ t.local_port }}</template>
                      <template v-else>localhost:{{ t.local_port }} → {{ t.remote_host }}:{{ t.remote_port }}</template>
                    </span>
                    <button type="button" class="text-otter-subtle hover:text-otter-coral text-sm leading-none px-1" @click="removeSavedTunnel(t.id)">✕</button>
                  </div>
                  <div class="flex items-center gap-1 text-xs">
                    <button
                      v-for="ty in (['local', 'dynamic', 'remote'] as const)"
                      :key="ty"
                      type="button"
                      class="px-2 py-1 rounded capitalize"
                      :class="newTun.tunnel_type === ty ? 'bg-otter-teal text-otter-dark' : 'text-otter-muted hover:bg-otter-surface'"
                      @click="newTun.tunnel_type = ty"
                    >{{ ty }}</button>
                  </div>
                  <div class="flex items-center gap-1.5 text-xs font-mono">
                    <template v-if="newTun.tunnel_type === 'remote'">
                      <span class="text-otter-subtle">R:</span>
                      <input v-model.number="newTun.remote_port" type="number" placeholder="3000" class="w-16 px-2 py-1.5 rounded-lg bg-otter-surface border border-otter-border text-otter-text focus:outline-none focus:border-otter-teal-dim" />
                      <span class="text-otter-subtle">→ L:</span>
                      <input v-model.number="newTun.local_port" type="number" placeholder="3000" class="w-16 px-2 py-1.5 rounded-lg bg-otter-surface border border-otter-border text-otter-text focus:outline-none focus:border-otter-teal-dim" />
                    </template>
                    <template v-else>
                      <span class="text-otter-subtle">L:</span>
                      <input v-model.number="newTun.local_port" type="number" placeholder="5432" class="w-16 px-2 py-1.5 rounded-lg bg-otter-surface border border-otter-border text-otter-text focus:outline-none focus:border-otter-teal-dim" />
                      <template v-if="newTun.tunnel_type === 'local'">
                        <span class="text-otter-subtle">→</span>
                        <input v-model="newTun.remote_host" placeholder="db.internal" class="flex-1 min-w-0 px-2 py-1.5 rounded-lg bg-otter-surface border border-otter-border text-otter-text focus:outline-none focus:border-otter-teal-dim" />
                        <span class="text-otter-subtle">:</span>
                        <input v-model.number="newTun.remote_port" type="number" placeholder="5432" class="w-16 px-2 py-1.5 rounded-lg bg-otter-surface border border-otter-border text-otter-text focus:outline-none focus:border-otter-teal-dim" />
                      </template>
                      <span v-else class="flex-1 text-otter-subtle">SOCKS5 proxy</span>
                    </template>
                    <button type="button" class="px-2 py-1.5 rounded-lg bg-otter-surface border border-otter-border text-otter-text hover:border-otter-teal-dim" @click="addSavedTunnel">Add</button>
                  </div>
                  <p v-if="tunnelError" class="text-[11px] text-otter-coral">{{ tunnelError }}</p>
                </div>
              </Transition>
            </template>

            <!-- ── Action buttons ── -->
            <div class="flex gap-2 pt-2">
              <button
                class="flex-1 py-2 rounded-lg bg-otter-teal text-otter-dark font-semibold
                       text-sm hover:opacity-90 transition-opacity disabled:opacity-50"
                :disabled="form.connecting.value || form.saving.value || !form.host.value || !form.username.value"
                @click="form.saveAndConnect()"
              >
                {{ form.connecting.value ? "Connecting..." : form.isEditing ? "Connect" : "Save & Connect" }}
              </button>
              <button
                class="flex-1 py-2 rounded-lg bg-otter-surface border border-otter-border
                       text-otter-text text-sm hover:border-otter-subtle
                       transition-colors disabled:opacity-50"
                :disabled="form.connecting.value || form.saving.value || !form.name.value || !form.host.value || !form.username.value"
                @click="form.save()"
              >
                {{ form.saving.value ? "Saving..." : form.isEditing ? "Update" : "Save" }}
              </button>
            </div>

            <!-- Delete server (edit-only) -->
            <template v-if="form.isEditing">
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

          <p v-if="form.error.value" class="mt-3 text-sm text-otter-coral break-all">
            {{ form.error.value }}
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

.section-enter-active {
  transition: all 0.2s ease-out;
}
.section-leave-active {
  transition: all 0.15s ease-in;
}
.section-enter-from,
.section-leave-to {
  opacity: 0;
  max-height: 0;
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
