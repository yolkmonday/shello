<script setup lang="ts">
import { computed, ref } from "vue";
import { Icon } from "@iconify/vue";
import { confirm } from "@tauri-apps/plugin-dialog";
import {
  useSftpStore,
  remoteParent,
  localParent,
  type FileEntry,
} from "../../stores/sftp";

const props = defineProps<{ side: "local" | "remote"; sessionId: string }>();

const store = useSftpStore();
const isRemote = computed(() => props.side === "remote");

const path = computed(() =>
  isRemote.value
    ? store.remotePath[props.sessionId] ?? ""
    : store.localPath[props.sessionId] ?? "",
);
const entries = computed<FileEntry[]>(() =>
  isRemote.value
    ? store.remoteEntries[props.sessionId] ?? []
    : store.localEntries[props.sessionId] ?? [],
);

const selected = ref<string | null>(null);
const error = ref<string>("");

// Inline input state for create/rename
type InputMode = { kind: "mkdir" | "newfile" | "rename" | "chmod"; target?: FileEntry };
const input = ref<InputMode | null>(null);
const inputValue = ref("");

function startInput(mode: InputMode) {
  input.value = mode;
  inputValue.value =
    mode.kind === "rename"
      ? mode.target!.name
      : mode.kind === "chmod"
        ? (mode.target!.mode ? (mode.target!.mode & 0o777).toString(8) : "644")
        : "";
}

async function commitInput() {
  const mode = input.value;
  const value = inputValue.value.trim();
  input.value = null;
  if (!mode || !value) return;
  try {
    error.value = "";
    if (mode.kind === "mkdir") await store.mkdir(props.sessionId, value);
    else if (mode.kind === "newfile") await store.createFile(props.sessionId, value);
    else if (mode.kind === "rename") await store.rename(props.sessionId, mode.target!, value);
    else if (mode.kind === "chmod")
      await store.chmod(props.sessionId, mode.target!, parseInt(value, 8));
  } catch (e) {
    error.value = String(e);
  }
}

function sorted(list: FileEntry[]): FileEntry[] {
  return [...list].sort((a, b) => {
    if (a.is_dir !== b.is_dir) return a.is_dir ? -1 : 1;
    return a.name.localeCompare(b.name);
  });
}

async function refresh() {
  try {
    error.value = "";
    if (isRemote.value) await store.refreshRemote(props.sessionId);
    else await store.refreshLocal(props.sessionId);
  } catch (e) {
    error.value = String(e);
  }
}

async function goUp() {
  const parent = isRemote.value ? remoteParent(path.value) : localParent(path.value);
  await navTo(parent);
}

async function navTo(target: string) {
  try {
    error.value = "";
    if (isRemote.value) await store.navRemote(props.sessionId, target);
    else await store.navLocal(props.sessionId, target);
    selected.value = null;
  } catch (e) {
    error.value = String(e);
  }
}

async function onRowDblClick(entry: FileEntry) {
  if (entry.is_dir) await navTo(entry.path);
  else transfer(entry);
}

function transfer(entry: FileEntry) {
  if (entry.is_dir) return; // directory transfer not supported in MVP
  if (isRemote.value) store.enqueueDownload(props.sessionId, entry);
  else store.enqueueUpload(props.sessionId, entry);
}

async function remove(entry: FileEntry) {
  const ok = await confirm(`Delete "${entry.name}"?`, { title: "Confirm delete", kind: "warning" });
  if (!ok) return;
  try {
    error.value = "";
    await store.remove(props.sessionId, entry);
  } catch (e) {
    error.value = String(e);
  }
}

function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  const units = ["KB", "MB", "GB", "TB"];
  let v = bytes / 1024;
  let i = 0;
  while (v >= 1024 && i < units.length - 1) {
    v /= 1024;
    i++;
  }
  return `${v.toFixed(v < 10 ? 1 : 0)} ${units[i]}`;
}

function formatTime(unix: number | null): string {
  if (!unix) return "";
  return new Date(unix * 1000).toLocaleDateString();
}

function fileIcon(entry: FileEntry): string {
  if (entry.is_symlink) return "mdi:link-variant";
  if (entry.is_dir) return "mdi:folder";
  return "mdi:file-outline";
}
</script>

<template>
  <div class="flex flex-col h-full min-w-0 bg-otter-card border border-otter-border rounded-lg overflow-hidden">
    <!-- Header -->
    <div class="flex items-center gap-1.5 px-2 py-1.5 border-b border-otter-border bg-otter-surface/40">
      <Icon :icon="isRemote ? 'mdi:server' : 'mdi:laptop'" class="w-4 h-4 text-otter-muted flex-shrink-0" />
      <span class="text-xs text-otter-muted flex-shrink-0">{{ isRemote ? 'Remote' : 'Local' }}</span>
      <span class="text-xs text-otter-subtle truncate flex-1" :title="path">{{ path }}</span>
      <button class="p-1 rounded hover:bg-otter-surface text-otter-muted hover:text-otter-text" title="Up" @click="goUp">
        <Icon icon="mdi:arrow-up" class="w-3.5 h-3.5" />
      </button>
      <button class="p-1 rounded hover:bg-otter-surface text-otter-muted hover:text-otter-text" title="Refresh" @click="refresh">
        <Icon icon="mdi:refresh" class="w-3.5 h-3.5" />
      </button>
      <template v-if="isRemote">
        <button class="p-1 rounded hover:bg-otter-surface text-otter-muted hover:text-otter-text" title="New folder" @click="startInput({ kind: 'mkdir' })">
          <Icon icon="mdi:folder-plus-outline" class="w-3.5 h-3.5" />
        </button>
        <button class="p-1 rounded hover:bg-otter-surface text-otter-muted hover:text-otter-text" title="New file" @click="startInput({ kind: 'newfile' })">
          <Icon icon="mdi:file-plus-outline" class="w-3.5 h-3.5" />
        </button>
      </template>
    </div>

    <!-- Inline input -->
    <div v-if="input" class="flex items-center gap-2 px-2 py-1.5 border-b border-otter-border bg-otter-surface/20">
      <span class="text-xs text-otter-muted capitalize">{{ input.kind === 'chmod' ? 'Mode (octal)' : input.kind }}</span>
      <input
        v-model="inputValue"
        class="flex-1 bg-otter-dark border border-otter-border rounded px-2 py-1 text-xs text-otter-text focus:outline-none focus:border-otter-teal-dim"
        autofocus
        @keydown.enter="commitInput"
        @keydown.esc="input = null"
      />
      <button class="text-xs text-otter-teal hover:underline" @click="commitInput">OK</button>
      <button class="text-xs text-otter-subtle hover:underline" @click="input = null">Cancel</button>
    </div>

    <!-- Error -->
    <div v-if="error" class="px-2 py-1 text-[11px] text-red-400 bg-red-500/10 border-b border-otter-border truncate" :title="error">
      {{ error }}
    </div>

    <!-- List -->
    <div class="flex-1 overflow-y-auto text-sm">
      <button
        class="w-full flex items-center gap-2 px-2 py-1 text-left text-otter-muted hover:bg-otter-surface/50"
        @click="goUp"
      >
        <Icon icon="mdi:folder-upload-outline" class="w-4 h-4 flex-shrink-0" />
        <span class="text-xs">..</span>
      </button>

      <div
        v-for="entry in sorted(entries)"
        :key="entry.path"
        class="group flex items-center gap-2 px-2 py-1 cursor-pointer"
        :class="selected === entry.path ? 'bg-otter-teal/15' : 'hover:bg-otter-surface/50'"
        @click="selected = entry.path"
        @dblclick="onRowDblClick(entry)"
      >
        <Icon
          :icon="fileIcon(entry)"
          class="w-4 h-4 flex-shrink-0"
          :class="entry.is_dir ? 'text-otter-blue' : 'text-otter-subtle'"
        />
        <span class="text-xs text-otter-text truncate flex-1">{{ entry.name }}</span>
        <span v-if="!entry.is_dir" class="text-[10px] text-otter-subtle flex-shrink-0 w-14 text-right">{{ formatSize(entry.size) }}</span>
        <span class="text-[10px] text-otter-subtle flex-shrink-0 w-16 text-right hidden md:inline">{{ formatTime(entry.modified) }}</span>

        <!-- Row actions -->
        <div class="flex items-center gap-0.5 opacity-0 group-hover:opacity-100 flex-shrink-0">
          <button
            v-if="!entry.is_dir"
            class="p-0.5 rounded hover:bg-otter-surface text-otter-muted hover:text-otter-teal"
            :title="isRemote ? 'Download' : 'Upload'"
            @click.stop="transfer(entry)"
          >
            <Icon :icon="isRemote ? 'mdi:download' : 'mdi:upload'" class="w-3.5 h-3.5" />
          </button>
          <template v-if="isRemote">
            <button class="p-0.5 rounded hover:bg-otter-surface text-otter-muted hover:text-otter-text" title="Rename" @click.stop="startInput({ kind: 'rename', target: entry })">
              <Icon icon="mdi:rename-outline" class="w-3.5 h-3.5" />
            </button>
            <button class="p-0.5 rounded hover:bg-otter-surface text-otter-muted hover:text-otter-text" title="Permissions" @click.stop="startInput({ kind: 'chmod', target: entry })">
              <Icon icon="mdi:shield-key-outline" class="w-3.5 h-3.5" />
            </button>
            <button class="p-0.5 rounded hover:bg-otter-surface text-otter-muted hover:text-red-400" title="Delete" @click.stop="remove(entry)">
              <Icon icon="mdi:trash-can-outline" class="w-3.5 h-3.5" />
            </button>
          </template>
        </div>
      </div>

      <div v-if="entries.length === 0" class="px-2 py-4 text-center text-xs text-otter-subtle">
        Empty
      </div>
    </div>
  </div>
</template>
