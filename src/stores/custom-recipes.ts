import { defineStore } from "pinia";
import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { type Recipe, type RecipeVariable, type RecipeStep } from "../lib/recipe-types";

interface CustomRecipeRow {
  id: string;
  name: string;
  description: string;
  icon: string;
  tags: string;
  variables: string;
  steps: string;
  created_at: string;
  updated_at: string;
}

function rowToRecipe(row: CustomRecipeRow): Recipe {
  return {
    id: row.id,
    name: row.name,
    description: row.description,
    icon: row.icon,
    tags: row.tags ? row.tags.split(",").map((t) => t.trim()).filter(Boolean) : [],
    os: [],
    variables: JSON.parse(row.variables) as RecipeVariable[],
    steps: JSON.parse(row.steps) as RecipeStep[],
  };
}

export const useCustomRecipesStore = defineStore("custom-recipes", () => {
  const recipes = ref<Recipe[]>([]);

  async function loadAll() {
    const rows = await invoke<CustomRecipeRow[]>("custom_recipe_list");
    recipes.value = rows.map(rowToRecipe);
  }

  async function create(recipe: {
    name: string;
    description: string;
    icon: string;
    tags: string[];
    variables: RecipeVariable[];
    steps: RecipeStep[];
  }) {
    await invoke("custom_recipe_create", {
      input: {
        name: recipe.name,
        description: recipe.description,
        icon: recipe.icon,
        tags: recipe.tags.join(","),
        variables: JSON.stringify(recipe.variables),
        steps: JSON.stringify(recipe.steps),
      },
    });
    await loadAll();
  }

  async function update(
    id: string,
    recipe: {
      name: string;
      description: string;
      icon: string;
      tags: string[];
      variables: RecipeVariable[];
      steps: RecipeStep[];
    }
  ) {
    await invoke("custom_recipe_update", {
      id,
      input: {
        name: recipe.name,
        description: recipe.description,
        icon: recipe.icon,
        tags: recipe.tags.join(","),
        variables: JSON.stringify(recipe.variables),
        steps: JSON.stringify(recipe.steps),
      },
    });
    await loadAll();
  }

  async function remove(id: string) {
    await invoke("custom_recipe_delete", { id });
    await loadAll();
  }

  return { recipes, loadAll, create, update, remove };
});
