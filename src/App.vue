<script setup lang="ts">
import { onMounted } from "vue";
import { useProfilesStore } from "./stores/profiles";
import TerminalLayout from "./components/TerminalLayout.vue";

const profilesStore = useProfilesStore();

onMounted(async () => {
  await profilesStore.loadAll();
});
</script>

<template>
  <TerminalLayout />
</template>
