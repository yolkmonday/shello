<script setup lang="ts">
import { ref, computed, onMounted, nextTick } from "vue";
import { toPng } from "html-to-image";
import { Icon } from "@iconify/vue";
import type { SerializedTerminal } from "../lib/terminal-to-html";
import LogoWordmark from "./LogoWordmark.vue";

const props = defineProps<{
  data: SerializedTerminal;
  title: string;
}>();

const emit = defineEmits<{
  close: [];
}>();

const frameRef = ref<HTMLDivElement>();
const exporting = ref(false);
const copied = ref(false);
const showSizeDropdown = ref(false);

// ── Padding ──────────────────────────────────────────────────────────
const paddings = ["16", "32", "48", "64"] as const;
const selectedPadding = ref<string>("48");

// ── Backgrounds (derived from terminal theme colors) ─────────────────
const backgrounds = computed(() => {
  const bg = props.data.background;
  const fg = props.data.foreground;
  return [
    { name: "Theme", value: bg, preview: bg },
    { name: "Gradient", value: `linear-gradient(145deg, ${bg} 0%, ${adjustBrightness(bg, 15)} 50%, ${adjustBrightness(bg, 25)} 100%)`, preview: `linear-gradient(145deg, ${bg}, ${adjustBrightness(bg, 25)})` },
    { name: "Accent", value: `linear-gradient(145deg, ${adjustBrightness(bg, 5)} 0%, ${mixColor(bg, props.data.foreground, 0.08)} 100%)`, preview: `linear-gradient(145deg, ${bg}, ${mixColor(bg, fg, 0.08)})` },
    { name: "Warm", value: `linear-gradient(145deg, ${mixColor(bg, "#ff6b6b", 0.1)} 0%, ${mixColor(bg, "#ffa500", 0.08)} 100%)`, preview: `linear-gradient(145deg, ${mixColor(bg, "#ff6b6b", 0.1)}, ${mixColor(bg, "#ffa500", 0.08)})` },
    { name: "Cool", value: `linear-gradient(145deg, ${mixColor(bg, "#4facfe", 0.1)} 0%, ${mixColor(bg, "#00f2fe", 0.08)} 100%)`, preview: `linear-gradient(145deg, ${mixColor(bg, "#4facfe", 0.1)}, ${mixColor(bg, "#00f2fe", 0.08)})` },
    { name: "None", value: "transparent", preview: "transparent" },
  ];
});
const selectedBgIndex = ref(1); // default to Gradient
const selectedBg = computed(() => backgrounds.value[selectedBgIndex.value]?.value || backgrounds.value[0].value);

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
const selectedPreset = ref(0); // Auto

const frameStyle = computed(() => {
  const preset = sizePresets[selectedPreset.value];
  if (!preset || preset.width === 0) {
    return { background: selectedBg.value, padding: selectedPadding.value + "px" };
  }
  // Fixed size: scale down for preview, export at full size
  const w = preset.width / 2;
  const h = preset.height / 2;
  return {
    background: selectedBg.value,
    width: w + "px",
    height: h + "px",
    display: "flex",
    alignItems: "center",
    justifyContent: "center",
    padding: selectedPadding.value + "px",
    boxSizing: "border-box" as const,
  };
});

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
  return rgbToHex(
    r1 + (r2 - r1) * ratio,
    g1 + (g2 - g1) * ratio,
    b1 + (b2 - b1) * ratio,
  );
}

// ── Export ────────────────────────────────────────────────────────────
async function exportPng() {
  if (!frameRef.value || exporting.value) return;
  exporting.value = true;
  try {
    const preset = sizePresets[selectedPreset.value];
    const opts: any = { cacheBust: true, pixelRatio: 2 };
    if (preset && preset.width > 0) {
      // For fixed sizes, render at actual dimensions
      opts.width = preset.width;
      opts.height = preset.height;
      opts.pixelRatio = 1;
      opts.style = {
        width: preset.width + "px",
        height: preset.height + "px",
      };
    }

    const dataUrl = await toPng(frameRef.value, opts);
    const a = document.createElement("a");
    const ts = new Date().toISOString().replace(/[:.]/g, "-");
    const sizeSuffix = preset && preset.width > 0 ? `-${preset.width}x${preset.height}` : "";
    a.href = dataUrl;
    a.download = `shello-${props.title.replace(/[^a-zA-Z0-9]/g, "-")}${sizeSuffix}-${ts}.png`;
    a.click();
  } catch (e) {
    console.error("Export failed:", e);
  } finally {
    exporting.value = false;
  }
}

async function copyToClipboard() {
  if (!frameRef.value) return;
  try {
    const preset = sizePresets[selectedPreset.value];
    const opts: any = { cacheBust: true, pixelRatio: 2 };
    if (preset && preset.width > 0) {
      opts.width = preset.width;
      opts.height = preset.height;
      opts.pixelRatio = 1;
      opts.style = { width: preset.width + "px", height: preset.height + "px" };
    }
    const dataUrl = await toPng(frameRef.value, opts);
    const res = await fetch(dataUrl);
    const blob = await res.blob();
    await navigator.clipboard.write([new ClipboardItem({ "image/png": blob })]);
    copied.value = true;
    setTimeout(() => { copied.value = false; }, 2000);
  } catch (e) {
    console.error("Copy failed:", e);
  }
}

onMounted(() => {
  nextTick(() => frameRef.value?.scrollIntoView({ behavior: "smooth", block: "center" }));
});
</script>

<template>
  <div
    class="fixed inset-0 z-[100] flex items-center justify-center bg-black/60 backdrop-blur-sm"
    @click.self="emit('close')"
    @keydown.escape="emit('close')"
  >
    <div class="flex flex-col max-w-[95vw] max-h-[92vh] rounded-2xl bg-otter-card border border-otter-border shadow-2xl overflow-hidden animate-ss-modal">
      <!-- Toolbar -->
      <div class="flex items-center gap-2 px-4 py-2.5 border-b border-otter-border flex-wrap">
        <h3 class="text-sm font-semibold text-otter-text mr-2">Screenshot</h3>

        <!-- Background picker -->
        <div class="flex items-center gap-1.5">
          <span class="text-[10px] text-otter-subtle uppercase tracking-wider">BG</span>
          <button
            v-for="(bg, i) in backgrounds"
            :key="bg.name"
            class="w-5 h-5 rounded-full border-2 transition-all"
            :class="selectedBgIndex === i ? 'border-otter-teal scale-110' : 'border-otter-border hover:border-otter-subtle'"
            :style="{ background: bg.preview === 'transparent' ? '#1a1a1d' : bg.preview }"
            :title="bg.name"
            @click="selectedBgIndex = i"
          >
            <Icon v-if="bg.preview === 'transparent'" icon="mdi:close" class="w-3 h-3 text-otter-subtle mx-auto" />
          </button>
        </div>

        <div class="w-px h-4 bg-otter-border"></div>

        <!-- Padding picker -->
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

        <!-- Actions -->
        <button
          class="flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-medium
                 bg-otter-surface border border-otter-border text-otter-text
                 hover:border-otter-subtle transition-colors"
          @click="copyToClipboard"
        >
          <Icon :icon="copied ? 'mdi:check' : 'mdi:content-copy'" class="w-3.5 h-3.5" />
          {{ copied ? 'Copied!' : 'Copy' }}
        </button>
        <button
          class="flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-semibold
                 bg-otter-teal text-otter-dark hover:opacity-90 transition-opacity
                 disabled:opacity-40"
          :disabled="exporting"
          @click="exportPng"
        >
          <Icon :icon="exporting ? 'mdi:loading' : 'mdi:download'" class="w-3.5 h-3.5" :class="exporting ? 'animate-spin' : ''" />
          Export PNG
        </button>
        <button class="text-otter-subtle hover:text-otter-text transition-colors" @click="emit('close')">
          <Icon icon="mdi:close" class="w-5 h-5" />
        </button>
      </div>

      <!-- Preview area -->
      <div class="flex-1 overflow-auto p-6 bg-otter-dark flex items-center justify-center">
        <!-- Exportable frame -->
        <div
          ref="frameRef"
          class="inline-flex items-center justify-center rounded-xl"
          :style="frameStyle"
        >
          <!-- Window chrome -->
          <div
            class="rounded-xl overflow-hidden shadow-2xl w-full"
            :style="{ maxWidth: sizePresets[selectedPreset].width > 0 ? '100%' : '900px', minWidth: sizePresets[selectedPreset].width > 0 ? undefined : '480px' }"
          >
            <!-- Title bar -->
            <div
              class="flex items-center gap-3 px-4 py-3"
              :style="{ backgroundColor: data.background, borderBottom: '1px solid rgba(255,255,255,0.06)' }"
            >
              <div class="flex items-center gap-1.5">
                <span class="w-3 h-3 rounded-full bg-[#ff5f57]"></span>
                <span class="w-3 h-3 rounded-full bg-[#febc2e]"></span>
                <span class="w-3 h-3 rounded-full bg-[#28c840]"></span>
              </div>
              <span
                class="flex-1 text-center text-xs opacity-50"
                :style="{ color: data.foreground }"
              >{{ title }}</span>
              <div class="w-[52px]"></div>
            </div>

            <!-- Terminal content -->
            <div
              class="overflow-hidden"
              :style="{
                backgroundColor: data.background,
                color: data.foreground,
                fontFamily: data.fontFamily + ', monospace',
                fontSize: (sizePresets[selectedPreset].width > 0 ? Math.max(10, data.fontSize - 2) : data.fontSize) + 'px',
                lineHeight: '1.4',
                padding: '16px 20px 20px',
              }"
            >
              <pre
                class="m-0 whitespace-pre"
                style="tab-size: 8"
                v-html="data.html"
              ></pre>
            </div>

            <!-- Footer watermark -->
            <div
              class="flex items-center justify-end px-4 py-2"
              :style="{ backgroundColor: data.background, borderTop: '1px solid rgba(255,255,255,0.04)' }"
            >
              <div class="flex items-center gap-1.5 opacity-30">
                <LogoWordmark :height="12" />
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
@keyframes ss-modal-in {
  from { opacity: 0; transform: scale(0.95); }
  to { opacity: 1; transform: scale(1); }
}
.animate-ss-modal {
  animation: ss-modal-in 0.2s ease-out;
}
</style>
