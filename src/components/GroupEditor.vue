<script setup lang="ts">
import { ref, onMounted } from "vue";
import { useProfilesStore, type Group } from "../stores/profiles";

const props = defineProps<{
  group?: Group;
}>();

const emit = defineEmits<{
  close: [];
  saved: [];
}>();

const store = useProfilesStore();

const name = ref("");
const color = ref("#475569");
const saving = ref(false);
const error = ref("");

const isEditing = !!props.group;

const presetColors = [
  "#EF4444", "#F59E0B", "#10B981", "#3B82F6",
  "#8B5CF6", "#EC4899", "#475569", "#2DD4A8",
];

onMounted(() => {
  if (props.group) {
    name.value = props.group.name;
    color.value = props.group.color;
  }
});

async function save() {
  error.value = "";
  saving.value = true;

  try {
    if (isEditing) {
      await store.updateGroup(props.group!.id, name.value, color.value);
    } else {
      await store.createGroup(name.value, color.value);
    }
    emit("saved");
    emit("close");
  } catch (e) {
    error.value = String(e);
  } finally {
    saving.value = false;
  }
}
</script>

<template>
  <Transition name="panel">
    <div
      class="fixed inset-0 z-50 flex justify-end"
      @click.self="emit('close')"
    >
      <div class="absolute inset-0 bg-black/60" @click="emit('close')"></div>
      <div class="relative w-full max-w-sm h-full bg-otter-card border-l border-otter-border
                  overflow-y-auto animate-slide-in-right">
        <div class="p-6">
          <div class="flex items-center justify-between mb-6">
            <h2 class="text-sm font-semibold text-otter-muted uppercase tracking-wider">
              {{ isEditing ? "Edit Group" : "New Group" }}
            </h2>
            <button
              class="text-otter-subtle hover:text-otter-text transition-colors text-lg"
              @click="emit('close')"
            >
              &times;
            </button>
          </div>

          <div class="flex flex-col gap-3">
            <input
              v-model="name"
              class="px-3 py-2 rounded-lg bg-otter-surface border border-otter-border
                     text-otter-text placeholder-otter-subtle text-sm
                     focus:outline-none focus:border-otter-teal-dim transition-colors"
              placeholder="Group name (e.g. Production)"
              @keyup.enter="save"
            />

            <!-- Color palette -->
            <div>
              <label class="text-xs text-otter-subtle uppercase tracking-wider">
                Color
              </label>
              <div class="flex gap-2 mt-1">
                <button
                  v-for="c in presetColors"
                  :key="c"
                  class="w-6 h-6 rounded-full transition-transform hover:scale-110"
                  :class="color === c ? 'ring-2 ring-white ring-offset-2 ring-offset-otter-card' : ''"
                  :style="{ backgroundColor: c }"
                  @click="color = c"
                ></button>
              </div>
            </div>

            <button
              class="w-full py-2 rounded-lg bg-otter-teal text-otter-dark font-semibold
                     text-sm hover:opacity-90 transition-opacity disabled:opacity-50"
              :disabled="saving || !name"
              @click="save"
            >
              {{ saving ? "Saving..." : isEditing ? "Update Group" : "Create Group" }}
            </button>
          </div>

          <p v-if="error" class="mt-3 text-sm text-otter-coral break-all">
            {{ error }}
          </p>
        </div>
      </div>
    </div>
  </Transition>
</template>

<style scoped>
@keyframes slide-in-right {
  from { transform: translateX(100%); }
  to { transform: translateX(0); }
}
.animate-slide-in-right {
  animation: slide-in-right 0.25s ease-out;
}
.panel-leave-active .animate-slide-in-right {
  animation: slide-in-right 0.2s ease-in reverse;
}
.panel-enter-active {
  transition: opacity 0.25s ease-out;
}
.panel-leave-active {
  transition: opacity 0.2s ease-in;
}
.panel-enter-from,
.panel-leave-to {
  opacity: 0;
}
</style>
