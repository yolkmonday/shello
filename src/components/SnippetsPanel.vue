<script setup lang="ts">
import { ref, computed, onMounted } from "vue";
import { Icon } from "@iconify/vue";
import { useSnippetsStore } from "../stores/snippets";
import { useRegistryStore } from "../stores/registry";
import { type DirectorySnippet, type SnippetCategory } from "../lib/snippet-directory";

const emit = defineEmits<{
  close: [];
  run: [command: string];
}>();

const store = useSnippetsStore();
const registry = useRegistryStore();

// Tabs: "my" = user snippets, "directory" = browse catalog
const activeTab = ref<"my" | "directory">("my");

// My snippets state
const adding = ref(false);
const editing = ref<string | null>(null);
const newName = ref("");
const newCommand = ref("");
const editName = ref("");
const editCommand = ref("");
const filterText = ref("");

// Directory state
const expandedCategory = ref<string | null>(null);
const addedFeedback = ref<string | null>(null);
const directorySearch = ref("");

onMounted(() => {
  store.loadAll();
  registry.init();
});

// ── My Snippets ─────────────────────────────────────────────────────

async function addSnippet() {
  if (!newName.value.trim() || !newCommand.value.trim()) return;
  await store.create(newName.value.trim(), newCommand.value.trim());
  newName.value = "";
  newCommand.value = "";
  adding.value = false;
}

function startEdit(id: string) {
  const s = store.snippets.find((s) => s.id === id);
  if (!s) return;
  editing.value = id;
  editName.value = s.name;
  editCommand.value = s.command;
}

async function saveEdit() {
  if (!editing.value) return;
  await store.update(editing.value, {
    name: editName.value.trim(),
    command: editCommand.value.trim(),
  });
  editing.value = null;
}

function runSnippet(command: string) {
  emit("run", command);
}

const filteredSnippets = computed(() => {
  const q = filterText.value.toLowerCase();
  if (!q) return store.snippets;
  return store.snippets.filter(
    (s) =>
      s.name.toLowerCase().includes(q) ||
      s.command.toLowerCase().includes(q) ||
      s.tags.toLowerCase().includes(q)
  );
});

// ── Directory ───────────────────────────────────────────────────────

function fuzzyMatch(text: string, query: string): boolean {
  text = text.toLowerCase();
  query = query.toLowerCase();
  if (text.includes(query)) return true;
  // Check if all query chars appear in order (typo-tolerant subsequence)
  let ti = 0;
  for (let qi = 0; qi < query.length; qi++) {
    const idx = text.indexOf(query[qi], ti);
    if (idx === -1) return false;
    ti = idx + 1;
  }
  return query.length >= 2;
}

const filteredDirectory = computed((): SnippetCategory[] => {
  const q = directorySearch.value.trim();
  if (!q) return registry.snippetCategories;
  return registry.snippetCategories
    .map((cat) => {
      const matchingItems = cat.items.filter(
        (item) =>
          fuzzyMatch(item.name, q) ||
          fuzzyMatch(item.command, q) ||
          fuzzyMatch(item.description, q)
      );
      if (matchingItems.length > 0) return { ...cat, items: matchingItems };
      if (fuzzyMatch(cat.name, q)) return cat;
      return null;
    })
    .filter((c): c is SnippetCategory => c !== null);
});

function toggleCategory(name: string) {
  expandedCategory.value = expandedCategory.value === name ? null : name;
}

async function addFromDirectory(item: DirectorySnippet) {
  // Check if already added
  const exists = store.snippets.some(
    (s) => s.command === item.command
  );
  if (exists) {
    addedFeedback.value = item.command;
    setTimeout(() => { addedFeedback.value = null; }, 1500);
    return;
  }

  await store.create(item.name, item.command);
  addedFeedback.value = item.command;
  setTimeout(() => { addedFeedback.value = null; }, 1500);
}

function isSnippetAdded(command: string): boolean {
  return store.snippets.some((s) => s.command === command);
}
</script>

<template>
  <div
    class="fixed inset-0 z-50 flex justify-end"
    @click.self="emit('close')"
  >
    <div class="absolute inset-0 bg-black/60" @click="emit('close')"></div>
    <div
      class="relative w-full max-w-sm h-full bg-otter-card border-l border-otter-border
             overflow-y-auto animate-slide-in-right"
    >
      <div class="p-5">
        <div class="flex items-center justify-between mb-4">
          <h2 class="text-sm font-semibold text-otter-muted uppercase tracking-wider">
            Snippets
          </h2>
          <button
            class="text-otter-subtle hover:text-otter-text transition-colors text-lg"
            @click="emit('close')"
          >
            &times;
          </button>
        </div>

        <!-- Tabs -->
        <div class="flex gap-1 mb-4 p-0.5 rounded-lg bg-otter-surface">
          <button
            class="flex-1 py-1.5 rounded-md text-xs font-medium transition-colors"
            :class="activeTab === 'my'
              ? 'bg-otter-card text-otter-text shadow-sm'
              : 'text-otter-muted hover:text-otter-text'"
            @click="activeTab = 'my'"
          >
            My Snippets
            <span v-if="store.snippets.length" class="ml-1 text-otter-subtle">({{ store.snippets.length }})</span>
          </button>
          <button
            class="flex-1 py-1.5 rounded-md text-xs font-medium transition-colors"
            :class="activeTab === 'directory'
              ? 'bg-otter-card text-otter-text shadow-sm'
              : 'text-otter-muted hover:text-otter-text'"
            @click="activeTab = 'directory'"
          >
            Directory
          </button>
        </div>

        <!-- ═══ My Snippets Tab ═══ -->
        <div v-if="activeTab === 'my'">
          <!-- Search -->
          <input
            v-model="filterText"
            class="w-full px-3 py-2 mb-3 rounded-lg bg-otter-surface border border-otter-border
                   text-otter-text placeholder-otter-subtle text-xs
                   focus:outline-none focus:border-otter-teal-dim transition-colors"
            placeholder="Filter snippets..."
          />

          <!-- Add button -->
          <button
            v-if="!adding"
            class="w-full py-2 mb-3 rounded-lg border border-dashed border-otter-border
                   text-otter-muted text-xs hover:border-otter-teal-dim hover:text-otter-text
                   transition-colors"
            @click="adding = true"
          >
            + Add Snippet
          </button>

          <!-- Add form -->
          <div v-if="adding" class="mb-3 p-3 rounded-lg bg-otter-surface border border-otter-border">
            <input
              v-model="newName"
              class="w-full px-2 py-1.5 mb-2 rounded bg-otter-dark border border-otter-border
                     text-otter-text placeholder-otter-subtle text-xs
                     focus:outline-none focus:border-otter-teal-dim"
              placeholder="Name (e.g. Check disk)"
              @keyup.enter="addSnippet"
            />
            <textarea
              v-model="newCommand"
              class="w-full px-2 py-1.5 mb-2 rounded bg-otter-dark border border-otter-border
                     text-otter-text placeholder-otter-subtle text-xs font-mono resize-none
                     focus:outline-none focus:border-otter-teal-dim"
              rows="2"
              placeholder="Command (e.g. df -h)"
            ></textarea>
            <div class="flex gap-2">
              <button
                class="flex-1 py-1.5 rounded bg-otter-teal text-otter-dark text-xs font-semibold
                       hover:opacity-90 transition-opacity disabled:opacity-50"
                :disabled="!newName.trim() || !newCommand.trim()"
                @click="addSnippet"
              >
                Save
              </button>
              <button
                class="flex-1 py-1.5 rounded bg-otter-dark border border-otter-border text-otter-text
                       text-xs hover:border-otter-subtle transition-colors"
                @click="adding = false; newName = ''; newCommand = ''"
              >
                Cancel
              </button>
            </div>
          </div>

          <!-- Snippet list -->
          <div class="flex flex-col gap-2">
            <div
              v-for="snippet in filteredSnippets"
              :key="snippet.id"
              class="group p-3 rounded-lg bg-otter-surface border border-otter-border
                     hover:border-otter-teal-dim transition-colors"
            >
              <!-- Edit mode -->
              <template v-if="editing === snippet.id">
                <input
                  v-model="editName"
                  class="w-full px-2 py-1 mb-1.5 rounded bg-otter-dark border border-otter-border
                         text-otter-text text-xs focus:outline-none focus:border-otter-teal-dim"
                />
                <textarea
                  v-model="editCommand"
                  class="w-full px-2 py-1 mb-1.5 rounded bg-otter-dark border border-otter-border
                         text-otter-text text-xs font-mono resize-none
                         focus:outline-none focus:border-otter-teal-dim"
                  rows="2"
                ></textarea>
                <div class="flex gap-2">
                  <button
                    class="flex-1 py-1 rounded bg-otter-teal text-otter-dark text-xs font-semibold"
                    @click="saveEdit"
                  >Save</button>
                  <button
                    class="flex-1 py-1 rounded bg-otter-dark border border-otter-border text-otter-text text-xs"
                    @click="editing = null"
                  >Cancel</button>
                </div>
              </template>

              <!-- View mode -->
              <template v-else>
                <div class="flex items-center justify-between mb-1">
                  <span class="text-xs font-semibold text-otter-text">{{ snippet.name }}</span>
                  <div class="flex items-center gap-1 opacity-0 group-hover:opacity-100 transition-opacity">
                    <button
                      class="p-1 text-otter-subtle hover:text-otter-text transition-colors"
                      title="Edit"
                      @click="startEdit(snippet.id)"
                    >
                      <svg class="w-3 h-3" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5"><path d="M11.5 1.5l3 3L5 14H2v-3L11.5 1.5z"/></svg>
                    </button>
                    <button
                      class="p-1 text-otter-subtle hover:text-otter-coral transition-colors"
                      title="Delete"
                      @click="store.remove(snippet.id)"
                    >
                      <svg class="w-3 h-3" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5"><path d="M4 4l8 8M12 4l-8 8"/></svg>
                    </button>
                  </div>
                </div>
                <div
                  class="px-2 py-1.5 rounded bg-otter-dark text-xs font-mono text-otter-muted
                         cursor-pointer hover:text-otter-teal transition-colors"
                  title="Click to run"
                  @click="runSnippet(snippet.command)"
                >
                  {{ snippet.command }}
                </div>
              </template>
            </div>

            <div
              v-if="filteredSnippets.length === 0 && !adding"
              class="flex flex-col items-center gap-2 py-8"
            >
              <p class="text-xs text-otter-subtle">No snippets yet</p>
              <button
                class="text-xs text-otter-teal hover:underline"
                @click="activeTab = 'directory'"
              >
                Browse Directory to get started
              </button>
            </div>
          </div>
        </div>

        <!-- ═══ Directory Tab ═══ -->
        <div v-if="activeTab === 'directory'">
          <p class="text-xs text-otter-subtle mb-3">
            Common server commands. Click <strong class="text-otter-text">+ Add</strong> to save to your snippets.
          </p>

          <!-- Directory search -->
          <input
            v-model="directorySearch"
            class="w-full px-3 py-2 mb-3 rounded-lg bg-otter-surface border border-otter-border
                   text-otter-text placeholder-otter-subtle text-xs
                   focus:outline-none focus:border-otter-teal-dim transition-colors"
            placeholder="Search commands..."
          />

          <div v-if="filteredDirectory.length === 0" class="py-6 text-center">
            <p class="text-xs text-otter-subtle">No commands found for "{{ directorySearch }}"</p>
          </div>

          <div class="flex flex-col gap-1.5">
            <div
              v-for="category in filteredDirectory"
              :key="category.name"
            >
              <!-- Category header -->
              <button
                class="w-full flex items-center gap-2 px-3 py-2 rounded-lg text-left transition-colors"
                :class="expandedCategory === category.name
                  ? 'bg-otter-surface text-otter-text'
                  : 'hover:bg-otter-surface/50 text-otter-muted'"
                @click="toggleCategory(category.name)"
              >
                <Icon :icon="category.icon" class="w-4 h-4 flex-shrink-0" />
                <span class="text-xs font-semibold flex-1">{{ category.name }}</span>
                <span class="text-[10px] text-otter-subtle">{{ category.items.length }}</span>
                <svg
                  class="w-3 h-3 text-otter-subtle transition-transform"
                  :class="{ 'rotate-90': expandedCategory === category.name }"
                  viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="2"
                ><path d="M6 4l4 4-4 4"/></svg>
              </button>

              <!-- Category items -->
              <div
                v-if="expandedCategory === category.name || directorySearch.trim().length > 0"
                class="ml-2 mt-1 mb-2 flex flex-col gap-1"
              >
                <div
                  v-for="item in category.items"
                  :key="item.command"
                  class="p-2.5 rounded-lg bg-otter-surface/50 border border-otter-border/50
                         hover:border-otter-border transition-colors"
                >
                  <div class="flex items-start gap-2 mb-1.5">
                    <div class="flex-1 min-w-0">
                      <span class="text-xs font-semibold text-otter-text">{{ item.name }}</span>
                    </div>
                    <button
                      v-if="!isSnippetAdded(item.command)"
                      class="flex-shrink-0 px-2 py-0.5 rounded text-[10px] font-semibold
                             bg-otter-teal/10 text-otter-teal border border-otter-teal/30
                             hover:bg-otter-teal/20 transition-colors"
                      @click="addFromDirectory(item)"
                    >
                      + Add
                    </button>
                    <span
                      v-else
                      class="flex-shrink-0 px-2 py-0.5 rounded text-[10px]
                             text-otter-subtle"
                    >
                      {{ addedFeedback === item.command ? 'Added!' : '✓' }}
                    </span>
                  </div>
                  <p class="text-[11px] text-otter-muted leading-relaxed mb-1.5">
                    {{ item.description }}
                  </p>
                  <code class="block px-2 py-1 rounded bg-otter-dark text-[11px] font-mono text-otter-subtle truncate">
                    {{ item.command }}
                  </code>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
@keyframes slide-in-right {
  from { transform: translateX(100%); }
  to { transform: translateX(0); }
}
.animate-slide-in-right {
  animation: slide-in-right 0.25s ease-out;
}
</style>
