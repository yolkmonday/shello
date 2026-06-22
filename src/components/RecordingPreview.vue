<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, nextTick } from "vue";
import { Icon } from "@iconify/vue";
import { Terminal } from "@xterm/xterm";
import { save } from "@tauri-apps/plugin-dialog";
import { writeFile } from "@tauri-apps/plugin-fs";
import { themes } from "../lib/themes";
import type { TerminalRecording } from "../lib/terminal-recording";
import LogoWordmark from "./LogoWordmark.vue";

const props = defineProps<{
  recording: TerminalRecording;
  title: string;
}>();

const emit = defineEmits<{
  close: [];
}>();

// ── State ────────────────────────────────────────────────────────────
const replayRef = ref<HTMLDivElement>();
const exporting = ref(false);
const exportProgress = ref(0);
const playing = ref(false);
const currentTime = ref(0);
const playbackSpeed = ref(1);

const speeds = [0.5, 1, 2, 4];

// ── Background (same as screenshot, theme-derived) ───────────────────
const backgrounds = computed(() => {
  const theme = themes[props.recording.theme];
  const bg = (theme.background as string) || "#121214";
  return [
    { name: "Theme", value: bg },
    { name: "Gradient", value: `linear-gradient(145deg, ${bg} 0%, ${adjustBrightness(bg, 15)} 50%, ${adjustBrightness(bg, 25)} 100%)` },
    { name: "Warm", value: `linear-gradient(145deg, ${mixColor(bg, "#ff6b6b", 0.1)} 0%, ${mixColor(bg, "#ffa500", 0.08)} 100%)` },
    { name: "Cool", value: `linear-gradient(145deg, ${mixColor(bg, "#4facfe", 0.1)} 0%, ${mixColor(bg, "#00f2fe", 0.08)} 100%)` },
    { name: "None", value: "transparent" },
  ];
});
const selectedBgIndex = ref(1);

const paddings = ["16", "32", "48", "64"] as const;
const selectedPadding = ref<string>("32");

// ── Size presets ─────────────────────────────────────────────────────
const sizePresets = [
  { name: "Auto", icon: "mdi:resize", width: 0, height: 0 },
  { name: "IG Story", icon: "mdi:cellphone", width: 1080, height: 1920 },
  { name: "IG Post", icon: "mdi:image-outline", width: 1080, height: 1080 },
  { name: "X/Twitter", icon: "mdi:twitter", width: 1200, height: 675 },
  { name: "OG Image", icon: "mdi:web", width: 1200, height: 630 },
  { name: "LinkedIn", icon: "mdi:linkedin", width: 1200, height: 627 },
  { name: "Facebook", icon: "mdi:facebook", width: 1200, height: 630 },
  { name: "Threads", icon: "mdi:at", width: 1080, height: 1350 },
  { name: "YouTube", icon: "mdi:youtube", width: 1280, height: 720 },
];
const selectedPreset = ref(0);
const showSizeDropdown = ref(false);

// ── Replay terminal ──────────────────────────────────────────────────
let replayTerminal: Terminal | null = null;
let playTimer: ReturnType<typeof setTimeout> | null = null;
let eventIndex = 0;

const formattedDuration = computed(() => formatTime(props.recording.duration));
const formattedCurrent = computed(() => formatTime(currentTime.value));

function formatTime(ms: number): string {
  const totalSec = Math.floor(ms / 1000);
  const min = Math.floor(totalSec / 60);
  const sec = totalSec % 60;
  return `${min}:${sec.toString().padStart(2, "0")}`;
}

const progressPercent = computed(() =>
  props.recording.duration > 0 ? (currentTime.value / props.recording.duration) * 100 : 0
);

onMounted(async () => {
  await nextTick();
  if (!replayRef.value) return;

  const theme = themes[props.recording.theme];
  replayTerminal = new Terminal({
    theme,
    fontSize: props.recording.fontSize,
    fontFamily: `'${props.recording.fontFamily}', monospace`,
    cols: props.recording.cols,
    rows: props.recording.rows,
    scrollback: 0,
    cursorBlink: false,
    disableStdin: true,
    allowProposedApi: true,
  });
  replayTerminal.open(replayRef.value);

  // Auto-play
  play();
});

onUnmounted(() => {
  stop();
  replayTerminal?.dispose();
});

function play() {
  if (playing.value) return;
  playing.value = true;
  scheduleNext();
}

function pause() {
  playing.value = false;
  if (playTimer) { clearTimeout(playTimer); playTimer = null; }
}

function stop() {
  pause();
  eventIndex = 0;
  currentTime.value = 0;
  replayTerminal?.reset();
}

function restart() {
  stop();
  play();
}

function scheduleNext() {
  if (!playing.value || eventIndex >= props.recording.events.length) {
    playing.value = false;
    return;
  }

  const event = props.recording.events[eventIndex];
  const prevTime = eventIndex > 0 ? props.recording.events[eventIndex - 1].time : 0;
  const delay = Math.max(0, (event.time - prevTime) / playbackSpeed.value);

  // Cap max delay to avoid long pauses
  const cappedDelay = Math.min(delay, 2000 / playbackSpeed.value);

  playTimer = setTimeout(() => {
    if (!playing.value || !replayTerminal) return;
    replayTerminal.write(event.data);
    currentTime.value = event.time;
    eventIndex++;
    scheduleNext();
  }, cappedDelay);
}

function seekTo(percent: number) {
  const wasPlaying = playing.value;
  pause();
  replayTerminal?.reset();
  const targetTime = (percent / 100) * props.recording.duration;

  // Replay all events up to targetTime
  for (let i = 0; i < props.recording.events.length; i++) {
    if (props.recording.events[i].time > targetTime) {
      eventIndex = i;
      break;
    }
    replayTerminal?.write(props.recording.events[i].data);
    if (i === props.recording.events.length - 1) {
      eventIndex = props.recording.events.length;
    }
  }
  currentTime.value = targetTime;
  if (wasPlaying) play();
}

function onSeek(e: MouseEvent) {
  const target = e.currentTarget as HTMLElement;
  const rect = target.getBoundingClientRect();
  const percent = Math.max(0, Math.min(100, ((e.clientX - rect.left) / rect.width) * 100));
  seekTo(percent);
}

// ── Export as WebM ───────────────────────────────────────────────────
async function exportVideo() {
  if (exporting.value || !replayRef.value) return;
  exporting.value = true;
  exportProgress.value = 0;

  try {
    // Create a fresh terminal for export
    const exportContainer = document.createElement("div");
    exportContainer.style.position = "fixed";
    exportContainer.style.left = "-9999px";
    exportContainer.style.top = "0";
    document.body.appendChild(exportContainer);

    const theme = themes[props.recording.theme];
    const exportTerm = new Terminal({
      theme,
      fontSize: props.recording.fontSize,
      fontFamily: `'${props.recording.fontFamily}', monospace`,
      cols: props.recording.cols,
      rows: props.recording.rows,
      scrollback: 0,
      cursorBlink: false,
      disableStdin: true,
      allowProposedApi: true,
    });

    const termDiv = document.createElement("div");
    exportContainer.appendChild(termDiv);
    exportTerm.open(termDiv);

    // Wait for render
    await new Promise(r => setTimeout(r, 200));

    // Get xterm canvas
    const xtermCanvas = termDiv.querySelector("canvas");
    if (!xtermCanvas) { cleanup(); return; }

    const pad = parseInt(selectedPadding.value);
    const titleBarHeight = 40;
    const footerHeight = 28;
    const borderRadius = 12;
    const preset = sizePresets[selectedPreset.value];

    const termW = xtermCanvas.width;
    const termH = xtermCanvas.height;
    const windowW = termW;
    const windowH = titleBarHeight + termH + footerHeight;

    // Use preset size or auto-fit
    const canvasW = preset.width > 0 ? preset.width : windowW + pad * 2;
    const canvasH = preset.height > 0 ? preset.height : windowH + pad * 2;

    const canvas = document.createElement("canvas");
    canvas.width = canvasW;
    canvas.height = canvasH;
    const ctx = canvas.getContext("2d")!;

    const bgStr = backgrounds.value[selectedBgIndex.value]?.value || (theme.background as string);
    const bgColor = (theme.background as string) || "#121214";
    const fgColor = (theme.foreground as string) || "#EDEDF0";

    // Setup MediaRecorder
    const stream = canvas.captureStream(30);
    const recorder = new MediaRecorder(stream, {
      mimeType: getSupportedMimeType(),
      videoBitsPerSecond: 4_000_000,
    });
    const chunks: Blob[] = [];
    recorder.ondataavailable = (e) => { if (e.data.size > 0) chunks.push(e.data); };

    const recorderDone = new Promise<void>(resolve => {
      recorder.onstop = () => resolve();
    });

    recorder.start(100);

    // Helper: draw a frame
    function drawFrame() {
      // Background
      ctx.clearRect(0, 0, canvasW, canvasH);
      if (bgStr.includes("gradient")) {
        const grad = ctx.createLinearGradient(0, 0, canvasW * 0.6, canvasH);
        // Parse gradient colors roughly
        const colors = bgStr.match(/#[0-9a-fA-F]{6}/g) || [bgColor];
        colors.forEach((c, i) => grad.addColorStop(i / Math.max(1, colors.length - 1), c));
        ctx.fillStyle = grad;
      } else if (bgStr === "transparent") {
        ctx.fillStyle = "#00000000";
      } else {
        ctx.fillStyle = bgStr;
      }
      ctx.fillRect(0, 0, canvasW, canvasH);

      // Window with rounded corners (centered for preset sizes)
      const wx = preset.width > 0 ? Math.max(pad, (canvasW - windowW) / 2) : pad;
      const wy = preset.height > 0 ? Math.max(pad, (canvasH - windowH) / 2) : pad;
      roundRect(ctx, wx, wy, windowW, windowH, borderRadius);
      ctx.fillStyle = bgColor;
      ctx.fill();

      // Title bar
      ctx.fillStyle = bgColor;
      ctx.fillRect(wx, wy, windowW, titleBarHeight);
      // Traffic lights
      const dotY = wy + titleBarHeight / 2;
      drawDot(ctx, wx + 16, dotY, 6, "#ff5f57");
      drawDot(ctx, wx + 34, dotY, 6, "#febc2e");
      drawDot(ctx, wx + 52, dotY, 6, "#28c840");
      // Title text
      ctx.fillStyle = fgColor;
      ctx.globalAlpha = 0.5;
      ctx.font = `12px -apple-system, sans-serif`;
      ctx.textAlign = "center";
      ctx.fillText(props.title, wx + windowW / 2, dotY + 4);
      ctx.globalAlpha = 1;
      ctx.textAlign = "start";

      // Separator line
      ctx.strokeStyle = "rgba(255,255,255,0.06)";
      ctx.lineWidth = 1;
      ctx.beginPath();
      ctx.moveTo(wx, wy + titleBarHeight);
      ctx.lineTo(wx + windowW, wy + titleBarHeight);
      ctx.stroke();

      // Terminal canvas
      const xtermCurrent = termDiv.querySelector("canvas");
      if (xtermCurrent) {
        ctx.drawImage(xtermCurrent, wx, wy + titleBarHeight);
      }

      // Footer separator
      ctx.strokeStyle = "rgba(255,255,255,0.04)";
      ctx.beginPath();
      ctx.moveTo(wx, wy + titleBarHeight + termH);
      ctx.lineTo(wx + windowW, wy + titleBarHeight + termH);
      ctx.stroke();

      // Footer bg
      ctx.fillStyle = bgColor;
      ctx.fillRect(wx, wy + titleBarHeight + termH, windowW, footerHeight);

      // Watermark text
      ctx.fillStyle = fgColor;
      ctx.globalAlpha = 0.25;
      ctx.font = "10px -apple-system, sans-serif";
      ctx.textAlign = "right";
      ctx.fillText("shello", wx + windowW - 12, wy + titleBarHeight + termH + 18);
      ctx.globalAlpha = 1;
      ctx.textAlign = "start";
    }

    // Replay and capture frames
    const events = props.recording.events;

    // Draw initial frame
    drawFrame();
    // Hold first frame for 500ms
    await sleep(500);

    for (let i = 0; i < events.length; i++) {
      const prevTime = i > 0 ? events[i - 1].time : 0;
      let delay = events[i].time - prevTime;
      // Cap long pauses
      delay = Math.min(delay, 2000);

      if (delay > 16) await sleep(delay);

      exportTerm.write(events[i].data);
      // Let xterm render
      await sleep(16);
      drawFrame();

      exportProgress.value = Math.round(((i + 1) / events.length) * 100);
    }

    // Hold last frame for 1s
    await sleep(1000);

    recorder.stop();
    await recorderDone;

    const mimeType = getSupportedMimeType();
    const ext = mimeType.includes("mp4") ? "mp4" : "webm";
    const blob = new Blob(chunks, { type: mimeType });

    // Convert blob to bytes
    const arrayBuffer = await blob.arrayBuffer();
    const bytes = new Uint8Array(arrayBuffer);

    // Show save dialog
    const ts = new Date().toISOString().replace(/[:.]/g, "-");
    const sizeSuffix = preset.width > 0 ? `-${preset.width}x${preset.height}` : "";
    const defaultName = `shello-${props.title.replace(/[^a-zA-Z0-9]/g, "-")}${sizeSuffix}-${ts}.${ext}`;

    const filePath = await save({
      title: "Save Recording",
      defaultPath: defaultName,
      filters: [{ name: ext === "mp4" ? "MP4 Video" : "WebM Video", extensions: [ext] }],
    });

    if (!filePath) {
      cleanup();
      return;
    }

    await writeFile(filePath, bytes);

    function cleanup() {
      exportTerm.dispose();
      exportContainer.remove();
      exporting.value = false;
      exportProgress.value = 0;
    }
    cleanup();
  } catch (e) {
    console.error("Export failed:", e);
    exporting.value = false;
  }
}

function getSupportedMimeType(): string {
  const types = [
    "video/webm;codecs=vp9",
    "video/webm;codecs=vp8",
    "video/webm",
    "video/mp4",
  ];
  for (const t of types) {
    if (MediaRecorder.isTypeSupported(t)) return t;
  }
  return "video/webm";
}

function sleep(ms: number): Promise<void> {
  return new Promise(r => setTimeout(r, ms));
}

// ── Canvas helpers ───────────────────────────────────────────────────
function roundRect(ctx: CanvasRenderingContext2D, x: number, y: number, w: number, h: number, r: number) {
  ctx.beginPath();
  ctx.moveTo(x + r, y);
  ctx.lineTo(x + w - r, y);
  ctx.quadraticCurveTo(x + w, y, x + w, y + r);
  ctx.lineTo(x + w, y + h - r);
  ctx.quadraticCurveTo(x + w, y + h, x + w - r, y + h);
  ctx.lineTo(x + r, y + h);
  ctx.quadraticCurveTo(x, y + h, x, y + h - r);
  ctx.lineTo(x, y + r);
  ctx.quadraticCurveTo(x, y, x + r, y);
  ctx.closePath();
}

function drawDot(ctx: CanvasRenderingContext2D, x: number, y: number, r: number, color: string) {
  ctx.beginPath();
  ctx.arc(x, y, r, 0, Math.PI * 2);
  ctx.fillStyle = color;
  ctx.fill();
}

// ── Color utils ──────────────────────────────────────────────────────
function hexToRgb(hex: string): [number, number, number] {
  const h = hex.replace("#", "");
  return [parseInt(h.slice(0, 2), 16), parseInt(h.slice(2, 4), 16), parseInt(h.slice(4, 6), 16)];
}

function rgbToHex(r: number, g: number, b: number): string {
  return "#" + [r, g, b].map(v => Math.min(255, Math.max(0, Math.round(v))).toString(16).padStart(2, "0")).join("");
}

function adjustBrightness(hex: string, amount: number): string {
  const [r, g, b] = hexToRgb(hex);
  const avg = (r + g + b) / 3;
  const dir = avg > 128 ? -1 : 1;
  return rgbToHex(r + dir * amount, g + dir * amount, b + dir * amount);
}

function mixColor(hex1: string, hex2: string, ratio: number): string {
  const [r1, g1, b1] = hexToRgb(hex1);
  const [r2, g2, b2] = hexToRgb(hex2);
  return rgbToHex(r1 + (r2 - r1) * ratio, g1 + (g2 - g1) * ratio, b1 + (b2 - b1) * ratio);
}
</script>

<template>
  <div
    class="fixed inset-0 z-[100] flex items-center justify-center bg-black/60 backdrop-blur-sm"
    @click.self="emit('close')"
    @keydown.escape="emit('close')"
  >
    <div class="flex flex-col w-full max-w-3xl max-h-[90vh] rounded-2xl bg-otter-card border border-otter-border shadow-2xl overflow-hidden animate-rec-modal">

      <!-- Toolbar -->
      <div class="flex items-center gap-2 px-4 py-2.5 border-b border-otter-border">
        <h3 class="text-sm font-semibold text-otter-text mr-2">Recording</h3>

        <!-- BG picker -->
        <div class="flex items-center gap-1.5">
          <span class="text-[10px] text-otter-subtle uppercase tracking-wider">BG</span>
          <button
            v-for="(bg, i) in backgrounds"
            :key="bg.name"
            class="w-5 h-5 rounded-full border-2 transition-all"
            :class="selectedBgIndex === i ? 'border-otter-teal scale-110' : 'border-otter-border hover:border-otter-subtle'"
            :style="{ background: bg.value === 'transparent' ? '#1a1a1d' : bg.value }"
            :title="bg.name"
            @click="selectedBgIndex = i"
          >
            <Icon v-if="bg.value === 'transparent'" icon="mdi:close" class="w-3 h-3 text-otter-subtle mx-auto" />
          </button>
        </div>

        <div class="w-px h-4 bg-otter-border"></div>

        <!-- Padding -->
        <div class="flex items-center gap-1">
          <span class="text-[10px] text-otter-subtle uppercase tracking-wider">PAD</span>
          <button
            v-for="p in paddings"
            :key="p"
            class="px-1.5 py-0.5 rounded text-[10px] font-mono transition-colors"
            :class="selectedPadding === p ? 'bg-otter-teal text-otter-dark' : 'text-otter-subtle hover:text-otter-text'"
            @click="selectedPadding = p"
          >{{ p }}</button>
        </div>

        <div class="w-px h-4 bg-otter-border"></div>

        <!-- Size presets (dropdown) -->
        <div class="relative">
          <button
            class="flex items-center gap-1.5 px-2 py-1 rounded-lg text-[11px]
                   bg-otter-surface border border-otter-border hover:border-otter-subtle transition-colors"
            @click="showSizeDropdown = !showSizeDropdown"
          >
            <Icon :icon="sizePresets[selectedPreset].icon" class="w-3.5 h-3.5 text-otter-teal" />
            <span class="text-otter-text">{{ sizePresets[selectedPreset].name }}</span>
            <span v-if="sizePresets[selectedPreset].width > 0" class="text-otter-subtle">
              {{ sizePresets[selectedPreset].width }}x{{ sizePresets[selectedPreset].height }}
            </span>
            <Icon icon="mdi:chevron-down" class="w-3 h-3 text-otter-subtle" />
          </button>
          <div v-if="showSizeDropdown" class="fixed inset-0 z-10" @click="showSizeDropdown = false"></div>
          <div
            v-if="showSizeDropdown"
            class="absolute top-full left-0 mt-1 w-52 rounded-lg bg-otter-card border border-otter-border shadow-xl z-20 overflow-hidden"
          >
            <div class="py-1">
              <button
                v-for="(preset, i) in sizePresets"
                :key="preset.name"
                class="w-full flex items-center gap-2.5 px-3 py-2 text-left text-xs hover:bg-otter-surface transition-colors"
                :class="selectedPreset === i ? 'text-otter-teal' : 'text-otter-text'"
                @click="selectedPreset = i; showSizeDropdown = false"
              >
                <Icon :icon="preset.icon" class="w-4 h-4 flex-shrink-0" />
                <span class="flex-1">{{ preset.name }}</span>
                <span v-if="preset.width > 0" class="text-[10px] text-otter-subtle font-mono">{{ preset.width }}x{{ preset.height }}</span>
                <Icon v-if="selectedPreset === i" icon="mdi:check" class="w-3.5 h-3.5 text-otter-teal flex-shrink-0" />
              </button>
            </div>
          </div>
        </div>

        <div class="flex-1"></div>

        <!-- Export -->
        <button
          class="flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-semibold
                 bg-otter-teal text-otter-dark hover:opacity-90 transition-opacity
                 disabled:opacity-40"
          :disabled="exporting"
          @click="exportVideo"
        >
          <Icon :icon="exporting ? 'mdi:loading' : 'mdi:download'" class="w-3.5 h-3.5" :class="exporting ? 'animate-spin' : ''" />
          {{ exporting ? `Exporting ${exportProgress}%` : 'Export Video' }}
        </button>
        <button class="text-otter-subtle hover:text-otter-text transition-colors" @click="emit('close')">
          <Icon icon="mdi:close" class="w-5 h-5" />
        </button>
      </div>

      <!-- Preview -->
      <div class="flex-1 overflow-auto p-6 bg-otter-dark flex items-center justify-center">
        <div
          class="inline-block rounded-xl"
          :style="{ background: backgrounds[selectedBgIndex]?.value, padding: selectedPadding + 'px' }"
        >
          <div class="rounded-xl overflow-hidden shadow-2xl" style="min-width: 480px; max-width: 800px;">
            <!-- Title bar -->
            <div
              class="flex items-center gap-3 px-4 py-3"
              :style="{ backgroundColor: themes[recording.theme].background, borderBottom: '1px solid rgba(255,255,255,0.06)' }"
            >
              <div class="flex items-center gap-1.5">
                <span class="w-3 h-3 rounded-full bg-[#ff5f57]"></span>
                <span class="w-3 h-3 rounded-full bg-[#febc2e]"></span>
                <span class="w-3 h-3 rounded-full bg-[#28c840]"></span>
              </div>
              <span
                class="flex-1 text-center text-xs opacity-50"
                :style="{ color: themes[recording.theme].foreground }"
              >{{ title }}</span>
              <div class="w-[52px]"></div>
            </div>

            <!-- Terminal replay -->
            <div
              ref="replayRef"
              class="xterm-replay"
              :style="{ backgroundColor: themes[recording.theme].background }"
            ></div>

            <!-- Footer -->
            <div
              class="flex items-center justify-end px-4 py-2"
              :style="{ backgroundColor: themes[recording.theme].background, borderTop: '1px solid rgba(255,255,255,0.04)' }"
            >
              <div class="flex items-center gap-1.5 opacity-30">
                <LogoWordmark :height="12" />
              </div>
            </div>
          </div>
        </div>
      </div>

      <!-- Playback controls -->
      <div class="px-4 py-3 border-t border-otter-border">
        <!-- Progress bar -->
        <div
          class="w-full h-1.5 rounded-full bg-otter-surface cursor-pointer mb-3 group"
          @click="onSeek"
        >
          <div
            class="h-full rounded-full bg-otter-teal transition-all relative"
            :style="{ width: progressPercent + '%' }"
          >
            <div class="absolute right-0 top-1/2 -translate-y-1/2 w-3 h-3 rounded-full bg-otter-teal
                        opacity-0 group-hover:opacity-100 transition-opacity shadow-md"></div>
          </div>
        </div>

        <div class="flex items-center gap-3">
          <!-- Play/Pause -->
          <button
            class="w-8 h-8 rounded-full bg-otter-surface flex items-center justify-center
                   text-otter-text hover:bg-otter-teal hover:text-otter-dark transition-colors"
            @click="playing ? pause() : play()"
          >
            <Icon :icon="playing ? 'mdi:pause' : 'mdi:play'" class="w-4 h-4" />
          </button>

          <!-- Restart -->
          <button
            class="text-otter-subtle hover:text-otter-text transition-colors"
            title="Restart"
            @click="restart"
          >
            <Icon icon="mdi:restart" class="w-4 h-4" />
          </button>

          <!-- Time -->
          <span class="text-[11px] font-mono text-otter-muted">
            {{ formattedCurrent }} / {{ formattedDuration }}
          </span>

          <div class="flex-1"></div>

          <!-- Speed -->
          <div class="flex items-center gap-1">
            <span class="text-[10px] text-otter-subtle">Speed</span>
            <button
              v-for="s in speeds"
              :key="s"
              class="px-1.5 py-0.5 rounded text-[10px] font-mono transition-colors"
              :class="playbackSpeed === s ? 'bg-otter-teal text-otter-dark' : 'text-otter-subtle hover:text-otter-text'"
              @click="playbackSpeed = s"
            >{{ s }}x</button>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
@keyframes rec-modal-in {
  from { opacity: 0; transform: scale(0.95); }
  to { opacity: 1; transform: scale(1); }
}
.animate-rec-modal {
  animation: rec-modal-in 0.2s ease-out;
}

.xterm-replay :deep(.xterm-viewport) {
  overflow: hidden !important;
}
.xterm-replay :deep(.xterm-scroll-area) {
  overflow: hidden !important;
}
</style>
