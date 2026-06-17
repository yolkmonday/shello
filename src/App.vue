<script setup lang="ts">
import { onMounted } from "vue";
import { useProfilesStore } from "./stores/profiles";
import { useUpdaterStore } from "./stores/updater";
import TerminalLayout from "./components/TerminalLayout.vue";
import UpdateToast from "./components/UpdateToast.vue";
import HostKeyDialog from "./components/HostKeyDialog.vue";

const profilesStore = useProfilesStore();
const updaterStore = useUpdaterStore();

onMounted(async () => {
  await profilesStore.loadAll();
  // Auto-check for updates in the background (silent on failure / in dev).
  updaterStore.checkForUpdate();
});
</script>

<template>
  <TerminalLayout />
  <UpdateToast />
  <HostKeyDialog />
</template>
