<script setup lang="ts">
import { ref } from "vue";
import type { PaneNode } from "../stores/terminal";
import TerminalView from "./TerminalView.vue";

const props = defineProps<{
  node: PaneNode;
}>();

const dragging = ref(false);
const containerRef = ref<HTMLDivElement>();

function startDrag(e: MouseEvent) {
  e.preventDefault();
  dragging.value = true;

  const onMove = (moveEvent: MouseEvent) => {
    if (props.node.type !== "split" || !containerRef.value) return;

    const rect = containerRef.value.getBoundingClientRect();
    let ratio: number;

    if (props.node.direction === "vertical") {
      ratio = (moveEvent.clientX - rect.left) / rect.width;
    } else {
      ratio = (moveEvent.clientY - rect.top) / rect.height;
    }

    props.node.ratio = Math.max(0.15, Math.min(0.85, ratio));
  };

  const onUp = () => {
    dragging.value = false;
    window.removeEventListener("mousemove", onMove);
    window.removeEventListener("mouseup", onUp);
  };

  window.addEventListener("mousemove", onMove);
  window.addEventListener("mouseup", onUp);
}
</script>

<template>
  <!-- Terminal leaf -->
  <TerminalView
    v-if="node.type === 'terminal'"
    :session-id="node.sessionId"
    class="w-full h-full"
  />

  <!-- Split container -->
  <div
    v-else
    ref="containerRef"
    class="w-full h-full flex"
    :class="node.direction === 'vertical' ? 'flex-row' : 'flex-col'"
  >
    <!-- First child -->
    <div
      :style="{
        [node.direction === 'vertical' ? 'width' : 'height']:
          node.ratio * 100 + '%',
      }"
      class="overflow-hidden"
    >
      <PaneContainer :node="node.children[0]" />
    </div>

    <!-- Divider -->
    <div
      :class="[
        'flex-shrink-0 bg-otter-border hover:bg-otter-teal-dim transition-colors',
        node.direction === 'vertical'
          ? 'w-1 cursor-col-resize'
          : 'h-1 cursor-row-resize',
        dragging ? 'bg-otter-teal' : '',
      ]"
      @mousedown="startDrag"
    ></div>

    <!-- Second child -->
    <div
      :style="{
        [node.direction === 'vertical' ? 'width' : 'height']:
          (1 - node.ratio) * 100 + '%',
      }"
      class="overflow-hidden"
    >
      <PaneContainer :node="node.children[1]" />
    </div>
  </div>
</template>
