import { defineStore } from "pinia";
import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

export interface FileEntry {
  name: string;
  path: string;
  is_dir: boolean;
  is_symlink: boolean;
  size: number;
  modified: number | null;
  mode: number | null;
}

export type TransferStatus = "queued" | "active" | "done" | "failed" | "cancelled";

export interface Transfer {
  id: string;
  direction: "download" | "upload";
  name: string;
  sessionId: string;
  localPath: string;
  remotePath: string;
  transferred: number;
  total: number;
  speed: number; // bytes/sec
  status: TransferStatus;
  error?: string;
  lastTime?: number;
  lastBytes?: number;
}

/** Parent directory for a POSIX-style remote path. */
export function remoteParent(path: string): string {
  if (path === "/" || path === "") return "/";
  const trimmed = path.replace(/\/+$/, "");
  const idx = trimmed.lastIndexOf("/");
  return idx <= 0 ? "/" : trimmed.slice(0, idx);
}

/** Parent directory for a local path (handles both / and \ separators). */
export function localParent(path: string): string {
  const trimmed = path.replace(/[\\/]+$/, "");
  const idx = Math.max(trimmed.lastIndexOf("/"), trimmed.lastIndexOf("\\"));
  if (idx < 0) return path;
  if (idx === 0) return "/";
  return trimmed.slice(0, idx);
}

function joinLocal(dir: string, name: string): string {
  const sep = dir.includes("\\") ? "\\" : "/";
  return dir.endsWith(sep) ? `${dir}${name}` : `${dir}${sep}${name}`;
}

function joinRemote(dir: string, name: string): string {
  return dir.endsWith("/") ? `${dir}${name}` : `${dir}/${name}`;
}

export const useSftpStore = defineStore("sftp", () => {
  const remotePath = ref<Record<string, string>>({});
  const localPath = ref<Record<string, string>>({});
  const remoteEntries = ref<Record<string, FileEntry[]>>({});
  const localEntries = ref<Record<string, FileEntry[]>>({});
  const transfers = ref<Transfer[]>([]);

  let running = false;
  let listenerReady = false;

  async function ensureListener() {
    if (listenerReady) return;
    listenerReady = true;
    await listen<{ transfer_id: string; transferred: number; total: number }>(
      "sftp-progress",
      (e) => {
        const t = transfers.value.find((x) => x.id === e.payload.transfer_id);
        if (!t) return;
        const now = performance.now();
        if (t.lastTime != null) {
          const dt = (now - t.lastTime) / 1000;
          if (dt > 0) {
            t.speed = (e.payload.transferred - (t.lastBytes ?? 0)) / dt;
          }
        }
        t.lastTime = now;
        t.lastBytes = e.payload.transferred;
        t.transferred = e.payload.transferred;
        t.total = e.payload.total;
        if (t.status === "queued") t.status = "active";
      },
    );
  }

  async function open(sessionId: string) {
    await ensureListener();
    const home = await invoke<string>("sftp_open", { sessionId });
    remotePath.value[sessionId] = home;
    if (!localPath.value[sessionId]) {
      localPath.value[sessionId] = await invoke<string>("sftp_local_home");
    }
    await Promise.all([refreshRemote(sessionId), refreshLocal(sessionId)]);
  }

  async function refreshRemote(sessionId: string) {
    remoteEntries.value[sessionId] = await invoke<FileEntry[]>("sftp_list", {
      sessionId,
      path: remotePath.value[sessionId],
    });
  }

  async function refreshLocal(sessionId: string) {
    localEntries.value[sessionId] = await invoke<FileEntry[]>("sftp_local_list", {
      path: localPath.value[sessionId],
    });
  }

  async function navRemote(sessionId: string, path: string) {
    remotePath.value[sessionId] = path;
    await refreshRemote(sessionId);
  }

  async function navLocal(sessionId: string, path: string) {
    localPath.value[sessionId] = path;
    await refreshLocal(sessionId);
  }

  // ── Remote-only mutations ──────────────────────────────────────────

  async function mkdir(sessionId: string, name: string) {
    await invoke("sftp_mkdir", {
      sessionId,
      path: joinRemote(remotePath.value[sessionId], name),
    });
    await refreshRemote(sessionId);
  }

  async function createFile(sessionId: string, name: string) {
    await invoke("sftp_create_file", {
      sessionId,
      path: joinRemote(remotePath.value[sessionId], name),
    });
    await refreshRemote(sessionId);
  }

  async function remove(sessionId: string, entry: FileEntry) {
    await invoke("sftp_delete", { sessionId, path: entry.path, isDir: entry.is_dir });
    await refreshRemote(sessionId);
  }

  async function rename(sessionId: string, entry: FileEntry, newName: string) {
    const to = joinRemote(remoteParent(entry.path), newName);
    await invoke("sftp_rename", { sessionId, from: entry.path, to });
    await refreshRemote(sessionId);
  }

  async function chmod(sessionId: string, entry: FileEntry, mode: number) {
    await invoke("sftp_chmod", { sessionId, path: entry.path, mode });
    await refreshRemote(sessionId);
  }

  // ── Transfers ──────────────────────────────────────────────────────

  function enqueueDownload(sessionId: string, entry: FileEntry) {
    transfers.value.push({
      id: crypto.randomUUID(),
      direction: "download",
      name: entry.name,
      sessionId,
      remotePath: entry.path,
      localPath: joinLocal(localPath.value[sessionId], entry.name),
      transferred: 0,
      total: entry.size,
      speed: 0,
      status: "queued",
    });
    void runQueue();
  }

  function enqueueUpload(sessionId: string, entry: FileEntry) {
    transfers.value.push({
      id: crypto.randomUUID(),
      direction: "upload",
      name: entry.name,
      sessionId,
      localPath: entry.path,
      remotePath: joinRemote(remotePath.value[sessionId], entry.name),
      transferred: 0,
      total: entry.size,
      speed: 0,
      status: "queued",
    });
    void runQueue();
  }

  async function runQueue() {
    if (running) return;
    running = true;
    try {
      for (;;) {
        const next = transfers.value.find((t) => t.status === "queued");
        if (!next) break;
        next.status = "active";
        try {
          if (next.direction === "download") {
            await invoke("sftp_download", {
              sessionId: next.sessionId,
              remotePath: next.remotePath,
              localPath: next.localPath,
              transferId: next.id,
            });
            next.status = "done";
            await refreshLocal(next.sessionId);
          } else {
            await invoke("sftp_upload", {
              sessionId: next.sessionId,
              localPath: next.localPath,
              remotePath: next.remotePath,
              transferId: next.id,
            });
            next.status = "done";
            await refreshRemote(next.sessionId);
          }
        } catch (e) {
          const msg = String(e);
          next.status = msg.includes("cancel") ? "cancelled" : "failed";
          next.error = msg;
        }
      }
    } finally {
      running = false;
    }
  }

  async function cancel(id: string) {
    const t = transfers.value.find((x) => x.id === id);
    if (t && (t.status === "active" || t.status === "queued")) {
      await invoke("sftp_cancel", { transferId: id });
    }
  }

  function clearFinished() {
    transfers.value = transfers.value.filter(
      (t) => t.status === "active" || t.status === "queued",
    );
  }

  async function close(sessionId: string) {
    await invoke("sftp_close", { sessionId });
    delete remoteEntries.value[sessionId];
    delete localEntries.value[sessionId];
    delete remotePath.value[sessionId];
    delete localPath.value[sessionId];
  }

  return {
    remotePath,
    localPath,
    remoteEntries,
    localEntries,
    transfers,
    open,
    refreshRemote,
    refreshLocal,
    navRemote,
    navLocal,
    mkdir,
    createFile,
    remove,
    rename,
    chmod,
    enqueueDownload,
    enqueueUpload,
    cancel,
    clearFinished,
    close,
  };
});
