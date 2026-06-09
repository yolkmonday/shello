<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, nextTick, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { useTerminalStore } from "../stores/terminal";
import { type ProfileSummary, type Group } from "../stores/profiles";
import { useProfilesStore } from "../stores/profiles";
import TerminalView from "./TerminalView.vue";
import ProfileEditor from "./ProfileEditor.vue";
import GroupEditor from "./GroupEditor.vue";
import VaultDialog from "./VaultDialog.vue";
import SettingsPage from "./SettingsPage.vue";
import SnippetsPanel from "./SnippetsPanel.vue";
import RecipeRunner from "./RecipeRunner.vue";
import KeyManager from "./KeyManager.vue";
import ScreenshotPreview from "./ScreenshotPreview.vue";
import RecordingPreview from "./RecordingPreview.vue";
import LogoIcon from "./LogoIcon.vue";
import LogoWordmark from "./LogoWordmark.vue";
import { Icon } from "@iconify/vue";
import { getOsIcon, getOsColor, getOsLabel } from "../lib/os-icons";
import type { SerializedTerminal } from "../lib/terminal-to-html";
import type { TerminalRecording } from "../lib/terminal-recording";

const store = useTerminalStore();
const profilesStore = useProfilesStore();

// Current view: "home" | "settings" | session id
const currentView = ref<string>("home");


function parseQuickConnect(input: string): { username: string; host: string; port: number } | null {
  const trimmed = input.trim();
  if (!trimmed) return null;

  let username = "root";
  let host = trimmed;
  let port = 22;

  // user@host:port or user@host
  if (host.includes("@")) {
    const [u, rest] = host.split("@", 2);
    username = u;
    host = rest;
  }

  // host:port
  if (host.includes(":")) {
    const [h, p] = host.split(":", 2);
    host = h;
    const parsed = parseInt(p, 10);
    if (!isNaN(parsed)) port = parsed;
  }

  if (!host) return null;
  return { username, host, port };
}


const quickConnectData = ref<{ username: string; host: string; port: number } | undefined>();

// Connect combobox
const showConnectDropdown = ref(false);
const connectSearch = ref("");
const connectInputRef = ref<HTMLInputElement | null>(null);

const filteredProfiles = computed(() => {
  const q = connectSearch.value.toLowerCase().trim();
  if (!q) return allProfiles.value;
  return allProfiles.value.filter(p =>
    p.name.toLowerCase().includes(q) ||
    p.host.toLowerCase().includes(q) ||
    p.username.toLowerCase().includes(q)
  );
});

function toggleConnectDropdown() {
  showConnectDropdown.value = !showConnectDropdown.value;
  connectSearch.value = "";
  if (showConnectDropdown.value) {
    nextTick(() => connectInputRef.value?.focus());
  }
}

function handleConnectSelect(profile: ProfileSummary) {
  showConnectDropdown.value = false;
  connectSearch.value = "";
  connectFromGrid(profile);
}

function handleConnectEnter() {
  if (filteredProfiles.value.length === 1) {
    handleConnectSelect(filteredProfiles.value[0]);
    return;
  }
  const parsed = parseQuickConnect(connectSearch.value);
  if (parsed) {
    showConnectDropdown.value = false;
    editingProfile.value = undefined;
    quickConnectData.value = parsed;
    showProfileEditor.value = true;
    connectSearch.value = "";
  }
}

// Tab drag & drop reorder
const dragTabId = ref<string | null>(null);
const dragOverTabId = ref<string | null>(null);

function onTabDragStart(e: DragEvent, sessionId: string) {
  dragTabId.value = sessionId;
  if (e.dataTransfer) {
    e.dataTransfer.effectAllowed = "move";
    e.dataTransfer.setData("text/plain", sessionId);
  }
}

function onTabDragOver(e: DragEvent, sessionId: string) {
  e.preventDefault();
  if (e.dataTransfer) e.dataTransfer.dropEffect = "move";
  dragOverTabId.value = sessionId;
}

function onTabDrop(e: DragEvent, targetId: string) {
  e.preventDefault();
  if (dragTabId.value && dragTabId.value !== targetId) {
    store.reorderSession(dragTabId.value, targetId);
  }
  dragTabId.value = null;
  dragOverTabId.value = null;
}

function onTabDragEnd() {
  dragTabId.value = null;
  dragOverTabId.value = null;
}

// Tab overflow dropdown
const showTabOverflow = ref(false);
const tabContainerRef = ref<HTMLElement | null>(null);


// Confirm close dialog
const confirmCloseSessionId = ref<string | null>(null);

function requestCloseSession(sessionId: string) {
  const session = store.sessions[sessionId];
  if (session?.status !== "connected") {
    disconnectSession(sessionId);
    return;
  }

  // If not at prompt, a process is likely running
  if (!store.atPrompt[sessionId]) {
    confirmCloseSessionId.value = sessionId;
    return;
  }

  disconnectSession(sessionId);
}

function confirmClose() {
  if (confirmCloseSessionId.value) {
    disconnectSession(confirmCloseSessionId.value);
    confirmCloseSessionId.value = null;
  }
}

// Snippets & Key manager panels
const showSnippets = ref(false);
const showRecipes = ref(false);
const showKeyManager = ref(false);

// Screenshot
const screenshotData = ref<SerializedTerminal | null>(null);
const screenshotTitle = ref("");
const mainTerminalRef = ref<InstanceType<typeof TerminalView> | null>(null);

function openScreenshotPreview() {
  if (!mainTerminalRef.value) return;
  const data = mainTerminalRef.value.getScreenshotData();
  if (!data) return;
  const session = store.sessions[currentView.value];
  screenshotTitle.value = session?.profile_name || `${session?.username}@${session?.host}` || "terminal";
  screenshotData.value = data;
}

// Recording
const recordingData = ref<TerminalRecording | null>(null);
const recordingTitle = ref("");
const isRecordingSession = ref(false);

function pauseRecording() {
  mainTerminalRef.value?.pauseRecording();
}

function resumeRecording() {
  mainTerminalRef.value?.resumeRecording();
}

function toggleRecording() {
  if (!mainTerminalRef.value) return;
  if (isRecordingSession.value) {
    const data = mainTerminalRef.value.stopRecording();
    if (data && data.events.length > 0) {
      const session = store.sessions[currentView.value];
      recordingTitle.value = session?.profile_name || `${session?.username}@${session?.host}` || "terminal";
      recordingData.value = data;
    }
    isRecordingSession.value = false;
  } else {
    mainTerminalRef.value.startRecording();
    isRecordingSession.value = true;
  }
}

// Split pane state
type SplitDirection = "none" | "horizontal" | "vertical" | "quad";
const splitDirection = ref<SplitDirection>("none");
const splitPanes = ref<(string | null)[]>([]);
const activePaneIndex = ref(0);

function toggleSplit(dir: SplitDirection) {
  if (splitDirection.value === dir) {
    // Toggle off
    splitDirection.value = "none";
    splitPanes.value = [];
    return;
  }
  splitDirection.value = dir;
  const current = currentView.value !== "home" && currentView.value !== "settings" ? currentView.value : null;
  if (dir === "horizontal" || dir === "vertical") {
    splitPanes.value = [current, null];
  } else {
    splitPanes.value = [current, null, null, null];
  }
  activePaneIndex.value = 0;
}

function setPaneSession(index: number, sessionId: string) {
  // Remove from any other pane first (prevent duplicates)
  splitPanes.value = splitPanes.value.map((id, i) => i === index ? sessionId : (id === sessionId ? null : id));
  activePaneIndex.value = index;
  store.setActiveSession(sessionId);
}

function focusPane(index: number) {
  activePaneIndex.value = index;
  const sid = splitPanes.value[index];
  if (sid) store.setActiveSession(sid);
}

const isSplitMode = computed(() => splitDirection.value !== "none");

// Split pane resize
const splitRatio = ref(0.5);
const splitRatioV = ref(0.5); // vertical ratio for quad mode
const isResizing = ref(false);
const resizeAxis = ref<"h" | "v">("h");
const splitContainerRef = ref<HTMLElement | null>(null);

function onDividerMouseDown(e: MouseEvent, axis: "h" | "v") {
  e.preventDefault();
  isResizing.value = true;
  resizeAxis.value = axis;

  const onMouseMove = (ev: MouseEvent) => {
    if (!splitContainerRef.value) return;
    const rect = splitContainerRef.value.getBoundingClientRect();
    if (axis === "h") {
      splitRatio.value = Math.max(0.15, Math.min(0.85, (ev.clientX - rect.left) / rect.width));
    } else {
      splitRatioV.value = Math.max(0.15, Math.min(0.85, (ev.clientY - rect.top) / rect.height));
    }
  };

  const onMouseUp = () => {
    isResizing.value = false;
    window.removeEventListener("mousemove", onMouseMove);
    window.removeEventListener("mouseup", onMouseUp);
  };

  window.addEventListener("mousemove", onMouseMove);
  window.addEventListener("mouseup", onMouseUp);
}

function availableSessionsForPane(paneIndex: number) {
  const assigned = new Set(splitPanes.value.filter((id, i) => id && i !== paneIndex));
  return sessionList.value.filter(s => s.status === "connected" && !assigned.has(s.id));
}

const paneConnecting = ref<Record<number, string | null>>({});

async function connectFromPane(profile: ProfileSummary, paneIndex: number) {
  if (paneConnecting.value[paneIndex]) return; // prevent double-click
  paneConnecting.value[paneIndex] = profile.name;
  try {
    const sessionId = await invoke<string>("profile_connect", {
      profileId: profile.id,
      cols: 80,
      rows: 24,
      timeoutSecs: store.connectTimeout,
    });
    store.addSession(sessionId, {
      id: sessionId,
      host: profile.host,
      port: profile.port,
      username: profile.username,
      connected_at: new Date().toISOString(),
      status: "connected",
      profile_id: profile.id,
      profile_name: profile.name,
    });
    setPaneSession(paneIndex, sessionId);
    recordLastConnected(profile.id);
    detectOs(sessionId, profile.id);
  } catch (e) {
    const err = String(e);
    if (err.includes("vault_locked")) {
      handleVaultLocked(() => connectFromPane(profile, paneIndex));
    }
  } finally {
    paneConnecting.value[paneIndex] = null;
  }
}

// Session logging state
const loggingSessions = ref<Set<string>>(new Set());

async function toggleLogging(sessionId: string) {
  if (loggingSessions.value.has(sessionId)) {
    await invoke("ssh_stop_logging", { sessionId });
    loggingSessions.value.delete(sessionId);
    loggingSessions.value = new Set(loggingSessions.value);
  } else {
    const session = store.sessions[sessionId];
    const name = session?.profile_name || `${session?.username}@${session?.host}`;
    const timestamp = new Date().toISOString().replace(/[:.]/g, "-");
    const filename = `${name}-${timestamp}.log`;
    // Save to Downloads or home dir
    const dir = await getLogsDir();
    const path = `${dir}/${filename}`;
    await invoke("ssh_start_logging", { sessionId, path });
    loggingSessions.value.add(sessionId);
    loggingSessions.value = new Set(loggingSessions.value);
  }
}

async function getLogsDir(): Promise<string> {
  // Use app data dir logs subdirectory
  const { appDataDir } = await import("@tauri-apps/api/path");
  const dir = await appDataDir();
  return `${dir}logs`;
}

function sendSnippetToTerminal(command: string) {
  const sessionId = store.activeSessionId;
  if (!sessionId) return;
  // Write command + Enter to the active session
  invoke("ssh_write", { sessionId, data: command + "\n" }).catch(() => {});
}

// Keyboard shortcuts
function handleGlobalKeydown(e: KeyboardEvent) {
  const meta = e.metaKey || e.ctrlKey;

  if (meta && e.key === "t") {
    e.preventDefault();
    toggleConnectDropdown();
  }

  if (meta && e.key === "w") {
    e.preventDefault();
    if (currentView.value !== "home" && currentView.value !== "settings") {
      requestCloseSession(currentView.value);
    }
  }

  // Cmd+1-9 switch tabs
  if (meta && e.key >= "1" && e.key <= "9") {
    e.preventDefault();
    const idx = parseInt(e.key) - 1;
    if (idx === 0) {
      switchToHome();
    } else {
      const sessions = Object.values(store.sessions);
      if (idx - 1 < sessions.length) {
        switchToSession(sessions[idx - 1].id);
      }
    }
  }

  // Cmd+K — command palette
  if (meta && e.key === "k") {
    e.preventDefault();
    openPalette();
  }

  // Cmd+= / Cmd+- — zoom font size
  if (meta && (e.key === "=" || e.key === "+")) {
    e.preventDefault();
    store.setFontSize(store.fontSize + 1);
  }
  if (meta && e.key === "-") {
    e.preventDefault();
    store.setFontSize(store.fontSize - 1);
  }
  if (meta && e.key === "0") {
    e.preventDefault();
    store.setFontSize(14);
  }

  // Cmd+Shift+] / Cmd+Shift+[ — next/prev tab
  if (meta && e.shiftKey && (e.key === "]" || e.key === "[")) {
    e.preventDefault();
    const sessions = Object.values(store.sessions);
    const allViews = ["home", ...sessions.map((s) => s.id)];
    const currentIdx = allViews.indexOf(currentView.value);
    if (currentIdx < 0) return;

    const nextIdx = e.key === "]"
      ? (currentIdx + 1) % allViews.length
      : (currentIdx - 1 + allViews.length) % allViews.length;

    const next = allViews[nextIdx];
    if (next === "home") switchToHome();
    else switchToSession(next);
  }
}

function handleClickOutside(_e: MouseEvent) {
  // Dropdowns now use fixed backdrop overlays for click-outside
}

onMounted(() => {
  window.addEventListener("keydown", handleGlobalKeydown);
  window.addEventListener("click", handleClickOutside);
  setupAutoReconnect();
});

onUnmounted(() => {
  window.removeEventListener("keydown", handleGlobalKeydown);
  window.removeEventListener("click", handleClickOutside);
  unlistenSshClosed?.();
});

const sessionList = computed(() => {
  const all = Object.values(store.sessions);
  const pinned = all.filter(s => store.pinnedSessions.has(s.id));
  const unpinned = all.filter(s => !store.pinnedSessions.has(s.id));
  return [...pinned, ...unpinned];
});

const sessionTabLabel = computed(() => {
  const list = sessionList.value;
  const nameCount: Record<string, number> = {};
  for (const s of list) {
    const name = s.profile_name || `${s.username}@${s.host}`;
    nameCount[name] = (nameCount[name] || 0) + 1;
  }
  const nameIndex: Record<string, number> = {};
  const labels: Record<string, string> = {};
  for (const s of list) {
    const name = s.profile_name || `${s.username}@${s.host}`;
    nameIndex[name] = (nameIndex[name] || 0) + 1;
    labels[s.id] = nameCount[name] > 1 ? `(${nameIndex[name]}) ${name}` : name;
  }
  return labels;
});

function switchToSession(id: string) {
  const session = store.sessions[id];
  if (session?.status === "disconnected" && session.profile_id) {
    reconnectSession(session);
    return;
  }
  store.setActiveSession(id);
  store.clearUnread(id);
  currentView.value = id;
}

function switchToHome() {
  currentView.value = "home";
  splitDirection.value = "none";
  splitPanes.value = [];
}

function switchToSettings() {
  currentView.value = "settings";
  splitDirection.value = "none";
  splitPanes.value = [];
}

// Connection loading overlay state
const connectingProfile = ref<ProfileSummary | null>(null);
const connectError = ref<string | null>(null);

function onConnecting(profile: ProfileSummary) {
  connectingProfile.value = profile;
  connectError.value = null;
}

function onConnectDone(error: string | null) {
  if (error) {
    connectError.value = error;
  } else {
    setTimeout(() => {
      connectingProfile.value = null;
    }, 300);
  }
}

// Profile/Group editor state
const showProfileEditor = ref(false);
const editingProfile = ref<ProfileSummary | undefined>();
const showGroupEditor = ref(false);
const editingGroup = ref<Group | undefined>();

function openNewConnection() {
  editingProfile.value = undefined;
  quickConnectData.value = undefined;
  showProfileEditor.value = true;
}

function openEditProfile(profile: ProfileSummary) {
  editingProfile.value = profile;
  showProfileEditor.value = true;
}

function onConnected(newSessionId: string) {
  currentView.value = newSessionId;
  detectOs(newSessionId);
}

async function detectOs(sessionId: string, profileId?: string) {
  try {
    const osInfo = await invoke<{ os_type: string; distro: string }>("ssh_detect_os", {
      sessionId,
      profileId: profileId ?? null,
    });
    store.updateSessionOs(sessionId, osInfo);
    // Reload profiles to get updated detected_os
    if (profileId) {
      await profilesStore.loadAll();
    }
  } catch {
    // OS detection is best-effort, don't block on failure
  }
}

async function disconnectSession(sessionId: string) {
  try {
    await invoke("ssh_disconnect", { sessionId });
  } catch {
    // May already be disconnected
  }
  store.removeSession(sessionId);
  // If closing the current view, go home
  if (currentView.value === sessionId) {
    const remaining = Object.keys(store.sessions);
    currentView.value = remaining.length > 0 ? remaining[0] : "home";
  }
}

// Reconnect a disconnected session (in-place, keeps tab position)
async function reconnectSession(session: typeof store.sessions[string]) {
  if (!session.profile_id) return;

  const profile = profilesStore.profiles[session.profile_id];
  if (!profile) return;

  const oldId = session.id;

  // Mark as connecting (reuse existing tab)
  store.sessions[oldId].status = "connected";

  try {
    const newSessionId = await invoke<string>("profile_connect", {
      profileId: profile.id,
      cols: 80,
      rows: 24,
    });

    // Replace the old session with the new one in-place
    store.replaceSession(oldId, newSessionId, {
      id: newSessionId,
      host: profile.host,
      port: profile.port,
      username: profile.username,
      connected_at: new Date().toISOString(),
      status: "connected",
      profile_id: profile.id,
      profile_name: profile.name,
      os_info: session.os_info,
    });

    // Update current view if we were viewing the old session
    if (currentView.value === oldId) {
      currentView.value = newSessionId;
    }

    // Update split panes if the old session was in one
    splitPanes.value = splitPanes.value.map(id => id === oldId ? newSessionId : id);

    recordLastConnected(profile.id);
    detectOs(newSessionId, profile.id);
  } catch (e) {
    const err = String(e);
    // Revert to disconnected on failure
    if (store.sessions[oldId]) {
      store.sessions[oldId].status = "disconnected";
    }
    if (err.includes("vault_locked")) {
      handleVaultLocked(() => reconnectSession(session));
    }
  }
}

// Reconnect all disconnected sessions
const disconnectedSessions = computed(() =>
  sessionList.value.filter(s => s.status === "disconnected" && s.profile_id)
);

async function reconnectAll() {
  for (const session of disconnectedSessions.value) {
    await reconnectSession(session);
  }
}


// Vault dialog state
const showVault = ref(false);
const vaultMode = ref<"setup" | "unlock" | "change">("unlock");
const pendingAction = ref<(() => void) | null>(null);

function handleVaultLocked(retryFn: () => void) {
  pendingAction.value = retryFn;
  vaultMode.value = profilesStore.vaultStatus.initialized ? "unlock" : "setup";
  showVault.value = true;
}

function onVaultUnlocked() {
  if (pendingAction.value) {
    pendingAction.value();
    pendingAction.value = null;
  }
}

function openChangePassword() {
  vaultMode.value = "change";
  showVault.value = true;
}

// Register vault locked handler
profilesStore.onVaultLocked = handleVaultLocked;

// Command palette (⌘K)
const showPalette = ref(false);
const paletteQuery = ref("");
const paletteInputRef = ref<HTMLInputElement | null>(null);
const paletteSelectedIndex = ref(0);

interface PaletteItem {
  id: string;
  label: string;
  sublabel?: string;
  icon: string;
  action: () => void;
  category: string;
}

const paletteItems = computed<PaletteItem[]>(() => {
  const items: PaletteItem[] = [];

  // Switch to tabs
  for (const s of sessionList.value) {
    items.push({
      id: `tab-${s.id}`,
      label: s.profile_name || `${s.username}@${s.host}`,
      sublabel: s.status,
      icon: s.status === "connected" ? "mdi:console" : "mdi:console-line",
      action: () => { switchToSession(s.id); showPalette.value = false; },
      category: "Tabs",
    });
  }

  // Connect to servers
  for (const p of allProfiles.value) {
    items.push({
      id: `connect-${p.id}`,
      label: `Connect: ${p.name}`,
      sublabel: `${p.username}@${p.host}`,
      icon: "mdi:lan-connect",
      action: () => { connectFromGrid(p); showPalette.value = false; },
      category: "Connect",
    });
  }

  // Actions
  items.push(
    { id: "home", label: "Go to Home", icon: "mdi:home", action: () => { switchToHome(); showPalette.value = false; }, category: "Navigate" },
    { id: "settings", label: "Open Settings", icon: "mdi:cog", action: () => { switchToSettings(); showPalette.value = false; }, category: "Navigate" },
    { id: "new", label: "Add New Server", icon: "mdi:plus", action: () => { openNewConnection(); showPalette.value = false; }, category: "Actions" },
    { id: "snippets", label: "Open Snippets", icon: "mdi:console", action: () => { showSnippets.value = true; showPalette.value = false; }, category: "Actions" },
    { id: "keys", label: "Open Key Manager", icon: "mdi:key-variant", action: () => { showKeyManager.value = true; showPalette.value = false; }, category: "Actions" },
    { id: "recipes", label: "Open Recipes", icon: "mdi:book-play-outline", action: () => { showRecipes.value = true; showPalette.value = false; }, category: "Actions" },
  );

  return items;
});

const filteredPaletteItems = computed(() => {
  const q = paletteQuery.value.toLowerCase().trim();
  if (!q) return paletteItems.value;
  return paletteItems.value.filter(i =>
    i.label.toLowerCase().includes(q) || (i.sublabel?.toLowerCase().includes(q))
  );
});

function openPalette() {
  showPalette.value = true;
  paletteQuery.value = "";
  paletteSelectedIndex.value = 0;
  nextTick(() => paletteInputRef.value?.focus());
}

function paletteKeydown(e: KeyboardEvent) {
  const items = filteredPaletteItems.value;
  if (e.key === "ArrowDown") {
    e.preventDefault();
    paletteSelectedIndex.value = (paletteSelectedIndex.value + 1) % items.length;
  } else if (e.key === "ArrowUp") {
    e.preventDefault();
    paletteSelectedIndex.value = (paletteSelectedIndex.value - 1 + items.length) % items.length;
  } else if (e.key === "Enter" && items.length > 0) {
    e.preventDefault();
    items[paletteSelectedIndex.value].action();
  }
}

// Auto-reconnect on disconnect
const autoReconnect = ref(true);
const reconnectAttempts = ref<Record<string, number>>({});
const MAX_RECONNECT_ATTEMPTS = 3;
const RECONNECT_DELAY = 3000;
let unlistenSshClosed: UnlistenFn | null = null;

async function setupAutoReconnect() {
  unlistenSshClosed = await listen<{ session_id: string; reason: string }>(
    "ssh_closed",
    (event) => {
      if (!autoReconnect.value) return;
      const session = store.sessions[event.payload.session_id];
      if (!session || !session.profile_id) return;

      const key = session.profile_id;
      const attempts = reconnectAttempts.value[key] || 0;
      if (attempts >= MAX_RECONNECT_ATTEMPTS) {
        reconnectAttempts.value[key] = 0;
        return;
      }

      reconnectAttempts.value[key] = attempts + 1;
      setTimeout(() => {
        const s = store.sessions[event.payload.session_id];
        if (s && s.status === "disconnected") {
          reconnectSession(s);
        }
      }, RECONNECT_DELAY);
    }
  );
}

// Reset reconnect counter on successful connect
watch(() => Object.values(store.sessions).filter(s => s.status === "connected").length, () => {
  for (const s of Object.values(store.sessions)) {
    if (s.status === "connected" && s.profile_id) {
      reconnectAttempts.value[s.profile_id] = 0;
    }
  }
});

// Last connected tracking
const lastConnected = ref<Record<string, string>>(
  JSON.parse(localStorage.getItem("shello-last-connected") || "{}")
);

function recordLastConnected(profileId: string) {
  lastConnected.value[profileId] = new Date().toISOString();
  localStorage.setItem("shello-last-connected", JSON.stringify(lastConnected.value));
}

function formatTimeAgo(iso: string): string {
  const diff = Date.now() - new Date(iso).getTime();
  const mins = Math.floor(diff / 60000);
  if (mins < 1) return "just now";
  if (mins < 60) return `${mins}m ago`;
  const hours = Math.floor(mins / 60);
  if (hours < 24) return `${hours}h ago`;
  const days = Math.floor(hours / 24);
  if (days < 30) return `${days}d ago`;
  return new Date(iso).toLocaleDateString();
}

// Home grid helpers
const homeSearch = ref("");
const activeGroupFilter = ref<string | null>(null);

const allProfiles = computed(() =>
  Object.values(profilesStore.profiles).sort(
    (a, b) => a.sort_order - b.sort_order || a.name.localeCompare(b.name)
  )
);

const allGroups = computed(() => Object.values(profilesStore.groups));

// Multi-select servers
const selectedServers = ref<Set<string>>(new Set());

function handleServerClick(e: MouseEvent, profile: ProfileSummary) {
  if (e.shiftKey) {
    e.preventDefault();
    if (selectedServers.value.has(profile.id)) {
      selectedServers.value.delete(profile.id);
    } else {
      selectedServers.value.add(profile.id);
    }
    selectedServers.value = new Set(selectedServers.value);
    return;
  }

  if (selectedServers.value.size > 0) {
    selectedServers.value = new Set();
  }

  connectFromGrid(profile);
}

async function connectSelected() {
  for (const id of selectedServers.value) {
    const profile = profilesStore.profiles[id];
    if (profile) await connectFromGrid(profile);
  }
  selectedServers.value = new Set();
}

// Server card drag reorder
const dragCardId = ref<string | null>(null);
const dragOverCardId = ref<string | null>(null);

function onCardDragStart(e: DragEvent, profileId: string) {
  dragCardId.value = profileId;
  if (e.dataTransfer) {
    e.dataTransfer.effectAllowed = "move";
    e.dataTransfer.setData("text/plain", profileId);
  }
}

function onCardDragOver(e: DragEvent, profileId: string) {
  e.preventDefault();
  if (e.dataTransfer) e.dataTransfer.dropEffect = "move";
  dragOverCardId.value = profileId;
}

function onCardDrop(e: DragEvent, targetId: string) {
  e.preventDefault();
  if (dragCardId.value && dragCardId.value !== targetId) {
    // Reorder in profiles store
    const ids = allProfiles.value.map(p => p.id);
    const fromIdx = ids.indexOf(dragCardId.value);
    const toIdx = ids.indexOf(targetId);
    if (fromIdx >= 0 && toIdx >= 0) {
      // Update sort_order client-side
      const reordered = [...allProfiles.value];
      const [moved] = reordered.splice(fromIdx, 1);
      reordered.splice(toIdx, 0, moved);
      reordered.forEach((p, i) => {
        profilesStore.profiles[p.id].sort_order = i;
      });
    }
  }
  dragCardId.value = null;
  dragOverCardId.value = null;
}

function onCardDragEnd() {
  dragCardId.value = null;
  dragOverCardId.value = null;
}

const filteredHomeProfiles = computed(() => {
  let list = allProfiles.value;
  if (activeGroupFilter.value) {
    list = list.filter(p => p.group_id === activeGroupFilter.value);
  }
  const q = homeSearch.value.toLowerCase().trim();
  if (q) {
    list = list.filter(p =>
      p.name.toLowerCase().includes(q) ||
      p.host.toLowerCase().includes(q) ||
      p.username.toLowerCase().includes(q)
    );
  }
  return list;
});

function getGroup(groupId: string | null): Group | null {
  if (!groupId) return null;
  return profilesStore.groups[groupId] ?? null;
}

async function connectFromGrid(profile: ProfileSummary) {
  onConnecting(profile);
  try {
    const sessionId = await invoke<string>("profile_connect", {
      profileId: profile.id,
      cols: 80,
      rows: 24,
      timeoutSecs: store.connectTimeout,
    });
    store.addSession(sessionId, {
      id: sessionId,
      host: profile.host,
      port: profile.port,
      username: profile.username,
      connected_at: new Date().toISOString(),
      status: "connected",
      profile_id: profile.id,
      profile_name: profile.name,
    });
    onConnectDone(null);
    currentView.value = sessionId;
    recordLastConnected(profile.id);
    detectOs(sessionId, profile.id);
  } catch (e) {
    const err = String(e);
    if (err.includes("vault_locked")) {
      onConnectDone(null);
      handleVaultLocked(() => connectFromGrid(profile));
    } else {
      onConnectDone(err);
    }
  }
}

</script>

<template>
  <div class="h-screen flex flex-col bg-otter-dark overflow-hidden">
    <!-- Top tab bar -->
    <div class="flex items-stretch bg-otter-card border-b border-otter-border h-10 flex-shrink-0">
      <!-- Home tab -->
      <button
        class="flex items-center gap-2 px-3 h-full border-r border-otter-border
               text-xs transition-colors flex-shrink-0"
        :class="currentView === 'home'
          ? 'bg-otter-dark text-otter-teal'
          : 'text-otter-muted hover:text-otter-text hover:bg-otter-surface/50'"
        @click="switchToHome"
      >
        <LogoIcon :size="16" />
        <span>Home</span>
      </button>

      <!-- Session tabs (scrollable, drag & drop) -->
      <div
        ref="tabContainerRef"
        class="flex-1 flex items-stretch overflow-x-auto min-w-0 scrollbar-hide"
      >
        <button
          v-for="session in sessionList"
          :key="session.id"
          draggable="true"
          class="flex items-center gap-2 px-3 h-full border-r border-otter-border
                 text-xs transition-colors flex-shrink-0 max-w-[180px] group cursor-grab active:cursor-grabbing"
          :class="[
            currentView === session.id
              ? 'bg-otter-dark text-otter-text'
              : 'text-otter-muted hover:text-otter-text hover:bg-otter-surface/50',
            dragOverTabId === session.id && dragTabId !== session.id ? 'border-l-2 border-l-otter-teal' : '',
            dragTabId === session.id ? 'opacity-40' : ''
          ]"
          @click="switchToSession(session.id)"
          @dragstart="onTabDragStart($event, session.id)"
          @dragover="onTabDragOver($event, session.id)"
          @drop="onTabDrop($event, session.id)"
          @dragend="onTabDragEnd"
        >
          <span class="relative flex-shrink-0 w-2 h-2">
            <span
              v-if="session.status === 'connected'"
              class="absolute inset-0 rounded-full bg-emerald-400 heartbeat"
            ></span>
            <span
              class="relative block w-2 h-2 rounded-full"
              :class="session.status === 'connected' ? 'bg-emerald-400' : 'bg-otter-coral'"
            ></span>
          </span>
          <Icon
            :icon="getOsIcon(session.os_info)"
            class="w-4 h-4 flex-shrink-0"
            :style="{ color: session.status === 'connected' ? getOsColor(session.os_info) : undefined }"
            :class="session.status !== 'connected' ? 'text-otter-subtle' : ''"
          />
          <span class="truncate" :class="session.profile_name ? '' : 'font-mono'">{{ sessionTabLabel[session.id] }}</span>
          <span v-if="store.hasUnread[session.id] && currentView !== session.id" class="w-1.5 h-1.5 rounded-full bg-otter-blue flex-shrink-0"></span>
          <!-- Pin indicator -->
          <Icon v-if="store.pinnedSessions.has(session.id)" icon="mdi:pin" class="w-3 h-3 text-otter-subtle flex-shrink-0" />
          <button
            v-if="session.status === 'disconnected' && session.profile_id"
            class="ml-auto text-otter-teal flex-shrink-0 opacity-0 group-hover:opacity-100 transition-opacity"
            title="Reconnect"
            @click.stop="reconnectSession(session)"
          >
            <Icon icon="mdi:refresh" class="w-3 h-3" />
          </button>
          <!-- Pin/Unpin button -->
          <button
            class="text-otter-subtle hover:text-otter-text flex-shrink-0
                   opacity-0 group-hover:opacity-100 transition-opacity"
            :class="{ 'ml-auto': session.status !== 'disconnected' || !session.profile_id }"
            :title="store.pinnedSessions.has(session.id) ? 'Unpin' : 'Pin'"
            @click.stop="store.togglePin(session.id)"
          >
            <Icon :icon="store.pinnedSessions.has(session.id) ? 'mdi:pin-off' : 'mdi:pin'" class="w-3 h-3" />
          </button>
          <!-- Close (hidden for pinned) -->
          <button
            v-if="!store.pinnedSessions.has(session.id)"
            class="text-otter-subtle hover:text-otter-coral flex-shrink-0
                   opacity-0 group-hover:opacity-100 transition-opacity"
            title="Close"
            @click.stop="requestCloseSession(session.id)"
          >
            ×
          </button>
        </button>

        <!-- Connect (inline next to tabs) -->
        <button
          class="flex items-center justify-center w-8 h-full flex-shrink-0
                 text-otter-subtle hover:bg-otter-dark hover:text-otter-text transition-colors"
          title="Connect (⌘T)"
          @click="toggleConnectDropdown"
        >
          <Icon icon="mdi:plus" class="w-3.5 h-3.5" />
        </button>
      </div>

      <!-- Tab overflow dropdown -->
      <div v-if="sessionList.length > 0" class="relative flex-shrink-0 border-l border-otter-border">
        <button
          class="flex items-center justify-center w-8 h-full text-otter-muted hover:text-otter-text transition-colors"
          title="All tabs"
          @click="showTabOverflow = !showTabOverflow"
        >
          <Icon icon="mdi:chevron-down" class="w-3.5 h-3.5" />
        </button>
        <div
          v-if="showTabOverflow"
          class="fixed inset-0 z-40"
          @click="showTabOverflow = false"
        ></div>
        <div
          v-if="showTabOverflow"
          class="absolute top-full right-0 mt-1 w-56 rounded-lg bg-otter-card border border-otter-border
                 shadow-xl z-50 overflow-hidden"
        >
          <div class="max-h-60 overflow-y-auto py-1">
            <button
              v-for="session in sessionList"
              :key="session.id"
              class="w-full flex items-center gap-2 px-3 py-2 text-left text-xs
                     hover:bg-otter-surface transition-colors"
              :class="currentView === session.id ? 'text-otter-teal' : 'text-otter-text'"
              @click="switchToSession(session.id); showTabOverflow = false"
            >
              <span
                class="w-2 h-2 rounded-full flex-shrink-0"
                :class="session.status === 'connected' ? 'bg-emerald-400' : 'bg-otter-coral'"
              ></span>
              <Icon
                :icon="getOsIcon(session.os_info)"
                class="w-3.5 h-3.5 flex-shrink-0"
                :style="{ color: session.status === 'connected' ? getOsColor(session.os_info) : undefined }"
              />
              <span class="truncate">{{ session.profile_name || `${session.username}@${session.host}` }}</span>
            </button>
          </div>
          <div v-if="disconnectedSessions.length > 0" class="border-t border-otter-border px-3 py-2">
            <button
              class="w-full flex items-center justify-center gap-1.5 text-xs text-otter-coral hover:text-otter-teal transition-colors"
              @click="reconnectAll(); showTabOverflow = false"
            >
              <Icon icon="mdi:connection" class="w-3.5 h-3.5" />
              Reconnect all ({{ disconnectedSessions.length }})
            </button>
          </div>
        </div>
      </div>

      <!-- Right actions (icon-only) -->
      <div class="flex items-stretch flex-shrink-0 border-l border-otter-border">
        <!-- Snippets -->
        <button
          class="flex items-center justify-center w-9 h-full transition-colors border-r border-otter-border
                 text-otter-muted hover:text-otter-text"
          title="Snippets"
          @click="showSnippets = true"
        >
          <Icon icon="mdi:console" class="w-4 h-4" />
        </button>
        <!-- Recipes -->
        <button
          class="flex items-center justify-center w-9 h-full transition-colors border-r border-otter-border
                 text-otter-muted hover:text-otter-text"
          title="Recipes"
          @click="showRecipes = true"
        >
          <Icon icon="mdi:book-play-outline" class="w-4 h-4" />
        </button>
        <!-- Keys -->
        <button
          class="flex items-center justify-center w-9 h-full transition-colors border-r border-otter-border
                 text-otter-muted hover:text-otter-text"
          title="Keys"
          @click="showKeyManager = true"
        >
          <Icon icon="mdi:key-variant" class="w-4 h-4" />
        </button>
        <!-- Log toggle -->
        <button
          v-if="currentView !== 'home' && currentView !== 'settings'"
          class="flex items-center justify-center w-9 h-full transition-colors border-r border-otter-border"
          :class="loggingSessions.has(currentView)
            ? 'text-otter-coral'
            : 'text-otter-muted hover:text-otter-text'"
          :title="loggingSessions.has(currentView) ? 'Stop logging' : 'Start logging'"
          @click="toggleLogging(currentView)"
        >
          <Icon :icon="loggingSessions.has(currentView) ? 'mdi:record-circle' : 'mdi:record-circle-outline'" class="w-4 h-4" />
        </button>
        <!-- Split buttons -->
        <button
          v-if="currentView !== 'home' && currentView !== 'settings'"
          class="flex items-center justify-center w-9 h-full transition-colors border-r border-otter-border"
          :class="splitDirection === 'horizontal' ? 'text-otter-teal' : 'text-otter-muted hover:text-otter-text'"
          title="Split Horizontal"
          @click="toggleSplit('horizontal')"
        >
          <Icon icon="mdi:dock-right" class="w-4 h-4" />
        </button>
        <button
          v-if="currentView !== 'home' && currentView !== 'settings'"
          class="flex items-center justify-center w-9 h-full transition-colors border-r border-otter-border"
          :class="splitDirection === 'vertical' ? 'text-otter-teal' : 'text-otter-muted hover:text-otter-text'"
          title="Split Vertical"
          @click="toggleSplit('vertical')"
        >
          <Icon icon="mdi:dock-bottom" class="w-4 h-4" />
        </button>
        <button
          v-if="currentView !== 'home' && currentView !== 'settings'"
          class="flex items-center justify-center w-9 h-full transition-colors border-r border-otter-border"
          :class="splitDirection === 'quad' ? 'text-otter-teal' : 'text-otter-muted hover:text-otter-text'"
          title="Split Quad"
          @click="toggleSplit('quad')"
        >
          <Icon icon="mdi:view-grid-outline" class="w-4 h-4" />
        </button>
        <!-- Settings -->
        <button
          class="flex items-center justify-center w-9 h-full transition-colors"
          :class="currentView === 'settings'
            ? 'text-otter-teal'
            : 'text-otter-muted hover:text-otter-text'"
          title="Settings"
          @click="switchToSettings"
        >
          <Icon icon="mdi:cog" class="w-4 h-4" />
        </button>
      </div>
    </div>

    <!-- Main content area -->
    <div class="flex-1 overflow-hidden relative">
      <!-- Connection loading overlay -->
      <Transition name="fade">
        <div
          v-if="connectingProfile"
          class="absolute inset-0 z-10 flex items-center justify-center bg-otter-dark/90 backdrop-blur-sm"
        >
          <div class="flex flex-col items-center gap-5 max-w-xs text-center">
            <div class="flex flex-col items-center gap-3">
              <div
                class="w-16 h-16 rounded-2xl bg-otter-surface border border-otter-border flex items-center justify-center"
                :class="{ 'animate-pulse': !connectError }"
              >
                <Icon
                  v-if="connectingProfile.detected_os"
                  :icon="getOsIcon({ os_type: '', distro: connectingProfile.detected_os })"
                  class="w-8 h-8"
                  :style="{ color: getOsColor({ os_type: '', distro: connectingProfile.detected_os }) }"
                />
                <LogoIcon v-else :size="32" />
              </div>
              <div>
                <p class="text-otter-text text-sm font-semibold">
                  {{ connectingProfile.name }}
                </p>
                <p class="text-otter-muted text-xs font-mono mt-0.5">
                  {{ connectingProfile.username }}@{{ connectingProfile.host }}
                </p>
              </div>
            </div>

            <!-- Connecting spinner -->
            <div v-if="!connectError" class="flex items-center gap-3 px-4 py-2.5 rounded-lg bg-otter-surface">
              <span class="inline-block w-3.5 h-3.5 border-2 border-otter-teal border-t-transparent rounded-full animate-spin"></span>
              <span class="text-xs text-otter-text">Connecting...</span>
            </div>

            <!-- Error state -->
            <div v-else class="flex flex-col items-center gap-3 w-full">
              <div class="w-10 h-10 rounded-full bg-otter-coral/20 flex items-center justify-center">
                <span class="text-otter-coral text-lg">✕</span>
              </div>
              <p class="text-sm font-semibold text-otter-coral">Connection Failed</p>
              <p class="text-xs text-otter-muted break-all px-2">{{ connectError }}</p>
              <div class="flex gap-2 mt-1">
                <button
                  class="px-3 py-1.5 rounded-lg bg-otter-surface border border-otter-border
                         text-xs text-otter-text hover:border-otter-subtle transition-colors"
                  @click="connectingProfile = null; connectError = null"
                >
                  Dismiss
                </button>
                <button
                  class="px-3 py-1.5 rounded-lg bg-otter-teal text-otter-dark font-semibold
                         text-xs hover:opacity-90 transition-opacity"
                  @click="openEditProfile(connectingProfile!); connectingProfile = null; connectError = null"
                >
                  Edit Server
                </button>
              </div>
            </div>
          </div>
        </div>
      </Transition>

      <!-- Settings page -->
      <SettingsPage
        v-show="currentView === 'settings'"
        @back="switchToHome"
        @change-password="openChangePassword"
      />

      <!-- Home: server grid -->
      <div
        v-show="currentView === 'home'"
        class="h-full overflow-y-auto"
      >
        <div class="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-6 lg:py-10">
          <!-- Header -->
          <div class="flex items-center gap-3 mb-6">
            <LogoWordmark :height="32" />
            <div class="ml-auto">
              <button
                class="px-3 py-1.5 rounded-lg bg-otter-teal text-otter-dark font-semibold
                       text-xs hover:opacity-90 transition-opacity"
                @click="openNewConnection"
              >
                + Add Server
              </button>
            </div>
          </div>

          <!-- Search + group filter bar -->
          <div class="mb-5 flex flex-col gap-3">
            <div class="relative">
              <input
                v-model="homeSearch"
                class="w-full px-4 py-2.5 pl-9 rounded-lg bg-otter-card border border-otter-border
                       text-otter-text placeholder-otter-subtle text-sm
                       focus:outline-none focus:border-otter-teal-dim transition-colors"
                placeholder="Search servers..."
              />
              <Icon icon="mdi:magnify" class="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-otter-subtle" />
            </div>
            <div v-if="allGroups.length > 0" class="flex items-center gap-2 flex-wrap">
              <button
                class="px-2.5 py-1 rounded-full text-[11px] font-medium transition-colors border"
                :class="!activeGroupFilter
                  ? 'bg-otter-teal/10 border-otter-teal text-otter-teal'
                  : 'border-otter-border text-otter-muted hover:text-otter-text hover:border-otter-subtle'"
                @click="activeGroupFilter = null"
              >
                All
              </button>
              <button
                v-for="group in allGroups"
                :key="group.id"
                class="flex items-center gap-1.5 px-2.5 py-1 rounded-full text-[11px] font-medium transition-colors border"
                :class="activeGroupFilter === group.id
                  ? 'bg-otter-teal/10 border-otter-teal text-otter-teal'
                  : 'border-otter-border text-otter-muted hover:text-otter-text hover:border-otter-subtle'"
                @click="activeGroupFilter = activeGroupFilter === group.id ? null : group.id"
              >
                <span class="w-2 h-2 rounded-full flex-shrink-0" :style="{ backgroundColor: group.color }"></span>
                {{ group.name }}
              </button>
            </div>
          </div>

          <!-- Server grid -->
          <div v-if="filteredHomeProfiles.length > 0 || allProfiles.length > 0" class="grid grid-cols-1 sm:grid-cols-2 md:grid-cols-3 lg:grid-cols-4 xl:grid-cols-5 2xl:grid-cols-6 gap-3">
            <div
              v-for="profile in filteredHomeProfiles"
              :key="profile.id"
              draggable="true"
              class="group relative bg-otter-card border border-otter-border rounded-xl p-4
                     cursor-pointer hover:border-otter-teal-dim hover:bg-otter-surface/50
                     transition-all duration-200"
              :class="[
                connectingProfile?.id === profile.id ? 'opacity-50 pointer-events-none' : '',
                dragCardId === profile.id ? 'opacity-40' : '',
                dragOverCardId === profile.id && dragCardId !== profile.id ? 'ring-2 ring-otter-teal' : '',
                selectedServers.has(profile.id) ? 'ring-2 ring-otter-blue border-otter-blue' : ''
              ]"
              @click="handleServerClick($event, profile)"
              @dragstart="onCardDragStart($event, profile.id)"
              @dragover="onCardDragOver($event, profile.id)"
              @drop="onCardDrop($event, profile.id)"
              @dragend="onCardDragEnd"
            >
              <!-- Group + auth icon -->
              <div class="flex items-center gap-2 mb-2">
                <div
                  v-if="getGroup(profile.group_id)"
                  class="w-2 h-2 rounded-full flex-shrink-0"
                  :style="{ backgroundColor: getGroup(profile.group_id)!.color }"
                ></div>
                <span class="text-xs text-otter-subtle uppercase tracking-wider truncate">
                  {{ getGroup(profile.group_id)?.name || 'Ungrouped' }}
                </span>
                <Icon
                  :icon="profile.auth_type === 'key' ? 'mdi:key-variant' : 'mdi:lock'"
                  class="ml-auto w-3.5 h-3.5"
                  :class="profile.auth_type === 'key' ? 'text-otter-teal' : 'text-amber-400'"
                  :title="profile.auth_type === 'key' ? 'Key authentication' : 'Password authentication'"
                />
              </div>

              <div class="flex items-center gap-3">
                <div class="w-10 h-10 rounded-lg bg-otter-surface flex items-center justify-center flex-shrink-0">
                  <Icon
                    v-if="profile.detected_os"
                    :icon="getOsIcon({ os_type: '', distro: profile.detected_os })"
                    class="w-6 h-6"
                    :style="{ color: getOsColor({ os_type: '', distro: profile.detected_os }) }"
                    :title="getOsLabel({ os_type: '', distro: profile.detected_os })"
                  />
                  <LogoIcon v-else :size="24" />
                </div>
                <div class="min-w-0 flex-1">
                  <h3 class="text-base font-semibold text-otter-text truncate">
                    {{ profile.name }}
                  </h3>
                  <p v-if="lastConnected[profile.id]" class="text-[10px] text-otter-subtle mt-0.5">
                    {{ formatTimeAgo(lastConnected[profile.id]) }}
                  </p>
                </div>
              </div>

              <!-- Connecting state -->
              <div
                v-if="connectingProfile?.id === profile.id"
                class="absolute inset-0 rounded-xl bg-otter-dark/80 flex items-center justify-center"
              >
                <span class="inline-block w-4 h-4 border-2 border-otter-teal border-t-transparent rounded-full animate-spin"></span>
              </div>

              <!-- Hover edit button -->
              <button
                class="absolute right-2 bottom-2 flex items-center gap-1 px-2 py-1
                       rounded-md bg-otter-surface/90 border border-otter-border
                       text-otter-muted hover:text-otter-text hover:border-otter-subtle
                       opacity-0 group-hover:opacity-100 transition-all"
                @click.stop="openEditProfile(profile)"
              >
                <svg class="w-3 h-3" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
                  <path d="M11.5 1.5l3 3L5 14H2v-3L11.5 1.5z" />
                </svg>
                <span class="text-xs">Edit</span>
              </button>
            </div>

            <!-- Add new server card -->
            <div
              class="flex flex-col items-center justify-center gap-2 bg-otter-card/50
                     border border-dashed border-otter-border rounded-xl p-4 cursor-pointer
                     hover:border-otter-teal-dim hover:bg-otter-surface/30 transition-all duration-200
                     min-h-[120px]"
              @click="openNewConnection"
            >
              <span class="text-2xl text-otter-subtle">+</span>
              <span class="text-xs text-otter-subtle">Add Server</span>
            </div>
          </div>

          <!-- Connect selected floating bar -->
          <div
            v-if="selectedServers.size > 0"
            class="mt-4 flex items-center justify-between bg-otter-card border border-otter-blue rounded-xl px-4 py-3"
          >
            <span class="text-sm text-otter-text">{{ selectedServers.size }} server{{ selectedServers.size > 1 ? 's' : '' }} selected</span>
            <div class="flex gap-2">
              <button
                class="px-3 py-1.5 rounded-lg bg-otter-surface border border-otter-border
                       text-xs text-otter-muted hover:text-otter-text transition-colors"
                @click="selectedServers = new Set()"
              >
                Cancel
              </button>
              <button
                class="px-3 py-1.5 rounded-lg bg-otter-teal text-otter-dark font-semibold
                       text-xs hover:opacity-90 transition-opacity"
                @click="connectSelected"
              >
                Connect All
              </button>
            </div>
          </div>

          <!-- No search results -->
          <div v-else-if="allProfiles.length > 0 && filteredHomeProfiles.length === 0" class="flex flex-col items-center justify-center py-16 gap-3">
            <Icon icon="mdi:magnify" class="w-10 h-10 text-otter-subtle" />
            <p class="text-otter-muted text-sm">No servers match your search</p>
            <button
              class="text-xs text-otter-teal hover:underline"
              @click="homeSearch = ''; activeGroupFilter = null"
            >
              Clear filters
            </button>
          </div>

          <!-- Empty state -->
          <div v-else-if="allProfiles.length === 0" class="flex flex-col items-center justify-center py-20 gap-4">
            <LogoIcon :size="64" />
            <p class="text-otter-muted text-sm">No saved servers yet</p>
            <button
              class="px-4 py-2 rounded-lg bg-otter-teal text-otter-dark font-semibold
                     text-xs hover:opacity-90 transition-opacity"
              @click="openNewConnection"
            >
              + Add Server
            </button>
          </div>
        </div>
      </div>

      <!-- Terminal sessions -->
      <template v-if="!isSplitMode">
        <!-- Single terminal mode -->
        <TerminalView
          v-for="session in sessionList"
          :key="session.id"
          :ref="(el: any) => { if (currentView === session.id) mainTerminalRef = el; }"
          v-show="currentView === session.id"
          :session-id="session.id"
          class="w-full h-full"
          @screenshot="openScreenshotPreview"
          @toggle-record="toggleRecording"
          @pause-record="pauseRecording"
          @resume-record="resumeRecording"
        />
      </template>
      <template v-else>
        <!-- Split pane mode -->
        <div
          v-show="currentView !== 'home' && currentView !== 'settings'"
          ref="splitContainerRef"
          class="w-full h-full flex"
          :class="[
            splitDirection === 'vertical' ? 'flex-col' : 'flex-row',
            isResizing ? 'select-none' : ''
          ]"
        >
          <!-- Horizontal / Vertical: 2 panes -->
          <template v-if="splitDirection === 'horizontal' || splitDirection === 'vertical'">
            <div
              class="relative bg-otter-dark overflow-hidden cursor-pointer"
              :class="activePaneIndex === 0 ? 'ring-1 ring-otter-teal ring-inset' : ''"
              :style="splitDirection === 'horizontal'
                ? { width: (splitRatio * 100) + '%' }
                : { height: (splitRatio * 100) + '%' }"
              @click="focusPane(0)"
            >
              <TerminalView v-if="splitPanes[0]" :session-id="splitPanes[0]" class="w-full h-full" />
              <div v-else class="w-full h-full flex flex-col items-center justify-center gap-3 overflow-y-auto py-4">
                <!-- Connecting spinner -->
                <template v-if="paneConnecting[0]">
                  <span class="inline-block w-5 h-5 border-2 border-otter-teal border-t-transparent rounded-full animate-spin"></span>
                  <p class="text-xs text-otter-muted">Connecting to {{ paneConnecting[0] }}...</p>
                </template>
                <template v-else>
                  <template v-if="availableSessionsForPane(0).length > 0">
                    <p class="text-[10px] text-otter-subtle uppercase tracking-wider">Active Sessions</p>
                    <div class="flex flex-col gap-1 max-w-[220px] w-full px-4">
                      <button v-for="s in availableSessionsForPane(0)" :key="s.id" class="flex items-center gap-2 px-3 py-2 rounded-lg bg-otter-surface border border-otter-border hover:border-otter-teal-dim text-left text-xs transition-colors" @click.stop="setPaneSession(0, s.id)">
                        <Icon :icon="getOsIcon(s.os_info)" class="w-3.5 h-3.5 flex-shrink-0" :style="{ color: getOsColor(s.os_info) }" />
                        <span class="truncate text-otter-text">{{ s.profile_name || `${s.username}@${s.host}` }}</span>
                      </button>
                    </div>
                  </template>
                  <template v-if="allProfiles.length > 0">
                    <p class="text-[10px] text-otter-subtle uppercase tracking-wider">Connect Server</p>
                    <div class="flex flex-col gap-1 max-w-[220px] w-full px-4">
                      <button v-for="profile in allProfiles" :key="profile.id" class="flex items-center gap-2 px-3 py-2 rounded-lg bg-otter-card border border-otter-border hover:border-otter-teal-dim text-left text-xs transition-colors" @click.stop="connectFromPane(profile, 0)">
                        <Icon v-if="profile.detected_os" :icon="getOsIcon({ os_type: '', distro: profile.detected_os })" class="w-3.5 h-3.5 flex-shrink-0" :style="{ color: getOsColor({ os_type: '', distro: profile.detected_os }) }" />
                        <LogoIcon v-else :size="14" />
                        <div class="flex flex-col min-w-0"><span class="truncate text-otter-text">{{ profile.name }}</span><span class="truncate text-[10px] text-otter-subtle font-mono">{{ profile.username }}@{{ profile.host }}</span></div>
                      </button>
                    </div>
                  </template>
                  <p v-if="availableSessionsForPane(0).length === 0 && allProfiles.length === 0" class="text-[10px] text-otter-subtle text-center py-2">No servers configured</p>
                </template>
              </div>
            </div>
            <!-- Draggable divider -->
            <div
              class="flex-shrink-0 bg-otter-border hover:bg-otter-teal/50 transition-colors"
              :class="splitDirection === 'horizontal'
                ? 'w-[3px] cursor-col-resize'
                : 'h-[3px] cursor-row-resize'"
              @mousedown="onDividerMouseDown($event, splitDirection === 'horizontal' ? 'h' : 'v')"
            ></div>
            <div
              class="relative bg-otter-dark overflow-hidden cursor-pointer flex-1"
              :class="activePaneIndex === 1 ? 'ring-1 ring-otter-teal ring-inset' : ''"
              @click="focusPane(1)"
            >
              <TerminalView v-if="splitPanes[1]" :session-id="splitPanes[1]" class="w-full h-full" />
              <div v-else class="w-full h-full flex flex-col items-center justify-center gap-3 overflow-y-auto py-4">
                <template v-if="paneConnecting[1]">
                  <span class="inline-block w-5 h-5 border-2 border-otter-teal border-t-transparent rounded-full animate-spin"></span>
                  <p class="text-xs text-otter-muted">Connecting to {{ paneConnecting[1] }}...</p>
                </template>
                <template v-else>
                  <template v-if="availableSessionsForPane(1).length > 0">
                    <p class="text-[10px] text-otter-subtle uppercase tracking-wider">Active Sessions</p>
                    <div class="flex flex-col gap-1 max-w-[220px] w-full px-4">
                      <button v-for="s in availableSessionsForPane(1)" :key="s.id" class="flex items-center gap-2 px-3 py-2 rounded-lg bg-otter-surface border border-otter-border hover:border-otter-teal-dim text-left text-xs transition-colors" @click.stop="setPaneSession(1, s.id)">
                        <Icon :icon="getOsIcon(s.os_info)" class="w-3.5 h-3.5 flex-shrink-0" :style="{ color: getOsColor(s.os_info) }" />
                        <span class="truncate text-otter-text">{{ s.profile_name || `${s.username}@${s.host}` }}</span>
                      </button>
                    </div>
                  </template>
                  <template v-if="allProfiles.length > 0">
                    <p class="text-[10px] text-otter-subtle uppercase tracking-wider">Connect Server</p>
                    <div class="flex flex-col gap-1 max-w-[220px] w-full px-4">
                      <button v-for="profile in allProfiles" :key="profile.id" class="flex items-center gap-2 px-3 py-2 rounded-lg bg-otter-card border border-otter-border hover:border-otter-teal-dim text-left text-xs transition-colors" @click.stop="connectFromPane(profile, 1)">
                        <Icon v-if="profile.detected_os" :icon="getOsIcon({ os_type: '', distro: profile.detected_os })" class="w-3.5 h-3.5 flex-shrink-0" :style="{ color: getOsColor({ os_type: '', distro: profile.detected_os }) }" />
                        <LogoIcon v-else :size="14" />
                        <div class="flex flex-col min-w-0"><span class="truncate text-otter-text">{{ profile.name }}</span><span class="truncate text-[10px] text-otter-subtle font-mono">{{ profile.username }}@{{ profile.host }}</span></div>
                      </button>
                    </div>
                  </template>
                  <p v-if="availableSessionsForPane(1).length === 0 && allProfiles.length === 0" class="text-[10px] text-otter-subtle text-center py-2">No servers configured</p>
                </template>
              </div>
            </div>
          </template>

          <!-- Quad: 4 panes with cross dividers -->
          <template v-if="splitDirection === 'quad'">
            <div class="flex flex-col" :style="{ width: (splitRatio * 100) + '%' }">
              <div class="relative bg-otter-dark overflow-hidden cursor-pointer" :class="activePaneIndex === 0 ? 'ring-1 ring-otter-teal ring-inset' : ''" :style="{ height: (splitRatioV * 100) + '%' }" @click="focusPane(0)">
                <TerminalView v-if="splitPanes[0]" :session-id="splitPanes[0]" class="w-full h-full" />
                <div v-else class="w-full h-full flex items-center justify-center text-[10px] text-otter-subtle">Select a session</div>
              </div>
              <div class="flex-shrink-0 h-[3px] bg-otter-border hover:bg-otter-teal/50 cursor-row-resize transition-colors" @mousedown="onDividerMouseDown($event, 'v')"></div>
              <div class="relative bg-otter-dark overflow-hidden cursor-pointer flex-1" :class="activePaneIndex === 2 ? 'ring-1 ring-otter-teal ring-inset' : ''" @click="focusPane(2)">
                <TerminalView v-if="splitPanes[2]" :session-id="splitPanes[2]" class="w-full h-full" />
                <div v-else class="w-full h-full flex items-center justify-center text-[10px] text-otter-subtle">Select a session</div>
              </div>
            </div>
            <div class="flex-shrink-0 w-[3px] bg-otter-border hover:bg-otter-teal/50 cursor-col-resize transition-colors" @mousedown="onDividerMouseDown($event, 'h')"></div>
            <div class="flex flex-col flex-1">
              <div class="relative bg-otter-dark overflow-hidden cursor-pointer" :class="activePaneIndex === 1 ? 'ring-1 ring-otter-teal ring-inset' : ''" :style="{ height: (splitRatioV * 100) + '%' }" @click="focusPane(1)">
                <TerminalView v-if="splitPanes[1]" :session-id="splitPanes[1]" class="w-full h-full" />
                <div v-else class="w-full h-full flex items-center justify-center text-[10px] text-otter-subtle">Select a session</div>
              </div>
              <div class="flex-shrink-0 h-[3px] bg-otter-border hover:bg-otter-teal/50 cursor-row-resize transition-colors" @mousedown="onDividerMouseDown($event, 'v')"></div>
              <div class="relative bg-otter-dark overflow-hidden cursor-pointer flex-1" :class="activePaneIndex === 3 ? 'ring-1 ring-otter-teal ring-inset' : ''" @click="focusPane(3)">
                <TerminalView v-if="splitPanes[3]" :session-id="splitPanes[3]" class="w-full h-full" />
                <div v-else class="w-full h-full flex items-center justify-center text-[10px] text-otter-subtle">Select a session</div>
              </div>
            </div>
          </template>
        </div>
      </template>
    </div>

    <ProfileEditor
      v-if="showProfileEditor"
      :profile="editingProfile"
      :prefill="quickConnectData"
      @close="showProfileEditor = false; quickConnectData = undefined"
      @saved="showProfileEditor = false"
      @connected="onConnected"
    />

    <GroupEditor
      v-if="showGroupEditor"
      :group="editingGroup"
      @close="showGroupEditor = false"
      @saved="showGroupEditor = false"
    />

    <SnippetsPanel
      v-if="showSnippets"
      @close="showSnippets = false"
      @run="sendSnippetToTerminal"
    />

    <RecipeRunner
      v-if="showRecipes"
      @close="showRecipes = false"
    />

    <KeyManager
      v-if="showKeyManager"
      @close="showKeyManager = false"
    />

    <ScreenshotPreview
      v-if="screenshotData"
      :data="screenshotData"
      :title="screenshotTitle"
      @close="screenshotData = null"
    />

    <RecordingPreview
      v-if="recordingData"
      :recording="recordingData"
      :title="recordingTitle"
      @close="recordingData = null"
    />

    <VaultDialog
      v-if="showVault"
      :mode="vaultMode"
      @close="showVault = false"
      @unlocked="onVaultUnlocked"
    />

    <!-- Connect search (Algolia-style) -->
    <Teleport to="body">
      <div
        v-if="showConnectDropdown"
        class="fixed inset-0 z-[100] flex items-start justify-center pt-[18vh] bg-black/50 backdrop-blur-sm"
        @click.self="showConnectDropdown = false"
        @keydown.escape="showConnectDropdown = false"
      >
        <div class="w-full max-w-lg rounded-xl bg-otter-card border border-otter-border shadow-2xl overflow-hidden animate-connect-modal">
          <!-- Search input -->
          <div class="flex items-center gap-3 px-4 py-3 border-b border-otter-border">
            <Icon icon="mdi:magnify" class="w-5 h-5 text-otter-teal flex-shrink-0" />
            <input
              ref="connectInputRef"
              v-model="connectSearch"
              class="flex-1 bg-transparent text-sm text-otter-text placeholder-otter-subtle
                     focus:outline-none"
              placeholder="Search servers or type user@host:port..."
              @keyup.enter="handleConnectEnter"
              @keydown.escape="showConnectDropdown = false"
            />
            <kbd class="px-1.5 py-0.5 rounded bg-otter-surface border border-otter-border text-[10px] text-otter-subtle font-mono">ESC</kbd>
          </div>

          <!-- Results -->
          <div class="max-h-72 overflow-y-auto">
            <div v-if="filteredProfiles.length > 0" class="py-1">
              <p class="px-4 pt-2 pb-1 text-[10px] font-semibold text-otter-subtle uppercase tracking-wider">Servers</p>
              <button
                v-for="profile in filteredProfiles"
                :key="profile.id"
                class="w-full flex items-center gap-3 px-4 py-2.5 text-left
                       hover:bg-otter-surface transition-colors group"
                @click="handleConnectSelect(profile)"
              >
                <div class="w-8 h-8 rounded-lg bg-otter-surface flex items-center justify-center flex-shrink-0 group-hover:bg-otter-dark transition-colors">
                  <Icon
                    v-if="profile.detected_os"
                    :icon="getOsIcon({ os_type: '', distro: profile.detected_os })"
                    class="w-4 h-4"
                    :style="{ color: getOsColor({ os_type: '', distro: profile.detected_os }) }"
                  />
                  <LogoIcon v-else :size="16" />
                </div>
                <div class="flex flex-col min-w-0 flex-1">
                  <span class="text-sm text-otter-text truncate">{{ profile.name }}</span>
                  <span class="text-[11px] text-otter-subtle font-mono truncate">{{ profile.username }}@{{ profile.host }}{{ profile.port !== 22 ? ':' + profile.port : '' }}</span>
                </div>
                <Icon icon="mdi:arrow-right" class="w-4 h-4 text-otter-subtle opacity-0 group-hover:opacity-100 transition-opacity" />
              </button>
            </div>

            <!-- Quick connect hint -->
            <div
              v-if="filteredProfiles.length === 0 && connectSearch.trim()"
              class="px-4 py-6 text-center"
            >
              <p class="text-xs text-otter-subtle mb-2">No matching servers</p>
              <button
                class="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-otter-teal/10 text-xs text-otter-teal hover:bg-otter-teal/20 transition-colors"
                @click="handleConnectEnter"
              >
                <Icon icon="mdi:flash" class="w-3.5 h-3.5" />
                Quick connect to "{{ connectSearch }}"
              </button>
            </div>

            <!-- Empty state -->
            <div v-if="filteredProfiles.length === 0 && !connectSearch.trim()" class="px-4 py-6 text-center">
              <p class="text-xs text-otter-subtle">Type to search or enter user@host:port</p>
            </div>
          </div>

          <!-- Footer -->
          <div class="flex items-center justify-between px-4 py-2.5 border-t border-otter-border bg-otter-surface/50">
            <div class="flex items-center gap-3 text-[10px] text-otter-subtle">
              <span class="flex items-center gap-1"><kbd class="px-1 py-0.5 rounded bg-otter-surface border border-otter-border font-mono">↵</kbd> connect</span>
              <span class="flex items-center gap-1"><kbd class="px-1 py-0.5 rounded bg-otter-surface border border-otter-border font-mono">esc</kbd> close</span>
            </div>
            <button
              class="flex items-center gap-1.5 text-[11px] text-otter-teal hover:underline"
              @click="showConnectDropdown = false; openNewConnection()"
            >
              <Icon icon="mdi:plus" class="w-3 h-3" />
              Add New Server
            </button>
          </div>
        </div>
      </div>
    </Teleport>

    <!-- Command palette -->
    <Teleport to="body">
      <div
        v-if="showPalette"
        class="fixed inset-0 z-[100] flex items-start justify-center pt-[15vh] bg-black/40 backdrop-blur-sm"
        @click.self="showPalette = false"
      >
        <div class="w-full max-w-md bg-otter-card border border-otter-border rounded-xl shadow-2xl overflow-hidden">
          <div class="p-3 border-b border-otter-border">
            <input
              ref="paletteInputRef"
              v-model="paletteQuery"
              class="w-full px-3 py-2 rounded-lg bg-otter-surface border border-otter-border
                     text-sm text-otter-text placeholder-otter-subtle
                     focus:outline-none focus:border-otter-teal-dim"
              placeholder="Type a command..."
              @keydown="paletteKeydown"
              @keydown.escape="showPalette = false"
              @input="paletteSelectedIndex = 0"
            />
          </div>
          <div class="max-h-72 overflow-y-auto py-1">
            <template v-for="(item, idx) in filteredPaletteItems" :key="item.id">
              <button
                class="w-full flex items-center gap-3 px-4 py-2 text-left text-sm transition-colors"
                :class="idx === paletteSelectedIndex ? 'bg-otter-teal/10 text-otter-teal' : 'text-otter-text hover:bg-otter-surface'"
                @click="item.action()"
                @mouseenter="paletteSelectedIndex = idx"
              >
                <Icon :icon="item.icon" class="w-4 h-4 flex-shrink-0 text-otter-muted" />
                <div class="flex-1 min-w-0">
                  <span class="truncate block">{{ item.label }}</span>
                  <span v-if="item.sublabel" class="text-[10px] text-otter-subtle truncate block">{{ item.sublabel }}</span>
                </div>
                <span class="text-[10px] text-otter-subtle">{{ item.category }}</span>
              </button>
            </template>
            <p v-if="filteredPaletteItems.length === 0" class="px-4 py-3 text-xs text-otter-subtle text-center">No results</p>
          </div>
        </div>
      </div>
    </Teleport>

    <!-- Confirm close dialog -->
    <Teleport to="body">
      <div
        v-if="confirmCloseSessionId"
        class="fixed inset-0 z-[100] flex items-center justify-center bg-black/50 backdrop-blur-sm"
        @click.self="confirmCloseSessionId = null"
      >
        <div class="bg-otter-card border border-otter-border rounded-xl p-5 w-80 shadow-xl">
          <h3 class="text-sm font-semibold text-otter-text mb-2">Process Still Running</h3>
          <p class="text-xs text-otter-muted mb-4">
            A process is still running on
            <span class="text-otter-text font-mono">{{
              store.sessions[confirmCloseSessionId]?.profile_name
              || `${store.sessions[confirmCloseSessionId]?.username}@${store.sessions[confirmCloseSessionId]?.host}`
            }}</span>.
            Are you sure you want to disconnect?
          </p>
          <div class="flex justify-end gap-2">
            <button
              class="px-3 py-1.5 rounded-lg bg-otter-surface border border-otter-border
                     text-xs text-otter-text hover:border-otter-subtle transition-colors"
              @click="confirmCloseSessionId = null"
            >
              Cancel
            </button>
            <button
              class="px-3 py-1.5 rounded-lg bg-otter-coral text-white font-semibold
                     text-xs hover:opacity-90 transition-opacity"
              @click="confirmClose"
            >
              Disconnect
            </button>
          </div>
        </div>
      </div>
    </Teleport>
  </div>
</template>

<style scoped>
.fade-enter-active,
.fade-leave-active {
  transition: opacity 0.3s ease;
}
.fade-enter-from,
.fade-leave-to {
  opacity: 0;
}

.scrollbar-hide {
  -ms-overflow-style: none;
  scrollbar-width: none;
}
.scrollbar-hide::-webkit-scrollbar {
  display: none;
}

@keyframes heartbeat {
  0%, 100% { opacity: 0; transform: scale(1); }
  50% { opacity: 0.4; transform: scale(2); }
}
.heartbeat {
  animation: heartbeat 3s ease-in-out infinite;
}

@keyframes connect-modal-in {
  from { opacity: 0; transform: scale(0.96) translateY(-8px); }
  to { opacity: 1; transform: scale(1) translateY(0); }
}
.animate-connect-modal {
  animation: connect-modal-in 0.15s ease-out;
}
</style>
