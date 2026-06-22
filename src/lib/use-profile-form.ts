import { ref, onMounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { useProfilesStore, type ProfileSummary } from "../stores/profiles";
import { useTerminalStore } from "../stores/terminal";

export interface ProfileFormPrefill {
  username: string;
  host: string;
  port: number;
}

export interface ProfileFormEmits {
  close: () => void;
  saved: () => void;
  connected: (sessionId: string) => void;
}

export function useProfileForm(
  emit: ProfileFormEmits,
  profile?: ProfileSummary,
  prefill?: ProfileFormPrefill,
) {
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
  const error = ref("");

  const isEditing = !!profile;

  onMounted(() => {
    if (profile) {
      name.value = profile.name;
      host.value = profile.host;
      port.value = profile.port;
      username.value = profile.username;
      authType.value = profile.auth_type;
      groupId.value = profile.group_id;
      tags.value = profile.tags;
    } else if (prefill) {
      host.value = prefill.host;
      port.value = prefill.port;
      username.value = prefill.username;
    }
  });

  function buildProfileData() {
    return {
      name: name.value,
      host: host.value,
      port: port.value || 22,
      username: username.value,
      auth_type: authType.value,
      password:
        authType.value === "password" ? password.value || undefined : undefined,
      key_path:
        authType.value === "key" ? keyPath.value || undefined : undefined,
      passphrase:
        authType.value === "key" ? passphrase.value || undefined : undefined,
      group_id: groupId.value,
      tags: tags.value,
    };
  }

  async function save() {
    error.value = "";
    saving.value = true;

    try {
      if (isEditing) {
        await profilesStore.updateProfile(profile!.id, buildProfileData());
      } else {
        await profilesStore.createProfile(buildProfileData());
      }
      emit.saved();
      emit.close();
    } catch (e) {
      const err = String(e);
      if (err.includes("vault_locked") && profilesStore.onVaultLocked) {
        saving.value = false;
        profilesStore.onVaultLocked(() => save());
        return;
      }
      error.value = err;
    } finally {
      saving.value = false;
    }
  }

  async function saveAndConnect() {
    error.value = "";
    connecting.value = true;

    try {
      const auth =
        authType.value === "password"
          ? { type: "password" as const, password: password.value }
          : {
              type: "key" as const,
              private_key_path: keyPath.value,
              passphrase: passphrase.value || null,
            };

      const sessionId = await invoke<string>("ssh_connect", {
        config: {
          host: host.value,
          port: port.value || 22,
          username: username.value,
          auth,
        },
      });

      await invoke("ssh_open_pty", {
        sessionId,
        cols: 80,
        rows: 24,
      });

      if (isEditing) {
        await profilesStore.updateProfile(profile!.id, buildProfileData());
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

      emit.connected(sessionId);
      emit.close();
    } catch (e) {
      const err = String(e);
      if (err.includes("vault_locked") && profilesStore.onVaultLocked) {
        connecting.value = false;
        profilesStore.onVaultLocked(() => saveAndConnect());
        return;
      }
      error.value = err;
    } finally {
      connecting.value = false;
    }
  }

  return {
    name,
    host,
    port,
    username,
    authType,
    password,
    keyPath,
    passphrase,
    groupId,
    tags,
    saving,
    connecting,
    error,
    isEditing,
    save,
    saveAndConnect,
  };
}
