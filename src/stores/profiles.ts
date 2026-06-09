import { defineStore } from "pinia";
import { ref, computed } from "vue";
import { invoke } from "@tauri-apps/api/core";

export interface ProfileSummary {
  id: string;
  name: string;
  host: string;
  port: number;
  username: string;
  auth_type: "password" | "key";
  group_id: string | null;
  tags: string;
  sort_order: number;
  detected_os: string | null;
  created_at: string;
  updated_at: string;
}

export interface Group {
  id: string;
  name: string;
  color: string;
  sort_order: number;
  created_at: string;
  updated_at: string;
}

export interface CreateProfileInput {
  name: string;
  host: string;
  port?: number;
  username: string;
  auth_type: "password" | "key";
  password?: string;
  key_path?: string;
  passphrase?: string;
  group_id?: string | null;
  tags?: string;
}

export interface UpdateProfileInput {
  name?: string;
  host?: string;
  port?: number;
  username?: string;
  auth_type?: string;
  password?: string;
  key_path?: string;
  passphrase?: string;
  group_id?: string | null;
  tags?: string;
}

export const useProfilesStore = defineStore("profiles", () => {
  const profiles = ref<Record<string, ProfileSummary>>({});
  const groups = ref<Record<string, Group>>({});
  const searchQuery = ref("");
  const searchResults = ref<ProfileSummary[]>([]);
  const isSearching = ref(false);

  // Computed: profiles grouped by group_id
  const groupedProfiles = computed(() => {
    const map = new Map<string | null, ProfileSummary[]>();
    const list = Object.values(profiles.value);

    for (const p of list) {
      const key = p.group_id;
      if (!map.has(key)) map.set(key, []);
      map.get(key)!.push(p);
    }

    // Sort each group's profiles by sort_order, then name
    for (const [, arr] of map) {
      arr.sort((a, b) => a.sort_order - b.sort_order || a.name.localeCompare(b.name));
    }

    return map;
  });

  // Sorted groups list
  const sortedGroups = computed(() =>
    Object.values(groups.value).sort(
      (a, b) => a.sort_order - b.sort_order || a.name.localeCompare(b.name)
    )
  );

  // Vault status tracking
  const vaultStatus = ref<{ initialized: boolean; unlocked: boolean }>({
    initialized: false,
    unlocked: false,
  });

  async function checkVaultStatus() {
    vaultStatus.value = await invoke("vault_status");
  }

  // Callback ref for vault locked handling
  const onVaultLocked = ref<((retryFn: () => void) => void) | null>(null);

  async function loadAll() {
    const [profileList, groupList] = await Promise.all([
      invoke<ProfileSummary[]>("profile_list"),
      invoke<Group[]>("group_list"),
    ]);

    profiles.value = {};
    for (const p of profileList) {
      profiles.value[p.id] = p;
    }

    groups.value = {};
    for (const g of groupList) {
      groups.value[g.id] = g;
    }

    await checkVaultStatus();
  }

  async function createProfile(input: CreateProfileInput): Promise<ProfileSummary> {
    const profile = await invoke<ProfileSummary>("profile_create", { input });
    profiles.value[profile.id] = profile;
    return profile;
  }

  async function updateProfile(id: string, input: UpdateProfileInput): Promise<ProfileSummary> {
    const profile = await invoke<ProfileSummary>("profile_update", { id, input });
    profiles.value[profile.id] = profile;
    return profile;
  }

  async function deleteProfile(id: string) {
    await invoke("profile_delete", { id });
    delete profiles.value[id];
  }

  async function createGroup(name: string, color: string): Promise<Group> {
    const group = await invoke<Group>("group_create", { name, color });
    groups.value[group.id] = group;
    return group;
  }

  async function updateGroup(id: string, name: string, color: string): Promise<Group> {
    const group = await invoke<Group>("group_update", { id, name, color });
    groups.value[group.id] = group;
    return group;
  }

  async function deleteGroup(id: string) {
    await invoke("group_delete", { id });
    delete groups.value[id];
    // Profiles in this group become ungrouped (DB handles ON DELETE SET NULL)
    for (const p of Object.values(profiles.value)) {
      if (p.group_id === id) p.group_id = null;
    }
  }

  let searchTimeout: ReturnType<typeof setTimeout> | null = null;

  async function search(query: string) {
    searchQuery.value = query;

    if (!query.trim()) {
      searchResults.value = [];
      isSearching.value = false;
      return;
    }

    isSearching.value = true;

    if (searchTimeout) clearTimeout(searchTimeout);
    searchTimeout = setTimeout(async () => {
      try {
        searchResults.value = await invoke<ProfileSummary[]>("profile_search", { query });
      } catch {
        searchResults.value = [];
      }
    }, 150);
  }

  function clearSearch() {
    searchQuery.value = "";
    searchResults.value = [];
    isSearching.value = false;
  }

  return {
    profiles,
    groups,
    searchQuery,
    searchResults,
    isSearching,
    groupedProfiles,
    sortedGroups,
    vaultStatus,
    checkVaultStatus,
    onVaultLocked,
    loadAll,
    createProfile,
    updateProfile,
    deleteProfile,
    createGroup,
    updateGroup,
    deleteGroup,
    search,
    clearSearch,
  };
});
