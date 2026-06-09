export interface RecipeVariable {
  key: string;
  label: string;
  placeholder: string;
  required: boolean;
  default?: string;
}

export interface RecipeStep {
  name: string;
  command: string;
  description: string;
  /** If true, failure of this step won't stop the recipe */
  optional?: boolean;
}

export interface Recipe {
  id: string;
  name: string;
  description: string;
  icon: string;
  tags: string[];
  os: string[];
  variables: RecipeVariable[];
  steps: RecipeStep[];
}

export type StepStatus = "pending" | "running" | "success" | "failed" | "skipped";
