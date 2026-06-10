<script setup lang="ts">
import { ref, computed, onMounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { save, open } from "@tauri-apps/plugin-dialog";
import { writeTextFile, readTextFile } from "@tauri-apps/plugin-fs";
import { useTerminalStore } from "../stores/terminal";
import { useProfilesStore } from "../stores/profiles";
import type { ThemeName } from "../lib/themes";
import LogoWordmark from "./LogoWordmark.vue";
import { Icon } from "@iconify/vue";
import { openUrl } from "@tauri-apps/plugin-opener";
import { getVersion } from "@tauri-apps/api/app";
import { useUpdaterStore } from "../stores/updater";

const emit = defineEmits<{
  "change-password": [];
  back: [];
}>();

const terminalStore = useTerminalStore();
const profilesStore = useProfilesStore();

const themeOptions: { value: ThemeName; label: string; preview: string }[] = [
  { value: "shello-dark", label: "Shello Dark", preview: "#121214" },
  { value: "shello-light", label: "Shello Light", preview: "#F5F5F5" },
  { value: "monokai", label: "Monokai", preview: "#272822" },
  { value: "dracula", label: "Dracula", preview: "#282A36" },
];

type Section = "terminal" | "connection" | "security" | "data" | "shortcuts" | "about";
const activeSection = ref<Section>("terminal");

const sections: { id: Section; label: string }[] = [
  { id: "terminal", label: "Terminal" },
  { id: "connection", label: "Connection" },
  { id: "security", label: "Security" },
  { id: "data", label: "Data" },
  { id: "shortcuts", label: "Shortcuts" },
  { id: "about", label: "About" },
];

const terminalFonts = [
  { name: "JetBrains Mono", category: "popular" },
  { name: "Fira Code", category: "popular" },
  { name: "Source Code Pro", category: "popular" },
  { name: "IBM Plex Mono", category: "popular" },
  { name: "Inconsolata", category: "classic" },
  { name: "Ubuntu Mono", category: "classic" },
  { name: "Space Mono", category: "stylish" },
];

const shortcuts = [
  { keys: ["⌘", "T"], action: "New Connection" },
  { keys: ["⌘", "W"], action: "Close Tab" },
  { keys: ["⌘", "1…9"], action: "Switch to Tab [1…9]" },
  { keys: ["⌘", "⇧", "]"], action: "Next Tab" },
  { keys: ["⌘", "⇧", "["], action: "Previous Tab" },
  { keys: ["⌘", "C"], action: "Copy from Terminal" },
  { keys: ["⌘", "V"], action: "Paste to Terminal" },
];

const isDark = computed(() => terminalStore.appMode === "dark");

const REPO_URL = "https://github.com/yolkmonday/shello";
const ISSUES_URL = "https://github.com/yolkmonday/shello/issues/new";

const updaterStore = useUpdaterStore();
const appVersion = ref("");
onMounted(async () => {
  try {
    appVersion.value = await getVersion();
  } catch {
    appVersion.value = "";
  }
});

const updaterStatusText = computed(() => {
  if (updaterStore.checking) return "Checking…";
  if (updaterStore.available) return `Version ${updaterStore.newVersion} available.`;
  if (updaterStore.upToDate) return "You're on the latest version.";
  if (updaterStore.error) return updaterStore.error;
  return "Check GitHub for new releases.";
});
const showFontDropdown = ref(false);

const importMessage = ref("");
const importError = ref(false);

async function forgetDevice() {
  try {
    await invoke("vault_forget_device");
    await profilesStore.checkVaultStatus();
  } catch (e) {
    console.error("Failed to forget device:", e);
  }
}

async function exportProfiles() {
  const profiles = Object.values(profilesStore.profiles);
  const groups = Object.values(profilesStore.groups);
  const data = JSON.stringify({ profiles, groups }, null, 2);

  const filePath = await save({
    title: "Export Profiles",
    defaultPath: `shello-profiles-${new Date().toISOString().slice(0, 10)}.json`,
    filters: [{ name: "JSON", extensions: ["json"] }],
  });
  if (!filePath) return;

  try {
    await writeTextFile(filePath, data);
    importError.value = false;
    importMessage.value = `Exported ${profiles.length} profile(s)`;
    setTimeout(() => { importMessage.value = ""; }, 4000);
  } catch (e) {
    importError.value = true;
    importMessage.value = `Export failed: ${e}`;
    setTimeout(() => { importMessage.value = ""; }, 4000);
  }
}

async function importProfiles() {
  const selected = await open({
    title: "Import Profiles",
    multiple: false,
    filters: [{ name: "JSON", extensions: ["json"] }],
  });
  if (!selected) return;

  try {
    const text = await readTextFile(selected);
    const data = JSON.parse(text);
    let importedCount = 0;

    // Import groups first
    const groupIdMap: Record<string, string> = {};
    if (data.groups && Array.isArray(data.groups)) {
      for (const g of data.groups) {
        const existing = Object.values(profilesStore.groups).find(
          (eg) => eg.name === g.name
        );
        if (existing) {
          groupIdMap[g.id] = existing.id;
        } else {
          const created = await profilesStore.createGroup(g.name, g.color || "#6B7280");
          groupIdMap[g.id] = created.id;
        }
      }
    }

    // Import profiles
    const profileList = data.profiles || data;
    if (Array.isArray(profileList)) {
      for (const p of profileList) {
        const existing = Object.values(profilesStore.profiles).find(
          (ep) => ep.host === p.host && ep.username === p.username && ep.port === p.port
        );
        if (existing) continue;

        await profilesStore.createProfile({
          name: p.name || `${p.username}@${p.host}`,
          host: p.host,
          port: p.port,
          username: p.username,
          auth_type: p.auth_type || "password",
          group_id: p.group_id ? (groupIdMap[p.group_id] || null) : null,
          tags: p.tags || "",
        });
        importedCount++;
      }
    }

    importError.value = false;
    importMessage.value = importedCount > 0
      ? `Imported ${importedCount} profile${importedCount > 1 ? "s" : ""}`
      : "No new profiles to import (all duplicates)";
  } catch (e) {
    importError.value = true;
    importMessage.value = `Import failed: ${e}`;
  }

  setTimeout(() => { importMessage.value = ""; }, 4000);
}
</script>

<template>
  <div class="h-full flex">
    <!-- Sidebar -->
    <div class="w-44 lg:w-52 flex-shrink-0 border-r border-otter-border flex flex-col bg-otter-card/50">
      <div class="px-5 py-5">
        <h1 class="text-sm font-bold text-otter-text tracking-tight">Settings</h1>
      </div>

      <!-- Nav items -->
      <nav class="flex-1 px-2 flex flex-col gap-0.5">
        <button
          v-for="s in sections"
          :key="s.id"
          class="w-full text-left px-3 py-2 rounded-lg text-sm transition-colors"
          :class="activeSection === s.id
            ? 'bg-otter-surface text-otter-text font-medium'
            : 'text-otter-muted hover:text-otter-text hover:bg-otter-surface/50'"
          @click="activeSection = s.id"
        >
          {{ s.label }}
        </button>
      </nav>

      <!-- External links -->
      <div class="px-2 pt-3 mt-3 border-t border-otter-border flex flex-col gap-0.5">
        <button
          class="w-full flex items-center gap-2 px-3 py-2 rounded-lg text-sm text-otter-muted hover:text-otter-text hover:bg-otter-surface/50 transition-colors"
          @click="openUrl(ISSUES_URL)"
        >
          <Icon icon="mdi:bug-outline" class="w-4 h-4" />
          <span>Report an issue</span>
        </button>
        <button
          class="w-full flex items-center gap-2 px-3 py-2 rounded-lg text-sm text-otter-muted hover:text-otter-text hover:bg-otter-surface/50 transition-colors"
          @click="openUrl(REPO_URL)"
        >
          <Icon icon="mdi:github" class="w-4 h-4" />
          <span>GitHub</span>
        </button>
      </div>

      <!-- Dark/Light toggle -->
      <div class="px-3 pb-4 pt-1">
        <button
          class="w-full flex items-center justify-between px-3 py-2 rounded-lg
                 hover:bg-otter-surface/50 transition-colors"
          @click="terminalStore.toggleAppMode()"
        >
          <div class="flex items-center gap-2">
            <Icon
              :icon="isDark ? 'mdi:weather-night' : 'mdi:weather-sunny'"
              class="w-4 h-4"
              :class="isDark ? 'text-otter-muted' : 'text-otter-amber'"
            />
            <span class="text-sm text-otter-muted">{{ isDark ? 'Dark' : 'Light' }}</span>
          </div>
          <!-- Toggle switch -->
          <div
            class="w-8 h-[18px] rounded-full transition-colors relative"
            :class="isDark ? 'bg-otter-subtle' : 'bg-otter-teal'"
          >
            <div
              class="absolute top-[2px] w-[14px] h-[14px] rounded-full bg-white shadow-sm transition-transform"
              :class="isDark ? 'left-[2px]' : 'left-[16px]'"
            ></div>
          </div>
        </button>
      </div>
    </div>

    <!-- Main content -->
    <div class="flex-1 overflow-y-auto">
      <div class="max-w-2xl px-4 sm:px-6 lg:px-8 py-6 lg:py-8">

        <!-- Terminal -->
        <div v-if="activeSection === 'terminal'" class="flex flex-col gap-6">
          <div>
            <h2 class="text-base font-semibold text-otter-text mb-1">Terminal</h2>
            <p class="text-xs text-otter-subtle">Terminal display settings</p>
          </div>

          <!-- Font Size -->
          <div>
            <div class="flex items-center justify-between mb-2">
              <label class="text-sm text-otter-muted">Font Size</label>
              <span class="text-sm font-mono text-otter-text">{{ terminalStore.fontSize }}px</span>
            </div>
            <div class="flex items-center gap-3">
              <button
                class="w-7 h-7 rounded-md bg-otter-surface border border-otter-border
                       text-otter-muted hover:text-otter-text hover:border-otter-subtle
                       flex items-center justify-center text-sm transition-colors"
                @click="terminalStore.setFontSize(terminalStore.fontSize - 1)"
              >
                −
              </button>
              <input
                type="range"
                :value="terminalStore.fontSize"
                min="10"
                max="24"
                step="1"
                class="flex-1"
                @input="terminalStore.setFontSize(Number(($event.target as HTMLInputElement).value))"
              />
              <button
                class="w-7 h-7 rounded-md bg-otter-surface border border-otter-border
                       text-otter-muted hover:text-otter-text hover:border-otter-subtle
                       flex items-center justify-center text-sm transition-colors"
                @click="terminalStore.setFontSize(terminalStore.fontSize + 1)"
              >
                +
              </button>
            </div>
          </div>

          <!-- Font Family -->
          <div>
            <label class="text-sm text-otter-muted mb-2 block">Font</label>
            <div class="relative">
              <button
                class="w-full flex items-center justify-between px-3 py-2.5 rounded-lg
                       bg-otter-surface border border-otter-border text-sm text-otter-text
                       hover:border-otter-subtle transition-colors"
                @click="showFontDropdown = !showFontDropdown"
              >
                <span :style="{ fontFamily: `'${terminalStore.fontFamily}', monospace` }">{{ terminalStore.fontFamily }}</span>
                <Icon icon="mdi:chevron-down" class="w-4 h-4 text-otter-subtle" />
              </button>
              <div
                v-if="showFontDropdown"
                class="absolute z-10 mt-1 w-full rounded-lg bg-otter-card border border-otter-border
                       shadow-lg overflow-hidden"
              >
                <template v-for="cat in ['popular', 'classic', 'stylish']" :key="cat">
                  <div class="px-3 py-1.5 text-[10px] text-otter-subtle uppercase tracking-wider bg-otter-surface/50">
                    {{ cat }}
                  </div>
                  <button
                    v-for="f in terminalFonts.filter(ff => ff.category === cat)"
                    :key="f.name"
                    class="w-full flex items-center justify-between px-3 py-2 text-sm
                           hover:bg-otter-surface transition-colors"
                    :class="terminalStore.fontFamily === f.name ? 'text-otter-teal' : 'text-otter-text'"
                    :style="{ fontFamily: `'${f.name}', monospace` }"
                    @click="terminalStore.setFontFamily(f.name); showFontDropdown = false"
                  >
                    {{ f.name }}
                    <Icon v-if="terminalStore.fontFamily === f.name" icon="mdi:check" class="w-4 h-4" />
                  </button>
                </template>
              </div>
            </div>
          </div>

          <!-- Theme selector -->
          <div>
            <label class="text-sm text-otter-muted mb-2 block">Theme</label>
            <div class="grid grid-cols-2 gap-2">
              <button
                v-for="opt in themeOptions"
                :key="opt.value"
                class="flex items-center gap-3 px-3 py-2.5 rounded-lg border transition-all duration-150"
                :class="terminalStore.theme === opt.value
                  ? 'border-otter-teal bg-otter-teal/5'
                  : 'border-otter-border hover:border-otter-subtle'"
                @click="terminalStore.setTheme(opt.value)"
              >
                <div
                  class="w-5 h-5 rounded-md border border-otter-border flex-shrink-0"
                  :style="{ backgroundColor: opt.preview }"
                ></div>
                <span class="text-sm text-otter-text">{{ opt.label }}</span>
                <span
                  v-if="terminalStore.theme === opt.value"
                  class="ml-auto text-otter-teal text-xs"
                >✓</span>
              </button>
            </div>
          </div>

          <!-- Cursor Style -->
          <div>
            <label class="text-sm text-otter-muted mb-2 block">Cursor Style</label>
            <div class="flex gap-2">
              <button
                v-for="style in (['block', 'underline', 'bar'] as const)"
                :key="style"
                class="flex items-center gap-2 px-4 py-2.5 rounded-lg border transition-all duration-150 flex-1"
                :class="terminalStore.cursorStyle === style
                  ? 'border-otter-teal bg-otter-teal/5'
                  : 'border-otter-border hover:border-otter-subtle'"
                @click="terminalStore.cursorStyle = style"
              >
                <!-- Cursor preview -->
                <span
                  class="inline-block bg-otter-teal"
                  :class="{
                    'w-2.5 h-4 rounded-[1px]': style === 'block',
                    'w-2.5 h-[2px]': style === 'underline',
                    'w-[2px] h-4': style === 'bar'
                  }"
                ></span>
                <span class="text-sm text-otter-text capitalize">{{ style }}</span>
                <span
                  v-if="terminalStore.cursorStyle === style"
                  class="ml-auto text-otter-teal text-xs"
                >✓</span>
              </button>
            </div>
          </div>

          <!-- Cursor Blink -->
          <div class="flex items-center justify-between">
            <label class="text-sm text-otter-muted">Cursor Blink</label>
            <button
              class="w-8 h-[18px] rounded-full transition-colors relative"
              :class="terminalStore.cursorBlink ? 'bg-otter-teal' : 'bg-otter-subtle'"
              @click="terminalStore.cursorBlink = !terminalStore.cursorBlink"
            >
              <div
                class="absolute top-[2px] w-[14px] h-[14px] rounded-full bg-white shadow-sm transition-transform"
                :class="terminalStore.cursorBlink ? 'left-[16px]' : 'left-[2px]'"
              ></div>
            </button>
          </div>

          <!-- Scrollback Buffer -->
          <div>
            <div class="flex items-center justify-between mb-2">
              <label class="text-sm text-otter-muted">Scrollback Buffer</label>
              <span class="text-sm font-mono text-otter-text">{{ terminalStore.scrollback.toLocaleString() }} lines</span>
            </div>
            <select
              class="w-full px-3 py-2 rounded-lg bg-otter-surface border border-otter-border
                     text-sm text-otter-text focus:outline-none focus:border-otter-teal-dim cursor-pointer"
              :value="terminalStore.scrollback"
              @change="terminalStore.scrollback = Number(($event.target as HTMLSelectElement).value)"
            >
              <option :value="1000">1,000</option>
              <option :value="5000">5,000</option>
              <option :value="10000">10,000</option>
              <option :value="50000">50,000</option>
              <option :value="100000">100,000</option>
            </select>
          </div>

          <!-- Terminal Opacity -->
          <div>
            <div class="flex items-center justify-between mb-2">
              <label class="text-sm text-otter-muted">Opacity</label>
              <span class="text-sm font-mono text-otter-text">{{ terminalStore.terminalOpacity }}%</span>
            </div>
            <input
              type="range"
              :value="terminalStore.terminalOpacity"
              min="50"
              max="100"
              step="5"
              class="w-full"
              @input="terminalStore.terminalOpacity = Number(($event.target as HTMLInputElement).value)"
            />
          </div>

          <!-- Bell -->
          <div>
            <label class="text-sm text-otter-muted mb-2 block">Bell</label>
            <div class="flex gap-2">
              <button
                v-for="opt in (['none', 'visual'] as const)"
                :key="opt"
                class="flex-1 px-3 py-2 rounded-lg border text-sm transition-all duration-150 capitalize"
                :class="terminalStore.bellStyle === opt
                  ? 'border-otter-teal bg-otter-teal/5 text-otter-text'
                  : 'border-otter-border text-otter-muted hover:border-otter-subtle'"
                @click="terminalStore.bellStyle = opt"
              >
                {{ opt === 'none' ? 'Off' : 'Visual Flash' }}
              </button>
            </div>
          </div>

          <!-- Font Preview -->
          <div>
            <label class="text-sm text-otter-muted mb-2 block">Preview</label>
            <div
              class="rounded-lg bg-otter-dark border border-otter-border p-4 text-otter-teal"
              :style="{ fontSize: terminalStore.fontSize + 'px', fontFamily: `'${terminalStore.fontFamily}', monospace` }"
            >
              <span class="text-otter-muted">$</span> ssh user@server.example.com<br>
              <span class="text-otter-muted">$</span> <span class="text-otter-text">echo</span> "The quick brown fox => 0xFF"
            </div>
          </div>
        </div>

        <!-- Connection -->
        <div v-if="activeSection === 'connection'" class="flex flex-col gap-6">
          <div>
            <h2 class="text-base font-semibold text-otter-text mb-1">Connection</h2>
            <p class="text-xs text-otter-subtle">SSH connection settings</p>
          </div>

          <!-- Connection Timeout -->
          <div>
            <div class="flex items-center justify-between mb-2">
              <label class="text-sm text-otter-muted">Connection Timeout</label>
              <span class="text-sm font-mono text-otter-text">{{ terminalStore.connectTimeout }}s</span>
            </div>
            <div class="flex items-center gap-3">
              <button
                class="w-7 h-7 rounded-md bg-otter-surface border border-otter-border
                       text-otter-muted hover:text-otter-text hover:border-otter-subtle
                       flex items-center justify-center text-sm transition-colors"
                @click="terminalStore.setConnectTimeout(terminalStore.connectTimeout - 5)"
              >
                −
              </button>
              <input
                type="range"
                :value="terminalStore.connectTimeout"
                min="5"
                max="60"
                step="5"
                class="flex-1"
                @input="terminalStore.setConnectTimeout(Number(($event.target as HTMLInputElement).value))"
              />
              <button
                class="w-7 h-7 rounded-md bg-otter-surface border border-otter-border
                       text-otter-muted hover:text-otter-text hover:border-otter-subtle
                       flex items-center justify-center text-sm transition-colors"
                @click="terminalStore.setConnectTimeout(terminalStore.connectTimeout + 5)"
              >
                +
              </button>
            </div>
            <p class="text-[10px] text-otter-subtle mt-1">Time to wait before giving up on a connection</p>
          </div>

          <!-- Auto-reconnect -->
          <div class="flex items-center justify-between">
            <div>
              <p class="text-sm text-otter-text">Auto-reconnect</p>
              <p class="text-xs text-otter-subtle mt-0.5">Automatically retry when a session disconnects (max 3 attempts)</p>
            </div>
            <span class="text-xs text-otter-subtle">Enabled</span>
          </div>

          <!-- Keep-alive -->
          <div class="flex items-center justify-between">
            <div>
              <p class="text-sm text-otter-text">Keep-alive Interval</p>
              <p class="text-xs text-otter-subtle mt-0.5">Server inactivity timeout is 5 minutes</p>
            </div>
            <span class="text-xs text-otter-subtle">300s</span>
          </div>
        </div>

        <!-- Security -->
        <div v-if="activeSection === 'security'" class="flex flex-col gap-6">
          <div>
            <h2 class="text-base font-semibold text-otter-text mb-1">Security</h2>
            <p class="text-xs text-otter-subtle">Vault and credential management</p>
          </div>

          <!-- Vault Status -->
          <div class="flex items-center justify-between bg-otter-card border border-otter-border rounded-xl px-5 py-4">
            <div>
              <p class="text-sm text-otter-text">Vault Status</p>
              <p class="text-xs text-otter-subtle mt-0.5">
                {{ profilesStore.vaultStatus.initialized
                  ? (profilesStore.vaultStatus.unlocked ? 'Initialized and unlocked' : 'Initialized but locked')
                  : 'Not set up yet' }}
              </p>
            </div>
            <div
              class="w-2.5 h-2.5 rounded-full"
              :class="profilesStore.vaultStatus.unlocked
                ? 'bg-otter-teal'
                : profilesStore.vaultStatus.initialized
                  ? 'bg-otter-amber'
                  : 'bg-otter-subtle'"
            ></div>
          </div>

          <!-- Change Password -->
          <button
            v-if="profilesStore.vaultStatus.initialized"
            class="w-full bg-otter-card border border-otter-border rounded-xl px-5 py-4
                   text-left hover:border-otter-subtle transition-colors"
            @click="emit('change-password')"
          >
            <p class="text-sm text-otter-text">Change Master Password</p>
            <p class="text-xs text-otter-subtle mt-0.5">Update the password used to encrypt credentials</p>
          </button>

          <!-- Forget Device -->
          <button
            v-if="profilesStore.vaultStatus.initialized"
            class="w-full bg-otter-card border border-otter-border rounded-xl px-5 py-4
                   text-left hover:border-otter-subtle transition-colors"
            @click="forgetDevice"
          >
            <p class="text-sm text-otter-text">Forget This Device</p>
            <p class="text-xs text-otter-subtle mt-0.5">Remove saved vault key from OS keychain</p>
          </button>
        </div>


        <!-- Data -->
        <div v-if="activeSection === 'data'" class="flex flex-col gap-6">
          <div>
            <h2 class="text-base font-semibold text-otter-text mb-1">Data</h2>
            <p class="text-xs text-otter-subtle">Export and import server profiles</p>
          </div>

          <div class="bg-otter-card border border-otter-border rounded-xl divide-y divide-otter-border">
            <div class="px-5 py-4 flex items-center gap-3">
              <div class="flex-1">
                <p class="text-sm text-otter-text">Export Profiles</p>
                <p class="text-xs text-otter-subtle mt-0.5">Download all profiles as JSON (credentials not included)</p>
              </div>
              <button
                class="px-3 py-1.5 rounded-lg bg-otter-surface border border-otter-border
                       text-xs text-otter-text hover:border-otter-subtle transition-colors"
                @click="exportProfiles"
              >
                Export
              </button>
            </div>

            <div class="px-5 py-4 flex items-center gap-3">
              <div class="flex-1">
                <p class="text-sm text-otter-text">Import Profiles</p>
                <p class="text-xs text-otter-subtle mt-0.5">Load profiles from a JSON file (duplicates are skipped)</p>
              </div>
              <button
                class="px-3 py-1.5 rounded-lg bg-otter-surface border border-otter-border
                       text-xs text-otter-text hover:border-otter-subtle transition-colors"
                @click="importProfiles"
              >
                Import
              </button>
            </div>
          </div>

          <div v-if="importMessage">
            <p class="text-xs" :class="importError ? 'text-otter-coral' : 'text-otter-teal'">
              {{ importMessage }}
            </p>
          </div>
        </div>

        <!-- Shortcuts -->
        <div v-if="activeSection === 'shortcuts'" class="flex flex-col gap-6">
          <div>
            <h2 class="text-base font-semibold text-otter-text mb-1">Keyboard Shortcuts</h2>
            <p class="text-xs text-otter-subtle">Mac hotkeys</p>
          </div>

          <div class="bg-otter-card border border-otter-border rounded-xl overflow-hidden">
            <table class="w-full">
              <thead>
                <tr class="border-b border-otter-border">
                  <th class="px-5 py-3 text-left text-xs font-semibold text-otter-text">Shortcut</th>
                  <th class="px-5 py-3 text-left text-xs font-semibold text-otter-text">Action</th>
                </tr>
              </thead>
              <tbody class="divide-y divide-otter-border">
                <tr v-for="(sc, i) in shortcuts" :key="i">
                  <td class="px-5 py-3">
                    <div class="flex items-center gap-1.5">
                      <kbd
                        v-for="(key, ki) in sc.keys"
                        :key="ki"
                        class="inline-flex items-center justify-center min-w-[24px] h-6 px-1.5
                               rounded-md bg-otter-surface border border-otter-border
                               text-xs text-otter-muted font-mono"
                      >{{ key }}</kbd>
                    </div>
                  </td>
                  <td class="px-5 py-3 text-sm text-otter-muted">{{ sc.action }}</td>
                </tr>
              </tbody>
            </table>
          </div>
        </div>

        <!-- About -->
        <div v-if="activeSection === 'about'" class="flex flex-col gap-6">
          <div>
            <h2 class="text-base font-semibold text-otter-text mb-1">About</h2>
          </div>

          <div class="bg-otter-card border border-otter-border rounded-xl px-5 py-6">
            <div class="flex items-center gap-3">
              <LogoWordmark :height="28" />
              <span class="text-xs text-otter-subtle font-mono ml-1">v{{ appVersion || '—' }}</span>
            </div>
            <p class="text-xs text-otter-subtle mt-3">A modern SSH client built with Tauri.</p>
          </div>

          <!-- Software updates -->
          <div class="bg-otter-card border border-otter-border rounded-xl px-5 py-4 flex items-center justify-between gap-3">
            <div class="min-w-0">
              <p class="text-sm text-otter-text">Software updates</p>
              <p class="text-xs mt-0.5 truncate" :class="updaterStore.error ? 'text-red-400' : 'text-otter-subtle'">
                {{ updaterStatusText }}
              </p>
            </div>
            <button
              v-if="updaterStore.available"
              class="px-3 py-1.5 rounded-lg bg-otter-teal text-otter-dark text-xs font-medium hover:opacity-90 flex-shrink-0 disabled:opacity-60"
              :disabled="updaterStore.downloading"
              @click="updaterStore.install()"
            >
              {{ updaterStore.downloading ? 'Installing…' : 'Install &amp; restart' }}
            </button>
            <button
              v-else
              class="px-3 py-1.5 rounded-lg border border-otter-border text-otter-muted hover:text-otter-text hover:border-otter-subtle text-xs flex-shrink-0 disabled:opacity-60"
              :disabled="updaterStore.checking"
              @click="updaterStore.checkForUpdate(false)"
            >
              {{ updaterStore.checking ? 'Checking…' : 'Check for updates' }}
            </button>
          </div>
        </div>

      </div>
    </div>
  </div>
</template>
