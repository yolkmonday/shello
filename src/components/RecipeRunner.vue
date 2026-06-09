<script setup lang="ts">
import { ref, computed, onMounted } from "vue";
import { Icon } from "@iconify/vue";
import { invoke } from "@tauri-apps/api/core";
import { useRegistryStore } from "../stores/registry";
import { useCustomRecipesStore } from "../stores/custom-recipes";
import { useTerminalStore } from "../stores/terminal";
import { type Recipe, type RecipeVariable, type RecipeStep, type StepStatus } from "../lib/recipe-types";

const emit = defineEmits<{
  close: [];
}>();

const registry = useRegistryStore();
const customStore = useCustomRecipesStore();
const termStore = useTerminalStore();

// Main state
const activeTab = ref<"directory" | "my">("directory");
const view = ref<"browse" | "setup" | "running" | "editor">("browse");
const searchQuery = ref("");
const selectedRecipe = ref<Recipe | null>(null);
const variables = ref<Record<string, string>>({});
const stepStatuses = ref<StepStatus[]>([]);
const stepOutputs = ref<string[]>([]);
const currentStepIndex = ref(-1);
const isRunning = ref(false);
const isComplete = ref(false);
const editingId = ref<string | null>(null);

// Editor state
const editorName = ref("");
const editorDescription = ref("");
const editorIcon = ref("mdi:script-text-outline");
const editorTags = ref("");
const editorVariables = ref<RecipeVariable[]>([]);
const editorSteps = ref<RecipeStep[]>([]);

onMounted(() => {
  registry.init();
  customStore.loadAll();
});

// ── Browse ──────────────────────────────────────────────────────────

const allRecipes = computed(() => {
  if (activeTab.value === "my") return customStore.recipes;
  return registry.recipes;
});

const filteredRecipes = computed(() => {
  const q = searchQuery.value.toLowerCase().trim();
  const list = allRecipes.value;
  if (!q) return list;
  return list.filter(
    (r) =>
      r.name.toLowerCase().includes(q) ||
      r.description.toLowerCase().includes(q) ||
      r.tags.some((t) => t.toLowerCase().includes(q))
  );
});

function selectRecipe(recipe: Recipe) {
  selectedRecipe.value = recipe;
  variables.value = {};
  for (const v of recipe.variables) {
    variables.value[v.key] = v.default || "";
  }
  view.value = "setup";
}

// ── Setup ───────────────────────────────────────────────────────────

const canStart = computed(() => {
  if (!selectedRecipe.value) return false;
  return selectedRecipe.value.variables
    .filter((v) => v.required)
    .every((v) => variables.value[v.key]?.trim());
});

const hasActiveSession = computed(() => !!termStore.activeSessionId);

function substituteVars(command: string): string {
  let result = command;
  for (const [key, val] of Object.entries(variables.value)) {
    result = result.split(`{{${key}}}`).join(val);
  }
  return result;
}

// ── Running ─────────────────────────────────────────────────────────

async function startRecipe() {
  if (!selectedRecipe.value || !termStore.activeSessionId) return;
  view.value = "running";
  isRunning.value = true;
  isComplete.value = false;
  stepStatuses.value = selectedRecipe.value.steps.map(() => "pending");
  stepOutputs.value = selectedRecipe.value.steps.map(() => "");
  currentStepIndex.value = 0;

  for (let i = 0; i < selectedRecipe.value.steps.length; i++) {
    if (!isRunning.value) {
      for (let j = i; j < stepStatuses.value.length; j++) stepStatuses.value[j] = "skipped";
      break;
    }
    currentStepIndex.value = i;
    stepStatuses.value[i] = "running";
    const step = selectedRecipe.value.steps[i];
    const command = substituteVars(step.command);
    try {
      const output = await invoke<string>("ssh_exec", { sessionId: termStore.activeSessionId, command });
      stepOutputs.value[i] = output;
      stepStatuses.value[i] = "success";
    } catch (e) {
      stepOutputs.value[i] = String(e);
      if (step.optional) {
        stepStatuses.value[i] = "skipped";
      } else {
        stepStatuses.value[i] = "failed";
        for (let j = i + 1; j < stepStatuses.value.length; j++) stepStatuses.value[j] = "skipped";
        break;
      }
    }
  }
  isRunning.value = false;
  isComplete.value = true;
}

function cancelRecipe() { isRunning.value = false; }

function backToBrowse() {
  view.value = "browse";
  selectedRecipe.value = null;
  isComplete.value = false;
}

// ── Editor ──────────────────────────────────────────────────────────

function openEditor(recipe?: Recipe) {
  if (recipe) {
    editingId.value = recipe.id;
    editorName.value = recipe.name;
    editorDescription.value = recipe.description;
    editorIcon.value = recipe.icon;
    editorTags.value = recipe.tags.join(", ");
    editorVariables.value = recipe.variables.map((v) => ({ ...v }));
    editorSteps.value = recipe.steps.map((s) => ({ ...s }));
  } else {
    editingId.value = null;
    editorName.value = "";
    editorDescription.value = "";
    editorIcon.value = "mdi:script-text-outline";
    editorTags.value = "";
    editorVariables.value = [];
    editorSteps.value = [];
  }
  view.value = "editor";
}

function addVariable() {
  editorVariables.value.push({ key: "", label: "", placeholder: "", required: true });
}
function removeVariable(i: number) { editorVariables.value.splice(i, 1); }

function addStep() {
  editorSteps.value.push({ name: "", command: "", description: "" });
}
function removeStep(i: number) { editorSteps.value.splice(i, 1); }
function moveStep(i: number, dir: -1 | 1) {
  const j = i + dir;
  if (j < 0 || j >= editorSteps.value.length) return;
  const tmp = editorSteps.value[i];
  editorSteps.value[i] = editorSteps.value[j];
  editorSteps.value[j] = tmp;
}

const canSave = computed(() =>
  editorName.value.trim() &&
  editorSteps.value.length > 0 &&
  editorSteps.value.every((s) => s.name.trim() && s.command.trim())
);

async function saveRecipe() {
  if (!canSave.value) return;
  const data = {
    name: editorName.value.trim(),
    description: editorDescription.value.trim(),
    icon: editorIcon.value.trim() || "mdi:script-text-outline",
    tags: editorTags.value.split(",").map((t) => t.trim()).filter(Boolean),
    variables: editorVariables.value.filter((v) => v.key.trim()),
    steps: editorSteps.value,
  };
  if (editingId.value) {
    await customStore.update(editingId.value, data);
  } else {
    await customStore.create(data);
  }
  view.value = "browse";
  activeTab.value = "my";
}

async function deleteRecipe(id: string) {
  await customStore.remove(id);
}

// ── Helpers ─────────────────────────────────────────────────────────

const statusIcon: Record<StepStatus, string> = {
  pending: "mdi:circle-outline", running: "mdi:loading",
  success: "mdi:check-circle", failed: "mdi:close-circle", skipped: "mdi:minus-circle-outline",
};
const statusColor: Record<StepStatus, string> = {
  pending: "text-otter-subtle", running: "text-otter-teal animate-spin",
  success: "text-emerald-400", failed: "text-red-400", skipped: "text-otter-subtle",
};
const successCount = computed(() => stepStatuses.value.filter((s) => s === "success").length);
const failedCount = computed(() => stepStatuses.value.filter((s) => s === "failed").length);

const inputClass = "w-full px-2.5 py-1.5 rounded bg-otter-surface border border-otter-border text-otter-text placeholder-otter-subtle text-xs focus:outline-none focus:border-otter-teal-dim transition-colors";
const monoInputClass = inputClass + " font-mono";
</script>

<template>
  <div class="fixed inset-0 z-50 flex justify-end" @click.self="emit('close')">
    <div class="absolute inset-0 bg-black/60" @click="emit('close')"></div>
    <div class="relative w-full max-w-md h-full bg-otter-card border-l border-otter-border overflow-y-auto animate-slide-in-right">
      <div class="p-5">
        <!-- Header -->
        <div class="flex items-center justify-between mb-4">
          <div class="flex items-center gap-2">
            <button
              v-if="view !== 'browse'"
              class="text-otter-subtle hover:text-otter-text transition-colors"
              @click="view === 'editor' || view === 'setup' ? backToBrowse() : undefined"
              :disabled="isRunning"
            >
              <Icon icon="mdi:arrow-left" class="w-4 h-4" />
            </button>
            <h2 class="text-sm font-semibold text-otter-muted uppercase tracking-wider">
              {{ view === 'editor' ? (editingId ? 'Edit Recipe' : 'New Recipe') : view === 'browse' ? 'Server Recipes' : selectedRecipe?.name }}
            </h2>
          </div>
          <button class="text-otter-subtle hover:text-otter-text transition-colors text-lg" @click="emit('close')">&times;</button>
        </div>

        <!-- ═══ Browse View ═══ -->
        <template v-if="view === 'browse'">
          <!-- Tabs -->
          <div class="flex gap-1 mb-4 p-0.5 rounded-lg bg-otter-surface">
            <button
              class="flex-1 py-1.5 rounded-md text-xs font-medium transition-colors"
              :class="activeTab === 'directory' ? 'bg-otter-card text-otter-text shadow-sm' : 'text-otter-muted hover:text-otter-text'"
              @click="activeTab = 'directory'"
            >
              Directory
              <span v-if="registry.recipes.length" class="ml-1 text-otter-subtle">({{ registry.recipes.length }})</span>
            </button>
            <button
              class="flex-1 py-1.5 rounded-md text-xs font-medium transition-colors"
              :class="activeTab === 'my' ? 'bg-otter-card text-otter-text shadow-sm' : 'text-otter-muted hover:text-otter-text'"
              @click="activeTab = 'my'"
            >
              My Recipes
              <span v-if="customStore.recipes.length" class="ml-1 text-otter-subtle">({{ customStore.recipes.length }})</span>
            </button>
          </div>

          <!-- Search -->
          <input
            v-model="searchQuery"
            class="w-full px-3 py-2 mb-3 rounded-lg bg-otter-surface border border-otter-border text-otter-text placeholder-otter-subtle text-xs focus:outline-none focus:border-otter-teal-dim transition-colors"
            placeholder="Search recipes..."
          />

          <!-- Create button (My tab) -->
          <button
            v-if="activeTab === 'my'"
            class="w-full py-2 mb-3 rounded-lg border border-dashed border-otter-border text-otter-muted text-xs hover:border-otter-teal-dim hover:text-otter-text transition-colors"
            @click="openEditor()"
          >
            + Create Recipe
          </button>

          <!-- Empty state -->
          <div v-if="filteredRecipes.length === 0" class="py-8 text-center">
            <p class="text-xs text-otter-subtle">
              {{ activeTab === 'my' ? 'No custom recipes yet' : 'No recipes found' }}
            </p>
            <button
              v-if="activeTab === 'my'"
              class="mt-2 text-xs text-otter-teal hover:underline"
              @click="openEditor()"
            >
              Create your first recipe
            </button>
          </div>

          <!-- Recipe list -->
          <div class="flex flex-col gap-2">
            <div
              v-for="recipe in filteredRecipes"
              :key="recipe.id"
              class="group p-3 rounded-lg bg-otter-surface border border-otter-border hover:border-otter-teal-dim transition-colors"
            >
              <div class="flex items-start gap-3 cursor-pointer" @click="selectRecipe(recipe)">
                <div class="w-8 h-8 rounded-lg bg-otter-dark flex items-center justify-center flex-shrink-0">
                  <Icon :icon="recipe.icon" class="w-4 h-4 text-otter-teal" />
                </div>
                <div class="flex-1 min-w-0">
                  <div class="text-xs font-semibold text-otter-text mb-0.5">{{ recipe.name }}</div>
                  <p class="text-[11px] text-otter-muted leading-relaxed">{{ recipe.description }}</p>
                  <div class="flex flex-wrap gap-1 mt-1.5">
                    <span v-for="tag in recipe.tags.slice(0, 4)" :key="tag" class="px-1.5 py-0.5 rounded text-[9px] bg-otter-dark text-otter-subtle">{{ tag }}</span>
                    <span v-if="recipe.os && recipe.os.length" class="px-1.5 py-0.5 rounded text-[9px] bg-otter-teal/10 text-otter-teal">{{ recipe.os.join(', ') }}</span>
                  </div>
                </div>
                <Icon icon="mdi:chevron-right" class="w-4 h-4 text-otter-subtle group-hover:text-otter-teal transition-colors flex-shrink-0 mt-1" />
              </div>
              <!-- Edit/Delete for custom recipes -->
              <div v-if="activeTab === 'my'" class="flex gap-2 mt-2 pt-2 border-t border-otter-border/50">
                <button class="text-[10px] text-otter-subtle hover:text-otter-text transition-colors" @click.stop="openEditor(recipe)">Edit</button>
                <button class="text-[10px] text-otter-subtle hover:text-red-400 transition-colors" @click.stop="deleteRecipe(recipe.id)">Delete</button>
              </div>
            </div>
          </div>

          <div v-if="registry.syncing" class="mt-4 text-center text-[10px] text-otter-subtle flex items-center justify-center gap-1">
            <Icon icon="mdi:loading" class="w-3 h-3 animate-spin" />
            Syncing recipes...
          </div>
        </template>

        <!-- ═══ Editor View ═══ -->
        <template v-if="view === 'editor'">
          <div class="flex flex-col gap-3">
            <!-- Basic info -->
            <div>
              <label class="block text-[10px] font-semibold text-otter-subtle uppercase tracking-wider mb-1">Name *</label>
              <input v-model="editorName" :class="inputClass" placeholder="e.g. Deploy My App" />
            </div>
            <div>
              <label class="block text-[10px] font-semibold text-otter-subtle uppercase tracking-wider mb-1">Description</label>
              <textarea v-model="editorDescription" :class="inputClass" rows="2" class="resize-none" placeholder="What does this recipe do?"></textarea>
            </div>
            <div class="flex gap-2">
              <div class="flex-1">
                <label class="block text-[10px] font-semibold text-otter-subtle uppercase tracking-wider mb-1">Icon</label>
                <div class="flex items-center gap-2">
                  <Icon :icon="editorIcon || 'mdi:script-text-outline'" class="w-5 h-5 text-otter-teal flex-shrink-0" />
                  <input v-model="editorIcon" :class="inputClass" placeholder="mdi:icon-name" />
                </div>
              </div>
              <div class="flex-1">
                <label class="block text-[10px] font-semibold text-otter-subtle uppercase tracking-wider mb-1">Tags</label>
                <input v-model="editorTags" :class="inputClass" placeholder="docker, deploy" />
              </div>
            </div>

            <!-- Variables -->
            <div>
              <div class="flex items-center justify-between mb-1">
                <label class="text-[10px] font-semibold text-otter-subtle uppercase tracking-wider">Variables</label>
                <button class="text-[10px] text-otter-teal hover:underline" @click="addVariable">+ Add</button>
              </div>
              <p class="text-[10px] text-otter-subtle mb-2">Use <code class="text-otter-teal" v-pre>{{KEY}}</code> in commands to reference variables.</p>
              <div class="flex flex-col gap-2">
                <div v-for="(v, i) in editorVariables" :key="i" class="p-2 rounded bg-otter-surface border border-otter-border/50">
                  <div class="flex gap-2 mb-1.5">
                    <input v-model="v.key" :class="monoInputClass" class="flex-1" placeholder="KEY" />
                    <input v-model="v.label" :class="inputClass" class="flex-1" placeholder="Label" />
                    <button class="text-otter-subtle hover:text-red-400 transition-colors px-1" @click="removeVariable(i)">
                      <Icon icon="mdi:close" class="w-3.5 h-3.5" />
                    </button>
                  </div>
                  <div class="flex gap-2">
                    <input v-model="v.placeholder" :class="inputClass" class="flex-1" placeholder="Placeholder" />
                    <input v-model="v.default" :class="inputClass" class="flex-1" placeholder="Default (optional)" />
                    <label class="flex items-center gap-1 text-[10px] text-otter-subtle flex-shrink-0">
                      <input type="checkbox" v-model="v.required" class="rounded" /> Req
                    </label>
                  </div>
                </div>
              </div>
            </div>

            <!-- Steps -->
            <div>
              <div class="flex items-center justify-between mb-1">
                <label class="text-[10px] font-semibold text-otter-subtle uppercase tracking-wider">Steps *</label>
                <button class="text-[10px] text-otter-teal hover:underline" @click="addStep">+ Add Step</button>
              </div>
              <div class="flex flex-col gap-2">
                <div v-for="(step, i) in editorSteps" :key="i" class="p-2.5 rounded bg-otter-surface border border-otter-border/50">
                  <div class="flex items-center gap-2 mb-1.5">
                    <span class="text-[10px] text-otter-subtle w-4 text-center">{{ i + 1 }}</span>
                    <input v-model="step.name" :class="inputClass" class="flex-1" placeholder="Step name" />
                    <div class="flex gap-0.5">
                      <button class="text-otter-subtle hover:text-otter-text p-0.5" @click="moveStep(i, -1)" :disabled="i === 0">
                        <Icon icon="mdi:arrow-up" class="w-3 h-3" />
                      </button>
                      <button class="text-otter-subtle hover:text-otter-text p-0.5" @click="moveStep(i, 1)" :disabled="i === editorSteps.length - 1">
                        <Icon icon="mdi:arrow-down" class="w-3 h-3" />
                      </button>
                      <button class="text-otter-subtle hover:text-red-400 p-0.5" @click="removeStep(i)">
                        <Icon icon="mdi:close" class="w-3 h-3" />
                      </button>
                    </div>
                  </div>
                  <textarea v-model="step.command" :class="monoInputClass" rows="2" class="resize-none mb-1.5" placeholder="Command to execute"></textarea>
                  <input v-model="step.description" :class="inputClass" placeholder="What this step does (optional)" />
                </div>
              </div>
              <button
                v-if="editorSteps.length > 0"
                class="mt-2 w-full py-1.5 rounded border border-dashed border-otter-border text-otter-subtle text-[10px] hover:border-otter-teal-dim hover:text-otter-text transition-colors"
                @click="addStep"
              >
                + Add Another Step
              </button>
            </div>

            <!-- Save -->
            <button
              class="w-full py-2.5 rounded-lg text-xs font-semibold transition-all disabled:opacity-40"
              :class="canSave ? 'bg-otter-teal text-otter-dark hover:opacity-90' : 'bg-otter-surface text-otter-subtle'"
              :disabled="!canSave"
              @click="saveRecipe"
            >
              {{ editingId ? 'Update Recipe' : 'Create Recipe' }}
            </button>
          </div>
        </template>

        <!-- ═══ Setup View ═══ -->
        <template v-if="view === 'setup' && selectedRecipe">
          <p class="text-xs text-otter-muted mb-4">{{ selectedRecipe.description }}</p>

          <div v-if="selectedRecipe.variables.length > 0" class="mb-4">
            <h3 class="text-[10px] font-semibold text-otter-subtle uppercase tracking-wider mb-2">Configuration</h3>
            <div class="flex flex-col gap-2">
              <div v-for="v in selectedRecipe.variables" :key="v.key">
                <label class="block text-[11px] text-otter-muted mb-1">
                  {{ v.label }} <span v-if="v.required" class="text-red-400">*</span>
                </label>
                <input v-model="variables[v.key]" :placeholder="v.placeholder" :class="monoInputClass" />
              </div>
            </div>
          </div>

          <div class="mb-4">
            <h3 class="text-[10px] font-semibold text-otter-subtle uppercase tracking-wider mb-2">Steps ({{ selectedRecipe.steps.length }})</h3>
            <div class="flex flex-col gap-1">
              <div v-for="(step, i) in selectedRecipe.steps" :key="i" class="flex items-start gap-2 p-2 rounded bg-otter-surface/50">
                <span class="text-[10px] text-otter-subtle mt-0.5 w-4 text-right flex-shrink-0">{{ i + 1 }}</span>
                <div class="min-w-0">
                  <div class="text-[11px] text-otter-text font-medium">{{ step.name }}</div>
                  <p class="text-[10px] text-otter-subtle">{{ step.description }}</p>
                </div>
              </div>
            </div>
          </div>

          <div v-if="!hasActiveSession" class="mb-3 p-2.5 rounded-lg bg-amber-500/10 border border-amber-500/30 text-[11px] text-amber-300">
            Connect to a server first. Recipes run commands on the active session.
          </div>

          <button
            class="w-full py-2.5 rounded-lg text-xs font-semibold transition-all disabled:opacity-40 disabled:cursor-not-allowed"
            :class="canStart && hasActiveSession ? 'bg-otter-teal text-otter-dark hover:opacity-90' : 'bg-otter-surface text-otter-subtle'"
            :disabled="!canStart || !hasActiveSession"
            @click="startRecipe"
          >
            Run Recipe
          </button>
        </template>

        <!-- ═══ Running View ═══ -->
        <template v-if="view === 'running' && selectedRecipe">
          <div v-if="isComplete" class="mb-4 p-3 rounded-lg border" :class="failedCount > 0 ? 'bg-red-500/10 border-red-500/30' : 'bg-emerald-500/10 border-emerald-500/30'">
            <div class="flex items-center gap-2">
              <Icon :icon="failedCount > 0 ? 'mdi:alert-circle' : 'mdi:check-circle'" class="w-4 h-4" :class="failedCount > 0 ? 'text-red-400' : 'text-emerald-400'" />
              <span class="text-xs font-semibold" :class="failedCount > 0 ? 'text-red-300' : 'text-emerald-300'">
                {{ failedCount > 0 ? `Completed with ${failedCount} error(s)` : 'All steps completed successfully!' }}
              </span>
            </div>
            <p class="text-[10px] text-otter-muted mt-1">{{ successCount }}/{{ selectedRecipe.steps.length }} steps succeeded</p>
          </div>

          <div class="flex flex-col gap-1.5">
            <div v-for="(step, i) in selectedRecipe.steps" :key="i" class="rounded-lg border transition-colors" :class="currentStepIndex === i && isRunning ? 'bg-otter-surface border-otter-teal/30' : 'bg-otter-surface/50 border-otter-border/50'">
              <div class="flex items-start gap-2 p-2.5">
                <Icon :icon="statusIcon[stepStatuses[i]]" class="w-4 h-4 flex-shrink-0 mt-0.5" :class="statusColor[stepStatuses[i]]" />
                <div class="flex-1 min-w-0">
                  <div class="text-[11px] font-medium" :class="stepStatuses[i] === 'running' ? 'text-otter-teal' : 'text-otter-text'">{{ step.name }}</div>
                  <p class="text-[10px] text-otter-subtle">{{ step.description }}</p>
                </div>
              </div>
              <div v-if="stepOutputs[i]" class="px-2.5 pb-2.5">
                <pre class="p-2 rounded bg-otter-dark text-[10px] font-mono text-otter-subtle overflow-x-auto max-h-24 overflow-y-auto whitespace-pre-wrap break-all">{{ stepOutputs[i] }}</pre>
              </div>
            </div>
          </div>

          <div class="mt-4 flex gap-2">
            <button v-if="isRunning" class="flex-1 py-2 rounded-lg bg-red-500/20 text-red-300 text-xs font-semibold hover:bg-red-500/30 transition-colors" @click="cancelRecipe">Cancel</button>
            <button v-if="isComplete" class="flex-1 py-2 rounded-lg bg-otter-surface border border-otter-border text-otter-text text-xs font-semibold hover:border-otter-teal-dim transition-colors" @click="backToBrowse">Back to Recipes</button>
            <button v-if="isComplete && failedCount > 0" class="flex-1 py-2 rounded-lg bg-otter-teal text-otter-dark text-xs font-semibold hover:opacity-90 transition-opacity" @click="startRecipe">Retry</button>
          </div>
        </template>
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
