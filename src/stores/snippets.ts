import { defineStore } from "pinia";
import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";

export interface Snippet {
  id: string;
  name: string;
  command: string;
  tags: string;
  sort_order: number;
  created_at: string;
  updated_at: string;
}

export const useSnippetsStore = defineStore("snippets", () => {
  const snippets = ref<Snippet[]>([]);

  async function loadAll() {
    snippets.value = await invoke<Snippet[]>("snippet_list");
  }

  async function create(name: string, command: string, tags?: string) {
    const snippet = await invoke<Snippet>("snippet_create", {
      input: { name, command, tags },
    });
    snippets.value.push(snippet);
    return snippet;
  }

  async function update(id: string, input: { name?: string; command?: string; tags?: string }) {
    const snippet = await invoke<Snippet>("snippet_update", { id, input });
    const idx = snippets.value.findIndex((s) => s.id === id);
    if (idx >= 0) snippets.value[idx] = snippet;
    return snippet;
  }

  async function remove(id: string) {
    await invoke("snippet_delete", { id });
    snippets.value = snippets.value.filter((s) => s.id !== id);
  }

  return { snippets, loadAll, create, update, remove };
});
