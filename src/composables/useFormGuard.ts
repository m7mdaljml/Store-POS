import { computed, ref, unref, type Ref } from "vue";
import { i18n } from "../i18n";

/**
 * Tracks whether a reactive form object has been modified since the last
 * baseline capture. Accepts either a `ref({...})` form or a `reactive({...})`
 * object. Call `capture()` right after (re)filling the form (openAdd /
 * openEdit / initial async load), and `markSaved()` after a successful save
 * so the form is considered clean again.
 */
export function useFormGuard<T extends object>(form: Ref<T> | T) {
  const baseline = ref(JSON.stringify(unref(form)));

  function capture() {
    baseline.value = JSON.stringify(unref(form));
  }

  function markSaved() {
    capture();
  }

  const isDirty = computed(
    () => JSON.stringify(unref(form)) !== baseline.value,
  );

  return { isDirty, capture, markSaved };
}

type FieldRules = Array<[field: string, valid: boolean, label: string]>;

/**
 * Validates required-field rules and writes localized messages into the
 * shared errors object. Returns true when every rule passes. Fields that
 * pass have their previous error cleared; failing fields keep/replace it.
 */
export function applyFieldRules(
  errors: Record<string, string>,
  rules: FieldRules,
): boolean {
  let ok = true;
  for (const [field, valid, label] of rules) {
    if (valid) {
      delete errors[field];
      continue;
    }
    errors[field] = i18n.global.t("common.fieldRequired", { field: label });
    ok = false;
  }
  return ok;
}

export function clearFieldError(errors: Record<string, string>, field: string) {
  delete errors[field];
}
