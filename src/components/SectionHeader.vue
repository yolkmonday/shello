<script setup lang="ts">
const props = defineProps<{
  title: string;
  open: boolean;
  count?: number;
}>();

const emit = defineEmits<{
  toggle: [];
}>();
</script>

<template>
  <button
    class="flex items-center gap-2 w-full py-1.5 group cursor-pointer select-none"
    @click="emit('toggle')"
  >
    <svg
      class="w-3 h-3 text-otter-subtle transition-transform duration-200"
      :class="{ 'rotate-0': open, '-rotate-90': !open }"
      viewBox="0 0 12 12"
      fill="none"
      stroke="currentColor"
      stroke-width="1.5"
    >
      <path d="M3 4.5l3 3 3-3" />
    </svg>
    <span
      class="text-xs text-otter-subtle uppercase tracking-wider group-hover:text-otter-muted transition-colors"
    >
      {{ title }}
    </span>
    <span
      v-if="count !== undefined && count > 0"
      class="ml-1 px-1.5 py-0.5 rounded-full text-[10px] font-medium
             bg-otter-surface text-otter-muted"
    >
      {{ count }}
    </span>
  </button>
</template>
