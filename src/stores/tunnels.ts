import { defineStore } from "pinia";
import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

export interface Tunnel {
  id: string;
  profile_id: string;
  tunnel_type: string;
  local_host: string;
  local_port: number;
  remote_host: string;
  remote_port: number;
  enabled: boolean;
  created_at: string;
  updated_at: string;
}

export type ActiveStatus = "starting" | "active" | "error" | "stopped";

export interface ActiveTunnel {
  tunnel_id: string;
  session_id: string;
  local_host: string;
  local_port: number;
  remote_host: string;
  remote_port: number;
  status: ActiveStatus;
  error?: string;
}

export interface TunnelConfig {
  tunnel_id: string;
  local_host: string;
  local_port: number;
  remote_host: string;
  remote_port: number;
}

export const useTunnelsStore = defineStore("tunnels", () => {
  const saved = ref<Record<string, Tunnel[]>>({}); // by profileId
  const active = ref<Record<string, ActiveTunnel[]>>({}); // by sessionId
  let listenerReady = false;

  async function ensureListener() {
    if (listenerReady) return;
    listenerReady = true;
    await listen<{ key: string; info: ActiveTunnel & { status: { state: string } } }>(
      "tunnel-status",
      (e) => {
        const info = e.payload.info;
        const sid = info.session_id;
        const list = active.value[sid] ? [...active.value[sid]] : [];
        const idx = list.findIndex((t) => t.tunnel_id === info.tunnel_id);
        if (info.status.state === "stopped") {
          active.value[sid] = list.filter((t) => t.tunnel_id !== info.tunnel_id);
          return;
        }
        const entry: ActiveTunnel = {
          tunnel_id: info.tunnel_id,
          session_id: sid,
          local_host: info.local_host,
          local_port: info.local_port,
          remote_host: info.remote_host,
          remote_port: info.remote_port,
          status: "active",
        };
        if (idx >= 0) list[idx] = entry;
        else list.push(entry);
        active.value[sid] = list;
      },
    );
  }

  // ── Saved (persistent) tunnels ─────────────────────────────────────

  async function loadSaved(profileId: string) {
    saved.value[profileId] = await invoke<Tunnel[]>("tunnel_list", { profileId });
  }

  async function createSaved(input: {
    profile_id: string;
    local_host?: string;
    local_port: number;
    remote_host: string;
    remote_port: number;
    enabled?: boolean;
  }) {
    await invoke<Tunnel>("tunnel_create", { input });
    await loadSaved(input.profile_id);
  }

  async function updateSaved(
    profileId: string,
    id: string,
    input: Partial<Pick<Tunnel, "local_host" | "local_port" | "remote_host" | "remote_port" | "enabled">>,
  ) {
    await invoke<Tunnel>("tunnel_update", { id, input });
    await loadSaved(profileId);
  }

  async function removeSaved(profileId: string, id: string) {
    await invoke("tunnel_delete", { id });
    await loadSaved(profileId);
  }

  // ── Runtime tunnels ────────────────────────────────────────────────

  function setActive(sessionId: string, entry: ActiveTunnel) {
    const list = active.value[sessionId] ? [...active.value[sessionId]] : [];
    const idx = list.findIndex((t) => t.tunnel_id === entry.tunnel_id);
    if (idx >= 0) list[idx] = entry;
    else list.push(entry);
    active.value[sessionId] = list;
  }

  async function start(sessionId: string, cfg: TunnelConfig) {
    await ensureListener();
    setActive(sessionId, { ...cfg, session_id: sessionId, status: "starting" });
    try {
      await invoke("tunnel_start", { sessionId, config: cfg });
      setActive(sessionId, { ...cfg, session_id: sessionId, status: "active" });
    } catch (e) {
      setActive(sessionId, { ...cfg, session_id: sessionId, status: "error", error: String(e) });
    }
  }

  async function stop(sessionId: string, tunnelId: string) {
    await invoke("tunnel_stop", { sessionId, tunnelId });
    active.value[sessionId] = (active.value[sessionId] ?? []).filter((t) => t.tunnel_id !== tunnelId);
  }

  async function retry(sessionId: string, tunnelId: string) {
    const t = (active.value[sessionId] ?? []).find((x) => x.tunnel_id === tunnelId);
    if (t) await start(sessionId, t);
  }

  async function refreshActive(sessionId: string) {
    active.value[sessionId] = await invoke<ActiveTunnel[]>("tunnel_active", { sessionId });
  }

  async function autoStart(sessionId: string, profileId: string) {
    await loadSaved(profileId);
    for (const t of (saved.value[profileId] ?? []).filter((x) => x.enabled)) {
      await start(sessionId, {
        tunnel_id: t.id,
        local_host: t.local_host,
        local_port: t.local_port,
        remote_host: t.remote_host,
        remote_port: t.remote_port,
      });
    }
  }

  function clearSession(sessionId: string) {
    delete active.value[sessionId];
  }

  return {
    saved,
    active,
    loadSaved,
    createSaved,
    updateSaved,
    removeSaved,
    start,
    stop,
    retry,
    refreshActive,
    autoStart,
    clearSession,
  };
});
