<script setup lang="ts">
import { ref, onMounted, onUnmounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { useTerminalStore } from "../stores/terminal";
import { useProfilesStore, type ProfileSummary, type Group } from "../stores/profiles";

const emit = defineEmits<{
  "new-connection": [];
  "split-connection": [sessionId: string, direction: "vertical" | "horizontal"];
  "open-profile-editor": [profile?: ProfileSummary];
  "open-group-editor": [group?: Group];
  "open-settings": [];
  connecting: [profile: ProfileSummary];
  "connect-done": [error: string | null];
}>();

const terminalStore = useTerminalStore();
const profilesStore = useProfilesStore();
const searchInput = ref<HTMLInputElement>();
const collapsedGroups = ref<Set<string | null>>(new Set());
const connectingId = ref<string | null>(null);

function toggleGroup(groupId: string | null) {
  if (collapsedGroups.value.has(groupId)) {
    collapsedGroups.value.delete(groupId);
  } else {
    collapsedGroups.value.add(groupId);
  }
}

async function connectProfile(profile: ProfileSummary) {
  connectingId.value = profile.id;
  emit("connecting", profile);
  try {
    const sessionId = await invoke<string>("profile_connect", {
      profileId: profile.id,
      cols: 80,
      rows: 24,
    });

    terminalStore.addSession(sessionId, {
      id: sessionId,
      host: profile.host,
      port: profile.port,
      username: profile.username,
      connected_at: new Date().toISOString(),
      status: "connected",
    });
    emit("connect-done", null);
  } catch (e) {
    const err = String(e);
    if (err.includes("vault_locked") && profilesStore.onVaultLocked) {
      connectingId.value = null;
      emit("connect-done", null);
      profilesStore.onVaultLocked(() => connectProfile(profile));
    } else {
      emit("connect-done", String(e));
    }
  } finally {
    connectingId.value = null;
  }
}

async function disconnectSession(sessionId: string) {
  try {
    await invoke("ssh_disconnect", { sessionId });
  } catch {
    // May already be disconnected
  }
  terminalStore.removeSession(sessionId);
}

async function deleteProfile(id: string) {
  await profilesStore.deleteProfile(id);
}

// Ctrl+K shortcut
function onKeydown(e: KeyboardEvent) {
  if ((e.metaKey || e.ctrlKey) && e.key === "k") {
    e.preventDefault();
    searchInput.value?.focus();
  }
}

onMounted(() => {
  window.addEventListener("keydown", onKeydown);
});

onUnmounted(() => {
  window.removeEventListener("keydown", onKeydown);
});
</script>

<template>
  <div
    class="w-[200px] flex-shrink-0 bg-otter-card border-r border-otter-border
           flex flex-col h-full"
  >
    <!-- Logo -->
    <div class="p-4 flex items-center gap-2">
      <div
        class="w-8 h-8 rounded-lg flex items-center justify-center
               bg-gradient-to-br from-otter-teal to-otter-teal-dim
               text-otter-dark font-bold text-sm"
      >
        S
      </div>
      <span class="text-sm font-semibold text-otter-text tracking-tight">Shello</span>
    </div>

    <!-- Search bar -->
    <div class="px-2 pb-2">
      <input
        ref="searchInput"
        :value="profilesStore.searchQuery"
        class="w-full px-3 py-1.5 rounded-lg bg-otter-surface border border-otter-border
               text-otter-text placeholder-otter-subtle text-xs
               focus:outline-none focus:border-otter-teal-dim transition-colors"
        placeholder="Search... (Ctrl+K)"
        @input="profilesStore.search(($event.target as HTMLInputElement).value)"
        @keyup.escape="profilesStore.clearSearch(); searchInput?.blur()"
      />
    </div>

    <!-- Scrollable content -->
    <div class="flex-1 overflow-y-auto px-2">
      <!-- Search results mode -->
      <template v-if="profilesStore.isSearching">
        <div class="px-1 mb-1">
          <span class="text-xs text-otter-subtle uppercase tracking-wider">
            Search Results
          </span>
        </div>
        <div
          v-for="profile in profilesStore.searchResults"
          :key="profile.id"
          class="flex items-center gap-2 px-2 py-1.5 rounded-lg mb-0.5 cursor-pointer
                 text-xs text-otter-muted hover:bg-otter-surface/50 transition-colors"
          :class="{ 'opacity-50 pointer-events-none': connectingId === profile.id }"
          @click="connectProfile(profile)"
        >
          <span v-if="connectingId === profile.id" class="w-2 h-2 rounded-full bg-otter-teal animate-pulse flex-shrink-0"></span>
          <span class="truncate">{{ profile.name }}</span>
          <span class="ml-auto text-xs text-otter-subtle truncate">{{ connectingId === profile.id ? 'Connecting...' : profile.host }}</span>
        </div>
        <div
          v-if="profilesStore.searchResults.length === 0"
          class="px-2 py-4 text-xs text-otter-subtle text-center"
        >
          No profiles found
        </div>
      </template>

      <!-- Normal mode: Saved Profiles + Active Sessions -->
      <template v-else>
        <!-- Saved Profiles section -->
        <div class="mb-2">
          <div class="flex items-center px-1 mb-1">
            <span class="text-xs text-otter-subtle uppercase tracking-wider flex-1">
              Saved Profiles
            </span>
            <button
              class="text-otter-subtle hover:text-otter-teal text-xs px-1 transition-colors"
              title="New Group"
              @click="emit('open-group-editor')"
            >
              G+
            </button>
            <button
              class="text-otter-subtle hover:text-otter-teal text-sm px-1 transition-colors"
              title="New Profile"
              @click="emit('open-profile-editor')"
            >
              +
            </button>
          </div>

          <!-- Groups -->
          <template v-for="group in profilesStore.sortedGroups" :key="group.id">
            <div
              v-if="profilesStore.groupedProfiles.get(group.id)?.length"
              class="mb-1"
            >
              <!-- Group header -->
              <div
                class="flex items-center gap-1 px-2 py-0.5 text-xs text-otter-subtle
                       cursor-pointer hover:text-otter-muted transition-colors"
                @click="toggleGroup(group.id)"
                @contextmenu.prevent="emit('open-group-editor', group)"
              >
                <span class="text-[8px]">{{ collapsedGroups.has(group.id) ? '▶' : '▼' }}</span>
                <div
                  class="w-1.5 h-1.5 rounded-full flex-shrink-0"
                  :style="{ backgroundColor: group.color }"
                ></div>
                <span>{{ group.name }}</span>
              </div>

              <!-- Profiles in group -->
              <template v-if="!collapsedGroups.has(group.id)">
                <div
                  v-for="profile in profilesStore.groupedProfiles.get(group.id)"
                  :key="profile.id"
                  class="flex items-center gap-1.5 pl-5 pr-2 py-1 rounded-lg mb-0.5
                         cursor-pointer text-xs text-otter-muted
                         hover:bg-otter-surface/50 transition-colors"
                  :class="{ 'opacity-50 pointer-events-none': connectingId === profile.id }"
                  @click="connectProfile(profile)"
                  @contextmenu.prevent="emit('open-profile-editor', profile)"
                >
                  <span v-if="connectingId === profile.id" class="w-1.5 h-1.5 rounded-full bg-otter-teal animate-pulse flex-shrink-0"></span>
                  <span class="truncate">{{ profile.name }}</span>
                  <span v-if="connectingId === profile.id" class="ml-auto text-xs text-otter-subtle flex-shrink-0">Connecting...</span>
                  <button
                    v-else
                    class="ml-auto text-otter-subtle hover:text-otter-coral text-xs px-0.5 flex-shrink-0"
                    title="Delete"
                    @click.stop="deleteProfile(profile.id)"
                  >
                    &times;
                  </button>
                </div>
              </template>
            </div>
          </template>

          <!-- Ungrouped profiles -->
          <div v-if="profilesStore.groupedProfiles.get(null)?.length" class="mb-1">
            <div
              class="flex items-center gap-1 px-2 py-0.5 text-xs text-otter-subtle
                     cursor-pointer hover:text-otter-muted transition-colors"
              @click="toggleGroup(null)"
            >
              <span class="text-[8px]">{{ collapsedGroups.has(null) ? '▶' : '▼' }}</span>
              <span>Ungrouped</span>
            </div>

            <template v-if="!collapsedGroups.has(null)">
              <div
                v-for="profile in profilesStore.groupedProfiles.get(null)"
                :key="profile.id"
                class="flex items-center gap-1.5 pl-5 pr-2 py-1 rounded-lg mb-0.5
                       cursor-pointer text-xs text-otter-muted
                       hover:bg-otter-surface/50 transition-colors"
                :class="{ 'opacity-50 pointer-events-none': connectingId === profile.id }"
                @click="connectProfile(profile)"
                @contextmenu.prevent="emit('open-profile-editor', profile)"
              >
                <span v-if="connectingId === profile.id" class="w-1.5 h-1.5 rounded-full bg-otter-teal animate-pulse flex-shrink-0"></span>
                <span class="truncate">{{ profile.name }}</span>
                <span v-if="connectingId === profile.id" class="ml-auto text-xs text-otter-subtle flex-shrink-0">Connecting...</span>
                <button
                  v-else
                  class="ml-auto text-otter-subtle hover:text-otter-coral text-xs px-0.5 flex-shrink-0"
                  title="Delete"
                  @click.stop="deleteProfile(profile.id)"
                >
                  &times;
                </button>
              </div>
            </template>
          </div>

          <!-- Empty state for profiles -->
          <div
            v-if="Object.keys(profilesStore.profiles).length === 0"
            class="px-2 py-2 text-xs text-otter-subtle text-center"
          >
            No saved profiles
          </div>
        </div>

        <!-- Divider -->
        <div class="border-t border-otter-border mx-1 my-1"></div>

        <!-- Active Sessions section -->
        <div class="mb-2">
          <div class="px-1 mb-1">
            <span class="text-xs text-otter-subtle uppercase tracking-wider">
              Active Sessions
            </span>
          </div>

          <div
            v-for="session in terminalStore.sessions"
            :key="session.id"
            class="flex items-center gap-2 px-2 py-1.5 rounded-lg mb-0.5 cursor-pointer
                   transition-colors text-xs"
            :class="
              terminalStore.activeSessionId === session.id
                ? 'bg-otter-surface text-otter-teal'
                : 'text-otter-muted hover:bg-otter-surface/50'
            "
            @click="terminalStore.setActiveSession(session.id)"
          >
            <div
              class="w-1.5 h-1.5 rounded-full flex-shrink-0"
              :class="
                session.status === 'connected' ? 'bg-otter-teal' : 'bg-otter-subtle'
              "
            ></div>
            <span class="truncate font-mono">{{ session.username }}@{{ session.host }}</span>

            <div class="ml-auto flex gap-0.5 flex-shrink-0">
              <button
                class="text-otter-subtle hover:text-otter-text text-xs px-1"
                title="Split Right"
                @click.stop="emit('split-connection', session.id, 'vertical')"
              >
                ⎸
              </button>
              <button
                class="text-otter-subtle hover:text-otter-text text-xs px-1"
                title="Split Down"
                @click.stop="emit('split-connection', session.id, 'horizontal')"
              >
                ⎯
              </button>
              <button
                class="text-otter-subtle hover:text-otter-coral text-xs px-1"
                title="Disconnect"
                @click.stop="disconnectSession(session.id)"
              >
                ×
              </button>
            </div>
          </div>

          <div
            v-if="Object.keys(terminalStore.sessions).length === 0"
            class="px-2 py-2 text-xs text-otter-subtle text-center"
          >
            No active sessions
          </div>
        </div>
      </template>
    </div>

    <!-- Bottom actions -->
    <div class="px-2 py-2 border-t border-otter-border flex flex-col gap-1.5">
      <button
        class="w-full py-1.5 rounded-lg bg-otter-surface border border-otter-border
               text-otter-muted hover:text-otter-teal hover:border-otter-teal-dim
               text-xs transition-colors"
        @click="emit('new-connection')"
      >
        + New Connection
      </button>
      <button
        class="w-full py-1.5 rounded-lg text-otter-subtle hover:text-otter-text
               hover:bg-otter-surface/50 text-xs transition-colors flex items-center justify-center gap-1.5"
        @click="emit('open-settings')"
      >
        <svg class="w-3.5 h-3.5" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5">
          <circle cx="8" cy="8" r="2.5" />
          <path d="M8 1v2M8 13v2M1 8h2M13 8h2M2.9 2.9l1.4 1.4M11.7 11.7l1.4 1.4M13.1 2.9l-1.4 1.4M4.3 11.7l-1.4 1.4" />
        </svg>
        Settings
      </button>
    </div>
  </div>
</template>
