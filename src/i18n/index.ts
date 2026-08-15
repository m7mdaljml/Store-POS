import { createI18n } from "vue-i18n";
import { en } from "./locales/en";
import { ar } from "./locales/ar";

export const SUPPORTED_LOCALES = ["en", "ar"] as const;
export type Locale = (typeof SUPPORTED_LOCALES)[number];

const LOCALE_KEY = "pos_locale";

function readStoredLocale(): Locale {
  const stored = localStorage.getItem(LOCALE_KEY);
  return stored === "ar" ? "ar" : "en";
}

const locale = readStoredLocale();

function arPluralRule(choice: number): number {
  const category = new Intl.PluralRules("ar").select(choice);
  switch (category) {
    case "zero":
      return 0;
    case "one":
      return 1;
    case "two":
      return 2;
    case "few":
      return 3;
    case "many":
      return 4;
    default:
      return 5;
  }
}

export const i18n = createI18n({
  legacy: false,
  locale,
  fallbackLocale: "en",
  messages: { en, ar },
  pluralRules: { ar: arPluralRule },
});

function applyDirection(l: Locale) {
  document.documentElement.setAttribute("lang", l);
  document.documentElement.setAttribute("dir", l === "ar" ? "rtl" : "ltr");
}
applyDirection(locale);

export function setLocale(l: Locale) {
  i18n.global.locale.value = l;
  localStorage.setItem(LOCALE_KEY, l);
  applyDirection(l);
}

export function toggleLocale(): Locale {
  const next: Locale = i18n.global.locale.value === "en" ? "ar" : "en";
  setLocale(next);
  return next;
}

