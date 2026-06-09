<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { Terminal } from "@xterm/xterm";
import { FitAddon } from "@xterm/addon-fit";
import { WebglAddon } from "@xterm/addon-webgl";
import { WebLinksAddon } from "@xterm/addon-web-links";
import { SearchAddon } from "@xterm/addon-search";
import "@xterm/xterm/css/xterm.css";
import { useTerminalStore } from "../stores/terminal";
import { useSnippetsStore, type Snippet } from "../stores/snippets";
import { Icon } from "@iconify/vue";
import { themes, type ThemeName } from "../lib/themes";
import { serializeTerminalBuffer, type SerializedTerminal } from "../lib/terminal-to-html";
import { TerminalRecorder, type TerminalRecording } from "../lib/terminal-recording";

const props = defineProps<{
  sessionId: string;
}>();

const store = useTerminalStore();
const snippetsStore = useSnippetsStore();
const terminalRef = ref<HTMLDivElement>();
const searchVisible = ref(false);
const searchQuery = ref("");

// ── Shell prompt detection (for running process tracking) ───────────
const PROMPT_RE = /[$#%]\s*$/;
let promptDebounce: ReturnType<typeof setTimeout> | null = null;

function detectPrompt(text: string) {
  if (promptDebounce) clearTimeout(promptDebounce);
  promptDebounce = setTimeout(() => {
    const clean = text.replace(/\x1b\[[0-9;]*[a-zA-Z]/g, "").trimEnd();
    const lastLine = clean.split("\n").pop()?.trim() || "";
    store.setAtPrompt(props.sessionId, PROMPT_RE.test(lastLine));
  }, 100);
}

// ── Password prompt detection ───────────────────────────────────────
const passwordPromptVisible = ref(false);
const passwordPromptLabel = ref("");
let promptHideTimeout: ReturnType<typeof setTimeout> | null = null;
let outputBuffer = "";

const PASSWORD_PATTERNS = [
  /\[sudo\] password for \S+/i,
  /password for \S+:\s*$/i,
  /password:\s*$/i,
  /enter passphrase/i,
  /authentication password/i,
  /login password/i,
  /\S+'s password:\s*$/i,
];

function checkForPasswordPrompt(text: string) {
  outputBuffer += text;
  if (outputBuffer.length > 200) outputBuffer = outputBuffer.slice(-200);

  const clean = outputBuffer.replace(/\x1b\[[0-9;]*[a-zA-Z]/g, "").replace(/\r/g, "");
  for (const pattern of PASSWORD_PATTERNS) {
    const match = clean.match(pattern);
    if (match) {
      passwordPromptLabel.value = match[0].replace(/:\s*$/, "");
      passwordPromptVisible.value = true;
      if (promptHideTimeout) clearTimeout(promptHideTimeout);
      promptHideTimeout = setTimeout(() => { passwordPromptVisible.value = false; }, 15000);
      return;
    }
  }
}

async function autoFillPassword() {
  const session = store.sessions[props.sessionId];
  if (!session?.profile_id) { passwordPromptVisible.value = false; return; }
  try {
    const password = await invoke<string>("profile_get_password", { profileId: session.profile_id });
    if (password) await invoke("ssh_write", { sessionId: props.sessionId, data: password + "\n" });
  } catch { /* vault locked */ }
  passwordPromptVisible.value = false;
  outputBuffer = "";
}

function dismissPrompt() {
  passwordPromptVisible.value = false;
  outputBuffer = "";
  terminal?.focus();
}

// ── Inline snippet suggestions ──────────────────────────────────────
const snippetInput = ref("");
const snippetSelectedIndex = ref(0);
const snippetSuggestionsVisible = ref(false);
const snippetTop = ref(0);
const snippetLeft = ref(0);

function updateSnippetPosition() {
  if (!terminal || !terminalRef.value) return;
  const cursorY = terminal.buffer.active.cursorY;
  const cursorX = terminal.buffer.active.cursorX;
  // Calculate cell dimensions from container
  const rect = terminalRef.value.getBoundingClientRect();
  const padding = 8; // p-2
  const cellHeight = (rect.height - padding * 2) / terminal.rows;
  const cellWidth = (rect.width - padding * 2) / terminal.cols;
  snippetTop.value = padding + (cursorY + 1) * cellHeight;
  snippetLeft.value = padding + cursorX * cellWidth;
}

const filteredSnippets = computed(() => {
  const q = snippetInput.value.toLowerCase();
  if (q.length < 1) return [];
  return snippetsStore.snippets.filter(
    (s) => s.name.toLowerCase().includes(q) || s.command.toLowerCase().includes(q)
  ).slice(0, 6);
});

function showSnippetSuggestions() {
  if (filteredSnippets.value.length > 0) {
    updateSnippetPosition();
    snippetSuggestionsVisible.value = true;
    snippetSelectedIndex.value = 0;
  } else {
    snippetSuggestionsVisible.value = false;
  }
}

function hideSnippetSuggestions() {
  snippetSuggestionsVisible.value = false;
  snippetInput.value = "";
}

function selectSnippet(snippet: Snippet) {
  // Clear what user typed so far, then send the snippet command
  // Send backspaces to erase current input, then send the command
  const bs = "\x7f".repeat(snippetInput.value.length);
  invoke("ssh_write", {
    sessionId: props.sessionId,
    data: bs + snippet.command,
  }).catch(() => {});

  hideSnippetSuggestions();
  terminal?.focus();
}

function handleSnippetKey(e: KeyboardEvent): boolean {
  if (!snippetSuggestionsVisible.value) return true;

  if (e.type !== "keydown") return true;

  if (e.key === "ArrowDown") {
    e.preventDefault();
    snippetSelectedIndex.value = Math.min(
      snippetSelectedIndex.value + 1,
      filteredSnippets.value.length - 1
    );
    return false;
  }

  if (e.key === "ArrowUp") {
    e.preventDefault();
    snippetSelectedIndex.value = Math.max(snippetSelectedIndex.value - 1, 0);
    return false;
  }

  if (e.key === "Tab" || e.key === "Enter") {
    e.preventDefault();
    const selected = filteredSnippets.value[snippetSelectedIndex.value];
    if (selected) selectSnippet(selected);
    return false;
  }

  if (e.key === "Escape") {
    hideSnippetSuggestions();
    return false;
  }

  return true;
}

function trackInput(data: string) {
  // Password prompt is active — don't track snippets
  if (passwordPromptVisible.value) return;

  for (const ch of data) {
    if (ch === "\r" || ch === "\n") {
      // Enter — clear input
      hideSnippetSuggestions();
    } else if (ch === "\x7f" || ch === "\b") {
      // Backspace
      snippetInput.value = snippetInput.value.slice(0, -1);
      showSnippetSuggestions();
    } else if (ch === "\x03" || ch === "\x04") {
      // Ctrl+C / Ctrl+D — clear
      hideSnippetSuggestions();
    } else if (ch === "\x1b") {
      // Escape sequence start — ignore
      hideSnippetSuggestions();
    } else if (ch >= " ") {
      // Printable character
      snippetInput.value += ch;
      showSnippetSuggestions();
    }
  }
}

// ── Screenshot ──────────────────────────────────────────────────────
const screenshotFlash = ref(false);

async function takeScreenshot() {
  if (!terminalRef.value) return;
  const canvas = terminalRef.value.querySelector("canvas");
  if (!canvas) return;

  try {
    const blob = await new Promise<Blob | null>((resolve) =>
      canvas.toBlob(resolve, "image/png")
    );
    if (!blob) return;

    // Copy to clipboard
    await navigator.clipboard.write([
      new ClipboardItem({ "image/png": blob }),
    ]);

    // Also save to file
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    const session = store.sessions[props.sessionId];
    const name = session?.profile_name || `${session?.username}@${session?.host}`;
    const ts = new Date().toISOString().replace(/[:.]/g, "-");
    a.href = url;
    a.download = `shello-${name}-${ts}.png`;
    a.click();
    URL.revokeObjectURL(url);

    // Flash effect
    screenshotFlash.value = true;
    setTimeout(() => { screenshotFlash.value = false; }, 300);
  } catch (e) {
    console.error("Screenshot failed:", e);
  }
}

const isRecording = ref(false);
const isPaused = ref(false);
const recElapsed = ref("");
const recorder = new TerminalRecorder();
let recTimerInterval: ReturnType<typeof setInterval> | null = null;

function formatRecTime(ms: number): string {
  const totalSec = Math.floor(ms / 1000);
  const min = Math.floor(totalSec / 60);
  const sec = totalSec % 60;
  return `${min}:${sec.toString().padStart(2, "0")}`;
}

function startRecElapsedTimer() {
  recTimerInterval = setInterval(() => {
    recElapsed.value = formatRecTime(recorder.elapsed);
  }, 200);
}

function stopRecElapsedTimer() {
  if (recTimerInterval) { clearInterval(recTimerInterval); recTimerInterval = null; }
  recElapsed.value = "";
}

function startRecording() {
  recorder.start();
  isRecording.value = true;
  isPaused.value = false;
  startRecElapsedTimer();
}

function pauseRecording() {
  recorder.pause();
  isPaused.value = true;
}

function resumeRecording() {
  recorder.resume();
  isPaused.value = false;
}

function stopRecording(): TerminalRecording | null {
  if (!terminal) return null;
  isRecording.value = false;
  isPaused.value = false;
  stopRecElapsedTimer();
  return recorder.stop(
    terminal.cols,
    terminal.rows,
    store.theme as ThemeName,
    store.fontSize,
    store.fontFamily,
  );
}

// ── Terminal core ───────────────────────────────────────────────────
let terminal: Terminal | null = null;
let fitAddon: FitAddon | null = null;
let searchAddon: SearchAddon | null = null;
let unlistenData: UnlistenFn | null = null;
let unlistenClosed: UnlistenFn | null = null;
let resizeObserver: ResizeObserver | null = null;
let resizeTimeout: ReturnType<typeof setTimeout> | null = null;

onMounted(async () => {
  if (!terminalRef.value) return;

  // Load snippets if not loaded
  if (snippetsStore.snippets.length === 0) {
    snippetsStore.loadAll().catch(() => {});
  }

  terminal = new Terminal({
    theme: themes[store.theme],
    fontSize: store.fontSize,
    fontFamily: `'${store.fontFamily}', monospace`,
    cursorStyle: store.cursorStyle,
    cursorBlink: store.cursorBlink,
    scrollback: store.scrollback,
    allowProposedApi: true,
  });

  fitAddon = new FitAddon();
  terminal.loadAddon(fitAddon);
  terminal.loadAddon(new WebLinksAddon());
  searchAddon = new SearchAddon();
  terminal.loadAddon(searchAddon);

  terminal.open(terminalRef.value);

  try { terminal.loadAddon(new WebglAddon()); } catch { /* Canvas2D fallback */ }

  fitAddon.fit();
  invoke("ssh_resize", {
    sessionId: props.sessionId,
    cols: terminal.cols,
    rows: terminal.rows,
  }).catch(() => {});

  // Listen for SSH data
  unlistenData = await listen<{ session_id: string; data: number[] }>(
    "ssh_data",
    (event) => {
      if (event.payload.session_id === props.sessionId && terminal) {
        const bytes = new Uint8Array(event.payload.data);
        terminal.write(bytes);

        const text = new TextDecoder().decode(bytes);

        // Capture for recording
        if (recorder.recording) recorder.write(text);
        const session = store.sessions[props.sessionId];
        if (session?.profile_id) {
          checkForPasswordPrompt(text);
        }

        // Detect shell prompt to track running process state
        detectPrompt(text);

        // Mark tab as having unread output
        store.markUnread(props.sessionId);

      }
    }
  );

  // Listen for session closed
  unlistenClosed = await listen<{ session_id: string; reason: string }>(
    "ssh_closed",
    (event) => {
      if (event.payload.session_id === props.sessionId && terminal) {
        terminal.write(`\r\n\x1b[31m[${event.payload.reason}]\x1b[0m\r\n`);
        store.sessions[props.sessionId].status = "disconnected";
      }
    }
  );

  // Initialize as at-prompt
  store.setAtPrompt(props.sessionId, true);

  // Send keystrokes to backend
  terminal.onData((data: string) => {
    // When user presses Enter, a command may be starting
    if (data === "\r") {
      store.setAtPrompt(props.sessionId, false);
    }

    // Password prompt: hide on non-Enter input
    if (passwordPromptVisible.value && data !== "\r") {
      passwordPromptVisible.value = false;
      outputBuffer = "";
    }

    // Track input for snippet suggestions (only if suggestions didn't consume it)
    if (!snippetSuggestionsVisible.value || (data !== "\r" && data !== "\t")) {
      trackInput(data);
    }

    // Don't send to backend if snippet consumed the key
    if (snippetSuggestionsVisible.value && (data === "\r" || data === "\t")) {
      return;
    }

    invoke("ssh_write", {
      sessionId: props.sessionId,
      data,
    }).catch(() => {});
  });

  // Handle terminal focus
  terminal.textarea?.addEventListener("focus", () => {
    store.setActiveSession(props.sessionId);
  });

  // Keyboard shortcuts
  terminal.attachCustomKeyEventHandler((e: KeyboardEvent) => {
    // Snippet navigation — must be checked first
    if (snippetSuggestionsVisible.value) {
      const handled = handleSnippetKey(e);
      if (!handled) return false;
    }

    // Search
    if ((e.metaKey || e.ctrlKey) && e.key === "f" && e.type === "keydown") {
      searchVisible.value = true;
      setTimeout(() => {
        document.getElementById(`search-input-${props.sessionId}`)?.focus();
      }, 50);
      return false;
    }
    if (e.key === "Escape" && searchVisible.value && e.type === "keydown") {
      closeSearch();
      return false;
    }

    // Password auto-fill
    if (e.key === "Enter" && passwordPromptVisible.value && e.type === "keydown") {
      e.preventDefault();
      autoFillPassword();
      return false;
    }

    // Screenshot: Cmd+Shift+S
    if ((e.metaKey || e.ctrlKey) && e.shiftKey && e.key === "S" && e.type === "keydown") {
      takeScreenshot();
      return false;
    }

    return true;
  });

  // Resize observer
  resizeObserver = new ResizeObserver(() => {
    if (resizeTimeout) clearTimeout(resizeTimeout);
    resizeTimeout = setTimeout(() => {
      if (!fitAddon || !terminal) return;
      fitAddon.fit();
      invoke("ssh_resize", {
        sessionId: props.sessionId,
        cols: terminal.cols,
        rows: terminal.rows,
      }).catch(() => {});
    }, 100);
  });
  resizeObserver.observe(terminalRef.value);
});

function findNext() {
  if (searchAddon && searchQuery.value) searchAddon.findNext(searchQuery.value, { caseSensitive: false });
}
function findPrevious() {
  if (searchAddon && searchQuery.value) searchAddon.findPrevious(searchQuery.value, { caseSensitive: false });
}
function closeSearch() {
  searchVisible.value = false;
  searchQuery.value = "";
  searchAddon?.clearDecorations();
  terminal?.focus();
}

onUnmounted(() => {
  stopRecElapsedTimer();
  if (resizeTimeout) clearTimeout(resizeTimeout);
  if (promptHideTimeout) clearTimeout(promptHideTimeout);
  resizeObserver?.disconnect();
  unlistenData?.();
  unlistenClosed?.();
  terminal?.dispose();
});

const sessionInfo = computed(() => store.sessions[props.sessionId]);

const connectionDuration = ref("");
let durationInterval: ReturnType<typeof setInterval> | null = null;

function updateDuration() {
  const s = store.sessions[props.sessionId];
  if (!s?.connected_at) { connectionDuration.value = ""; return; }
  const diff = Math.floor((Date.now() - new Date(s.connected_at).getTime()) / 1000);
  const h = Math.floor(diff / 3600);
  const m = Math.floor((diff % 3600) / 60);
  const sec = diff % 60;
  connectionDuration.value = h > 0
    ? `${h}:${String(m).padStart(2, "0")}:${String(sec).padStart(2, "0")}`
    : `${m}:${String(sec).padStart(2, "0")}`;
}

onMounted(() => {
  updateDuration();
  durationInterval = setInterval(updateDuration, 1000);
});
onUnmounted(() => {
  if (durationInterval) clearInterval(durationInterval);
});

watch(() => store.activeSessionId, (newId) => {
  if (newId === props.sessionId && terminal && fitAddon) {
    requestAnimationFrame(() => { fitAddon!.fit(); });
  }
});
function getScreenshotData(): SerializedTerminal | null {
  if (!terminal) return null;
  return serializeTerminalBuffer(terminal, themes[store.theme]);
}

defineExpose({ getScreenshotData, startRecording, stopRecording, pauseRecording, resumeRecording, isRecording, isPaused });

watch(() => store.theme, (t) => { if (terminal) terminal.options.theme = themes[t]; });
watch(() => store.fontSize, (s) => { if (terminal && fitAddon) { terminal.options.fontSize = s; fitAddon.fit(); } });
watch(() => store.fontFamily, (f) => { if (terminal && fitAddon) { terminal.options.fontFamily = `'${f}', monospace`; fitAddon.fit(); } });
watch(() => store.cursorStyle, (s) => { if (terminal) terminal.options.cursorStyle = s; });
watch(() => store.cursorBlink, (b) => { if (terminal) terminal.options.cursorBlink = b; });
watch(() => store.scrollback, (s) => { if (terminal) terminal.options.scrollback = s; });
watch(() => store.bellStyle, (b) => { if (terminal) /* bellStyle removed in xterm v6 */ void b; });
</script>

<template>
  <div class="relative w-full h-full flex flex-col" :style="{ backgroundColor: themes[store.theme].background, opacity: store.terminalOpacity / 100 }">
    <!-- Screenshot flash -->
    <div
      v-if="screenshotFlash"
      class="absolute inset-0 z-20 bg-white/20 pointer-events-none transition-opacity duration-300"
    ></div>

    <!-- Recording indicator -->
    <div
      v-if="isRecording"
      class="absolute top-2 right-2 z-10 flex items-center gap-1.5 px-2 py-1
             bg-otter-coral/90 rounded-lg text-white text-[10px] font-semibold"
      :class="isPaused ? '' : 'animate-pulse'"
    >
      <span class="w-2 h-2 rounded-full" :class="isPaused ? 'bg-white/50' : 'bg-white'"></span>
      <span>{{ isPaused ? 'PAUSED' : 'REC' }}</span>
      <span class="font-mono opacity-80">{{ recElapsed || '0:00' }}</span>
      <button
        class="ml-1 w-4 h-4 flex items-center justify-center rounded hover:bg-white/20 transition-colors"
        :title="isPaused ? 'Resume' : 'Pause'"
        @click.stop="isPaused ? $emit('resumeRecord') : $emit('pauseRecord')"
      >
        <Icon :icon="isPaused ? 'mdi:play' : 'mdi:pause'" class="w-3 h-3" />
      </button>
    </div>

    <!-- Search bar -->
    <Transition name="search">
      <div
        v-if="searchVisible"
        class="absolute top-2 right-2 z-10 flex items-center gap-1.5 px-2 py-1.5
               bg-otter-card border border-otter-border rounded-lg shadow-lg"
        @click.stop
      >
        <input
          :id="`search-input-${sessionId}`"
          v-model="searchQuery"
          class="w-48 px-2 py-1 rounded bg-otter-surface border border-otter-border
                 text-otter-text placeholder-otter-subtle text-xs
                 focus:outline-none focus:border-otter-teal-dim"
          placeholder="Search..."
          @keydown.enter.exact="findNext"
          @keydown.enter.shift="findPrevious"
          @keydown.escape="closeSearch"
          @input="findNext"
        />
        <button class="p-1 text-otter-muted hover:text-otter-text transition-colors" title="Previous (Shift+Enter)" @click="findPrevious">
          <svg class="w-3.5 h-3.5" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="2"><path d="M4 10l4-4 4 4"/></svg>
        </button>
        <button class="p-1 text-otter-muted hover:text-otter-text transition-colors" title="Next (Enter)" @click="findNext">
          <svg class="w-3.5 h-3.5" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="2"><path d="M4 6l4 4 4-4"/></svg>
        </button>
        <button class="p-1 text-otter-muted hover:text-otter-text transition-colors" title="Close (Esc)" @click="closeSearch">
          <svg class="w-3.5 h-3.5" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="2"><path d="M4 4l8 8M12 4l-8 8"/></svg>
        </button>
      </div>
    </Transition>

    <!-- Inline snippet suggestions (positioned at cursor) -->
    <Transition name="snippet">
      <div
        v-if="snippetSuggestionsVisible && filteredSnippets.length > 0"
        class="absolute z-10 rounded-lg bg-otter-dark/95 border border-otter-border/50
               backdrop-blur-sm py-0.5 shadow-lg"
        :style="{ top: snippetTop + 'px', left: snippetLeft + 'px' }"
        @click.stop
      >
        <div
          v-for="(snippet, idx) in filteredSnippets"
          :key="snippet.id"
          class="flex items-center gap-2 px-2.5 py-1 cursor-pointer font-mono transition-colors"
          :class="idx === snippetSelectedIndex
            ? 'bg-otter-teal/15 text-otter-teal'
            : 'text-otter-subtle hover:text-otter-muted'"
          :style="{ fontSize: (store.fontSize - 1) + 'px' }"
          @click="selectSnippet(snippet)"
          @mouseenter="snippetSelectedIndex = idx"
        >
          <span class="opacity-50 w-3 text-center">{{ idx === snippetSelectedIndex ? '›' : ' ' }}</span>
          <span class="truncate max-w-[250px]">{{ snippet.command }}</span>
          <span class="opacity-30 truncate max-w-[80px] ml-1">{{ snippet.name }}</span>
        </div>
      </div>
    </Transition>

    <!-- Password prompt suggestion -->
    <Transition name="prompt">
      <div
        v-if="passwordPromptVisible"
        class="absolute bottom-4 left-1/2 -translate-x-1/2 z-10 flex items-center gap-3
               px-4 py-3 bg-otter-card border-2 border-otter-teal/60 rounded-2xl
               shadow-[0_0_20px_rgba(100,220,200,0.15)] prompt-glow"
        @click.stop
      >
        <div class="w-8 h-8 rounded-full bg-otter-teal/15 flex items-center justify-center flex-shrink-0 animate-pulse">
          <svg class="w-4 h-4 text-otter-teal" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5">
            <rect x="3" y="7" width="10" height="7" rx="1.5"/>
            <path d="M5 7V5a3 3 0 016 0v2"/>
          </svg>
        </div>
        <div class="flex flex-col min-w-0">
          <span class="text-[11px] text-otter-muted truncate">Password required</span>
          <span class="text-xs text-otter-text font-medium truncate max-w-[200px]">{{ passwordPromptLabel }}</span>
        </div>
        <button
          class="flex items-center gap-1.5 px-4 py-2 rounded-xl bg-otter-teal text-otter-dark text-xs font-bold
                 hover:opacity-90 transition-opacity whitespace-nowrap"
          @click="autoFillPassword"
        >
          Auto-fill
          <kbd class="px-1.5 py-0.5 rounded bg-otter-dark/20 text-[10px] font-mono">↵</kbd>
        </button>
        <button class="p-1 text-otter-subtle hover:text-otter-text transition-colors" @click="dismissPrompt">
          <svg class="w-3.5 h-3.5" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="2"><path d="M4 4l8 8M12 4l-8 8"/></svg>
        </button>
      </div>
    </Transition>

    <div
      ref="terminalRef"
      class="w-full flex-1 p-2 min-h-0"
      :class="{
        'ring-1 ring-otter-teal ring-opacity-50':
          store.activeSessionId === sessionId,
      }"
    ></div>

    <!-- Status bar -->
    <div
      v-if="sessionInfo"
      class="flex items-center gap-3 px-3 h-6 flex-shrink-0 border-t text-[10px] font-mono"
      :style="{ borderColor: themes[store.theme].brightBlack, color: themes[store.theme].brightBlack }"
    >
      <span class="flex items-center gap-1">
        <span
          class="w-1.5 h-1.5 rounded-full"
          :class="sessionInfo.status === 'connected' ? 'bg-emerald-400' : 'bg-otter-coral'"
        ></span>
        {{ sessionInfo.status === 'connected' ? 'SSH' : 'CLOSED' }}
      </span>
      <span>{{ sessionInfo.username }}@{{ sessionInfo.host }}:{{ sessionInfo.port }}</span>
      <span v-if="connectionDuration">{{ connectionDuration }}</span>
      <span class="ml-auto flex items-center gap-2">
        <button
          class="flex items-center gap-1 px-1.5 py-0.5 rounded hover:bg-white/10 transition-colors"
          :style="{ color: themes[store.theme].brightBlack }"
          @click="$emit('screenshot')"
        >
          <Icon icon="mdi:camera-outline" class="w-3 h-3" />
          <span>Screenshot</span>
        </button>
        <button
          class="flex items-center gap-1 px-1.5 py-0.5 rounded transition-colors"
          :class="isRecording ? 'animate-pulse' : ''"
          :style="{ color: isRecording ? '#f87171' : themes[store.theme].brightBlack }"
          @click="$emit('toggleRecord')"
        >
          <Icon :icon="isRecording ? 'mdi:stop-circle' : 'mdi:record-circle-outline'" class="w-3 h-3" />
          <span>{{ isRecording ? 'Stop' : 'Record' }}</span>
        </button>
        <span :style="{ color: themes[store.theme].brightBlack }">UTF-8</span>
      </span>
    </div>
  </div>
</template>

<style scoped>
.search-enter-active,
.search-leave-active {
  transition: opacity 0.15s ease, transform 0.15s ease;
}
.search-enter-from,
.search-leave-to {
  opacity: 0;
  transform: translateY(-8px);
}

.snippet-enter-active {
  transition: opacity 0.15s ease, transform 0.15s ease;
}
.snippet-leave-active {
  transition: opacity 0.1s ease, transform 0.1s ease;
}
.snippet-enter-from,
.snippet-leave-to {
  opacity: 0;
  transform: translateY(8px);
}

.prompt-enter-active {
  transition: opacity 0.25s ease, transform 0.25s cubic-bezier(0.34, 1.56, 0.64, 1);
}
.prompt-leave-active {
  transition: opacity 0.15s ease, transform 0.15s ease;
}
.prompt-enter-from {
  opacity: 0;
  transform: translate(-50%, 16px) scale(0.95);
}
.prompt-leave-to {
  opacity: 0;
  transform: translate(-50%, 8px);
}

.prompt-glow {
  animation: glow-pulse 2s ease-in-out infinite;
}
@keyframes glow-pulse {
  0%, 100% { box-shadow: 0 0 15px rgba(100, 220, 200, 0.1); }
  50% { box-shadow: 0 0 25px rgba(100, 220, 200, 0.25); }
}
</style>
