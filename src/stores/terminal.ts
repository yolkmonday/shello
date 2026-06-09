import { defineStore } from "pinia";
import { ref, watch } from "vue";
import type { ThemeName } from "../lib/themes";

export interface SessionInfo {
  id: string;
  host: string;
  port: number;
  username: string;
  connected_at: string;
  status: "connected" | "disconnected";
  profile_id?: string;
  profile_name?: string;
  os_info?: { os_type: string; distro: string } | null;
}

export type PaneNode =
  | { type: "terminal"; sessionId: string }
  | {
      type: "split";
      direction: "vertical" | "horizontal";
      children: [PaneNode, PaneNode];
      ratio: number;
    };

export const useTerminalStore = defineStore("terminal", () => {
  // State
  const sessions = ref<Record<string, SessionInfo>>({});
  const paneTree = ref<PaneNode | null>(null);
  const activeSessionId = ref<string | null>(null);
  const theme = ref<ThemeName>(
    (localStorage.getItem("shello-theme") as ThemeName) || "shello-dark"
  );
  const fontSize = ref<number>(
    Number(localStorage.getItem("shello-font-size")) || 14
  );
  const fontFamily = ref<string>(
    localStorage.getItem("shello-font-family") || "Fira Code"
  );
  const scrollback = ref<number>(
    Number(localStorage.getItem("shello-scrollback")) || 5000
  );
  const terminalOpacity = ref<number>(
    Number(localStorage.getItem("shello-opacity")) || 100
  );
  const bellStyle = ref<"none" | "sound" | "visual">(
    (localStorage.getItem("shello-bell") as "none" | "sound" | "visual") || "visual"
  );
  const cursorStyle = ref<"block" | "underline" | "bar">(
    (localStorage.getItem("shello-cursor-style") as "block" | "underline" | "bar") || "block"
  );
  const cursorBlink = ref<boolean>(
    localStorage.getItem("shello-cursor-blink") !== "false"
  );
  const connectTimeout = ref<number>(
    Number(localStorage.getItem("shello-connect-timeout")) || 10
  );
  const appMode = ref<"dark" | "light">(
    (localStorage.getItem("shello-app-mode") as "dark" | "light") || "dark"
  );

  // Persist preferences & sync UI theme
  function applyAppMode(mode: "dark" | "light") {
    document.documentElement.setAttribute("data-theme", mode);
  }
  applyAppMode(appMode.value);
  watch(theme, (v) => localStorage.setItem("shello-theme", v));
  watch(fontSize, (v) => localStorage.setItem("shello-font-size", String(v)));
  watch(fontFamily, (v) => localStorage.setItem("shello-font-family", v));
  watch(cursorStyle, (v) => localStorage.setItem("shello-cursor-style", v));
  watch(cursorBlink, (v) => localStorage.setItem("shello-cursor-blink", String(v)));
  watch(scrollback, (v) => localStorage.setItem("shello-scrollback", String(v)));
  watch(terminalOpacity, (v) => localStorage.setItem("shello-opacity", String(v)));
  watch(bellStyle, (v) => localStorage.setItem("shello-bell", v));
  watch(connectTimeout, (v) => localStorage.setItem("shello-connect-timeout", String(v)));
  watch(appMode, (v) => {
    localStorage.setItem("shello-app-mode", v);
    applyAppMode(v);
  });

  // Actions
  function addSession(id: string, info: SessionInfo) {
    sessions.value[id] = info;

    const newPane: PaneNode = { type: "terminal", sessionId: id };
    if (!paneTree.value) {
      paneTree.value = newPane;
    }
    // If adding via split, the caller uses splitPane() instead

    activeSessionId.value = id;
  }

  function removeSession(id: string) {
    delete sessions.value[id];
    closePane(id);

    if (activeSessionId.value === id) {
      const remaining = Object.keys(sessions.value);
      activeSessionId.value = remaining.length > 0 ? remaining[0] : null;
    }
  }

  function setActiveSession(id: string) {
    activeSessionId.value = id;
  }

  function splitPane(
    targetSessionId: string,
    direction: "vertical" | "horizontal",
    newSessionId: string
  ) {
    if (!paneTree.value) return;
    paneTree.value = splitNode(
      paneTree.value,
      targetSessionId,
      direction,
      newSessionId
    );
  }

  function closePane(sessionId: string) {
    if (!paneTree.value) return;

    // If the root is the terminal being closed
    if (
      paneTree.value.type === "terminal" &&
      paneTree.value.sessionId === sessionId
    ) {
      paneTree.value = null;
      return;
    }

    paneTree.value = removeNode(paneTree.value, sessionId);
  }

  function updateSessionOs(id: string, os_info: { os_type: string; distro: string }) {
    if (sessions.value[id]) {
      sessions.value[id].os_info = os_info;
    }
  }

  function replaceSession(oldId: string, newId: string, info: SessionInfo) {
    // Insert new session at the same position as old one
    const entries = Object.entries(sessions.value);
    const idx = entries.findIndex(([id]) => id === oldId);
    if (idx >= 0) {
      entries[idx] = [newId, info];
      sessions.value = Object.fromEntries(entries);
    } else {
      sessions.value[newId] = info;
    }

    if (activeSessionId.value === oldId) {
      activeSessionId.value = newId;
    }
  }

  // Track whether each session is at a shell prompt (no process running)
  const atPrompt = ref<Record<string, boolean>>({});

  // Pinned tabs
  const pinnedSessions = ref<Set<string>>(new Set());

  function togglePin(sessionId: string) {
    if (pinnedSessions.value.has(sessionId)) {
      pinnedSessions.value.delete(sessionId);
    } else {
      pinnedSessions.value.add(sessionId);
    }
    pinnedSessions.value = new Set(pinnedSessions.value);
  }

  // Track unread output on background tabs
  const hasUnread = ref<Record<string, boolean>>({});

  function markUnread(sessionId: string) {
    if (activeSessionId.value !== sessionId) {
      hasUnread.value[sessionId] = true;
    }
  }

  function clearUnread(sessionId: string) {
    hasUnread.value[sessionId] = false;
  }

  function setAtPrompt(sessionId: string, value: boolean) {
    atPrompt.value[sessionId] = value;
  }

  function reorderSession(fromId: string, toId: string) {
    const entries = Object.entries(sessions.value);
    const fromIdx = entries.findIndex(([id]) => id === fromId);
    const toIdx = entries.findIndex(([id]) => id === toId);
    if (fromIdx < 0 || toIdx < 0) return;
    const [moved] = entries.splice(fromIdx, 1);
    entries.splice(toIdx, 0, moved);
    sessions.value = Object.fromEntries(entries);
  }

  function setTheme(t: ThemeName) {
    theme.value = t;
  }

  function setFontSize(size: number) {
    fontSize.value = Math.max(10, Math.min(24, size));
  }

  function setFontFamily(font: string) {
    fontFamily.value = font;
  }

  function setConnectTimeout(secs: number) {
    connectTimeout.value = Math.max(5, Math.min(60, secs));
  }

  function setAppMode(mode: "dark" | "light") {
    appMode.value = mode;
  }

  function toggleAppMode() {
    appMode.value = appMode.value === "dark" ? "light" : "dark";
  }

  return {
    sessions,
    paneTree,
    activeSessionId,
    theme,
    fontSize,
    fontFamily,
    cursorStyle,
    cursorBlink,
    scrollback,
    terminalOpacity,
    bellStyle,
    connectTimeout,
    appMode,
    atPrompt,
    pinnedSessions,
    togglePin,
    hasUnread,
    markUnread,
    clearUnread,
    addSession,
    removeSession,
    setActiveSession,
    splitPane,
    closePane,
    updateSessionOs,
    setTheme,
    setFontSize,
    setFontFamily,
    setConnectTimeout,
    setAtPrompt,
    replaceSession,
    reorderSession,
    setAppMode,
    toggleAppMode,
  };
});

// ── Tree helpers ──────────────────────────────────────────────────────

function splitNode(
  node: PaneNode,
  targetSessionId: string,
  direction: "vertical" | "horizontal",
  newSessionId: string
): PaneNode {
  if (node.type === "terminal") {
    if (node.sessionId === targetSessionId) {
      return {
        type: "split",
        direction,
        children: [node, { type: "terminal", sessionId: newSessionId }],
        ratio: 0.5,
      };
    }
    return node;
  }

  return {
    ...node,
    children: [
      splitNode(node.children[0], targetSessionId, direction, newSessionId),
      splitNode(node.children[1], targetSessionId, direction, newSessionId),
    ],
  };
}

function removeNode(node: PaneNode, sessionId: string): PaneNode | null {
  if (node.type === "terminal") {
    return node.sessionId === sessionId ? null : node;
  }

  const left = removeNode(node.children[0], sessionId);
  const right = removeNode(node.children[1], sessionId);

  if (!left && !right) return null;
  if (!left) return right;
  if (!right) return left;

  return { ...node, children: [left, right] };
}
