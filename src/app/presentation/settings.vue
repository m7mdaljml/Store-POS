<template>
  <div class="d-flex flex-wrap align-items-center gap-2 mb-1">
    <h1 class="h3 mb-0">{{ t("settings.title") }}</h1>
  </div>
  <p class="text-muted mb-3">{{ t("settings.subtitle") }}</p>

  <ul class="nav nav-pills pos-tabs mb-3">
    <li v-for="tabDef in tabs" :key="tabDef.id" class="nav-item">
      <button
        type="button"
        class="nav-link"
        :class="{ active: tab === tabDef.id }"
        @click="tab = tabDef.id"
      >
        <i class="bi me-1" :class="tabDef.icon"></i>{{ t(tabDef.label) }}
      </button>
    </li>
  </ul>

  <div v-if="error" class="alert alert-danger py-2 small" role="alert">{{ error }}</div>
  <div v-if="notice" class="alert alert-success py-2 small" role="alert">{{ notice }}</div>

  <!-- F7.1 Store profile -->
  <div v-if="tab === 'store'" class="card">
    <div class="card-body">
      <div class="row g-3">
        <div class="col-md-6">
          <label class="form-label" for="set-name">{{ t("settings.storeName") }}</label>
          <input id="set-name" v-model="storeForm.name" class="form-control" type="text" />
        </div>
        <div class="col-md-6">
          <label class="form-label" for="set-phone">{{ t("settings.storePhone") }} {{ t("common.optional") }}</label>
          <input id="set-phone" v-model="storeForm.phone" class="form-control" type="text" />
        </div>
        <div class="col-12">
          <label class="form-label" for="set-address">{{ t("settings.storeAddress") }} {{ t("common.optional") }}</label>
          <textarea id="set-address" v-model="storeForm.address" class="form-control" rows="2"></textarea>
        </div>
        <div class="col-md-6">
          <label class="form-label" for="set-taxid">{{ t("settings.storeTaxId") }} {{ t("common.optional") }}</label>
          <input id="set-taxid" v-model="storeForm.taxId" class="form-control" type="text" />
        </div>
        <div class="col-md-6">
          <span class="form-label d-block">{{ t("settings.storeLogo") }}</span>
          <img v-if="settings.storeLogo" :src="settings.storeLogo" class="logo-preview mb-2 d-block" alt="" />
          <input ref="logoInput" class="d-none" type="file" accept="image/*" @change="onLogoPick" />
          <div class="btn-group btn-group-sm">
            <button type="button" class="btn btn-outline-secondary" @click="logoInput?.click()">
              <i class="bi bi-upload me-1"></i>{{ t("settings.uploadLogo") }}
            </button>
            <button
              v-if="settings.storeLogo"
              type="button"
              class="btn btn-outline-danger"
              @click="removeLogo"
            >
              {{ t("settings.removeLogo") }}
            </button>
          </div>
          <div v-if="logoError" class="text-danger small mt-1">{{ logoError }}</div>
        </div>
      </div>
      <div class="mt-3">
        <button type="button" class="btn btn-primary" :disabled="saving" @click="saveStore">
          {{ t("common.save") }}
        </button>
      </div>
    </div>
  </div>

  <!-- F7.2 Tax profiles -->
  <div v-if="tab === 'tax'" class="card">
    <div class="card-header d-flex justify-content-between align-items-center">
      <span>{{ taxProfiles.length ? t("common.results", { count: taxProfiles.length }) : t("settings.noProfiles") }}</span>
      <button type="button" class="btn btn-primary btn-sm" @click="openTaxModal()">
        <i class="bi bi-plus-lg me-1"></i>{{ t("settings.addProfile") }}
      </button>
    </div>
    <div class="table-responsive">
      <table class="table table-sm table-striped align-middle mb-0">
        <thead>
          <tr>
            <th>{{ t("settings.profileName") }}</th>
            <th class="text-end">{{ t("settings.ratePercent") }}</th>
            <th class="text-center">{{ t("settings.isDefault") }}</th>
            <th class="text-end">{{ t("common.actions") }}</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="p in taxProfiles" :key="p.id">
            <td>{{ p.name }}</td>
            <td class="text-end">{{ p.rate }}%</td>
            <td class="text-center">
              <span v-if="p.is_default" class="badge text-bg-success">{{ t("settings.isBase") }}</span>
            </td>
            <td class="text-end text-nowrap">
              <button type="button" class="btn btn-sm btn-outline-secondary me-1" @click="openTaxModal(p)">
                <i class="bi bi-pencil"></i>
              </button>
              <button
                v-if="!p.is_default"
                type="button"
                class="btn btn-sm btn-outline-secondary me-1"
                :title="t('settings.setDefault')"
                @click="makeTaxDefault(p.id)"
              >
                <i class="bi bi-star"></i>
              </button>
              <button type="button" class="btn btn-sm btn-outline-danger" @click="deleteTaxProfile(p)">
                <i class="bi bi-trash"></i>
              </button>
            </td>
          </tr>
        </tbody>
      </table>
    </div>
  </div>

  <!-- F7.3 Currencies -->
  <div v-if="tab === 'currency'" class="card">
    <div class="card-header d-flex justify-content-between align-items-center">
      <span>{{ currencies.length ? t("common.results", { count: currencies.length }) : t("settings.noCurrencies") }}</span>
      <button type="button" class="btn btn-primary btn-sm" @click="openCurrencyModal()">
        <i class="bi bi-plus-lg me-1"></i>{{ t("settings.addCurrency") }}
      </button>
    </div>
    <div class="table-responsive">
      <table class="table table-sm table-striped align-middle mb-0">
        <thead>
          <tr>
            <th>{{ t("settings.currencyCode") }}</th>
            <th>{{ t("settings.currencyName") }}</th>
            <th>{{ t("settings.currencySymbol") }}</th>
            <th class="text-end">{{ t("settings.exchangeRate") }}</th>
            <th class="text-center">{{ t("settings.isBase") }}</th>
            <th class="text-end">{{ t("common.actions") }}</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="c in currencies" :key="c.id">
            <td class="fw-semibold">{{ c.code }}</td>
            <td>{{ c.name }}</td>
            <td>{{ c.symbol }}</td>
            <td class="text-end">{{ c.rate }}</td>
            <td class="text-center">
              <span v-if="c.is_base" class="badge text-bg-success">{{ t("settings.isBase") }}</span>
            </td>
            <td class="text-end text-nowrap">
              <button type="button" class="btn btn-sm btn-outline-secondary me-1" @click="openCurrencyModal(c)">
                <i class="bi bi-pencil"></i>
              </button>
              <button
                v-if="!c.is_base"
                type="button"
                class="btn btn-sm btn-outline-secondary me-1"
                :title="t('settings.setBase')"
                @click="setBaseCurrency(c)"
              >
                <i class="bi bi-star"></i>
              </button>
              <button
                type="button"
                class="btn btn-sm btn-outline-danger"
                :disabled="c.is_base === 1"
                @click="deleteCurrency(c)"
              >
                <i class="bi bi-trash"></i>
              </button>
            </td>
          </tr>
        </tbody>
      </table>
    </div>
    <div class="card-footer small text-muted">{{ t("settings.rateHint") }}</div>
  </div>

  <!-- F7.4 Receipt customization -->
  <div v-if="tab === 'receipt'" class="card">
    <div class="card-body">
      <div class="row g-3">
        <div class="col-12">
          <label class="form-label" for="r-header">{{ t("settings.receiptHeader") }}</label>
          <input id="r-header" v-model="receiptForm.header" class="form-control" type="text" />
          <div class="form-text">{{ t("settings.receiptHeaderHint") }}</div>
        </div>
        <div class="col-12">
          <label class="form-label" for="r-footer">{{ t("settings.receiptFooter") }}</label>
          <textarea
            id="r-footer"
            v-model="receiptForm.footer"
            class="form-control"
            rows="2"
            :placeholder="t('settings.footerPlaceholder')"
          ></textarea>
        </div>
        <div class="col-md-4">
          <label class="form-label" for="r-logo-pos">{{ t("settings.logoPosition") }}</label>
          <select id="r-logo-pos" v-model="receiptForm.logoPos" class="form-select">
            <option value="top">{{ t("settings.posTop") }}</option>
            <option value="bottom">{{ t("settings.posBottom") }}</option>
          </select>
        </div>
        <div class="col-md-8">
          <span class="form-label d-block">{{ t("settings.paperFormat") }}</span>
          <div class="btn-group">
            <button
              type="button"
              class="btn btn-outline-secondary"
              :class="{ active: receiptForm.format === 'thermal' }"
              @click="receiptForm.format = 'thermal'"
            >
              {{ t("settings.formatThermal") }}
            </button>
            <button
              type="button"
              class="btn btn-outline-secondary"
              :class="{ active: receiptForm.format === 'a4' }"
              @click="receiptForm.format = 'a4'"
            >
              {{ t("settings.formatA4") }}
            </button>
          </div>
        </div>
      </div>
      <div class="mt-3">
        <button type="button" class="btn btn-primary" :disabled="saving" @click="saveReceipt">
          {{ t("common.save") }}
        </button>
      </div>
    </div>
  </div>

  <!-- F7.5 App preferences -->
  <div v-if="tab === 'prefs'" class="card">
    <div class="card-body">
      <div class="row g-4">
        <div class="col-md-4">
          <span class="form-label d-block fw-semibold">{{ t("settings.theme") }}</span>
          <div class="btn-group">
            <button
              type="button"
              class="btn btn-outline-secondary"
              :class="{ active: theme.theme === 'dark' }"
              @click="theme.set('dark')"
            >
              <i class="bi bi-moon me-1"></i>{{ t("settings.themeDark") }}
            </button>
            <button
              type="button"
              class="btn btn-outline-secondary"
              :class="{ active: theme.theme === 'light' }"
              @click="theme.set('light')"
            >
              <i class="bi bi-sun me-1"></i>{{ t("settings.themeLight") }}
            </button>
          </div>
        </div>
        <div class="col-md-4">
          <label class="form-label fw-semibold" for="pref-lang">{{ t("settings.language") }}</label>
          <select id="pref-lang" class="form-select" :value="locale" @change="onLangChange">
            <option value="en">English</option>
            <option value="ar">العربية</option>
          </select>
        </div>
        <div class="col-md-4">
          <span class="form-label d-block fw-semibold">{{ t("settings.sounds") }}</span>
          <div class="form-check form-switch">
            <input
              id="pref-sound"
              class="form-check-input"
              type="checkbox"
              role="switch"
              :checked="settings.soundEnabled"
              @change="onSoundToggle"
            />
            <label class="form-check-label small text-muted" for="pref-sound">
              {{ t("settings.soundsHint") }}
            </label>
          </div>
        </div>
      </div>
    </div>
  </div>

  <!-- Tax profile modal -->
  <div v-if="showTaxModal" class="modal-backdrop show"></div>
  <div v-if="showTaxModal" class="modal d-block" tabindex="-1">
    <div class="modal-dialog modal-dialog-centered">
      <div class="modal-content">
        <div class="modal-header">
          <h5 class="modal-title">{{ taxEditingId ? t("common.edit") : t("settings.addProfile") }}</h5>
          <button type="button" class="btn-close" @click="showTaxModal = false"></button>
        </div>
        <div class="modal-body">
          <div class="mb-3">
            <label class="form-label" for="tax-name">{{ t("settings.profileName") }}</label>
            <input id="tax-name" v-model="taxForm.name" class="form-control" type="text" />
          </div>
          <div class="mb-3">
            <label class="form-label" for="tax-rate">{{ t("settings.ratePercent") }}</label>
            <input id="tax-rate" v-model.number="taxForm.rate" class="form-control" type="number" min="0" max="100" step="0.01" />
          </div>
          <div class="form-check form-switch">
            <input id="tax-default" v-model="taxForm.isDefault" class="form-check-input" type="checkbox" role="switch" />
            <label class="form-check-label" for="tax-default">{{ t("settings.isDefault") }}</label>
          </div>
        </div>
        <div class="modal-footer">
          <button type="button" class="btn btn-outline-secondary" @click="showTaxModal = false">
            {{ t("common.cancel") }}
          </button>
          <button type="button" class="btn btn-primary" :disabled="saving" @click="saveTaxProfile">
            {{ t("common.save") }}
          </button>
        </div>
      </div>
    </div>
  </div>

  <!-- Currency modal -->
  <div v-if="showCurrencyModal" class="modal-backdrop show"></div>
  <div v-if="showCurrencyModal" class="modal d-block" tabindex="-1">
    <div class="modal-dialog modal-dialog-centered">
      <div class="modal-content">
        <div class="modal-header">
          <h5 class="modal-title">{{ currencyEditingId ? t("common.edit") : t("settings.addCurrency") }}</h5>
          <button type="button" class="btn-close" @click="showCurrencyModal = false"></button>
        </div>
        <div class="modal-body">
          <div class="row g-3">
            <div class="col-6">
              <label class="form-label" for="cur-code">{{ t("settings.currencyCode") }}</label>
              <input
                v-model="currencyForm.code"
                class="form-control text-uppercase"
                type="text"
                maxlength="5"
              />
            </div>
            <div class="col-6">
              <label class="form-label" for="cur-symbol">{{ t("settings.currencySymbol") }}</label>
              <input id="cur-symbol" v-model="currencyForm.symbol" class="form-control" type="text" maxlength="4" />
            </div>
            <div class="col-12">
              <label class="form-label" for="cur-name">{{ t("settings.currencyName") }}</label>
              <input id="cur-name" v-model="currencyForm.name" class="form-control" type="text" />
            </div>
            <div class="col-12">
              <label class="form-label" for="cur-rate">{{ t("settings.exchangeRate") }}</label>
              <input
                id="cur-rate"
                v-model.number="currencyForm.rate"
                class="form-control"
                type="number"
                min="0.000001"
                step="0.0001"
              />
            </div>
          </div>
        </div>
        <div class="modal-footer">
          <button type="button" class="btn btn-outline-secondary" @click="showCurrencyModal = false">
            {{ t("common.cancel") }}
          </button>
          <button type="button" class="btn btn-primary" :disabled="saving" @click="saveCurrency">
            {{ t("common.save") }}
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { onMounted, reactive, ref } from "vue";
import { useI18n } from "vue-i18n";
import { execute, insert, select, selectOne } from "../../lib/db";
import { useSettingsStore } from "../../stores/settings";
import { useThemeStore } from "../../stores/theme";
import { setLocale, type Locale } from "../../i18n";
import type { Currency, TaxProfile } from "../../types";

const { t, locale } = useI18n();
const settings = useSettingsStore();
const theme = useThemeStore();

type TabId = "store" | "tax" | "currency" | "receipt" | "prefs";
const tabs: Array<{ id: TabId; label: string; icon: string }> = [
  { id: "store", label: "settings.tabStore", icon: "bi-shop" },
  { id: "tax", label: "settings.tabTax", icon: "bi-percent" },
  { id: "currency", label: "settings.tabCurrency", icon: "bi-cash-coin" },
  { id: "receipt", label: "settings.tabReceipt", icon: "bi-receipt" },
  { id: "prefs", label: "settings.tabPrefs", icon: "bi-sliders" },
];
const tab = ref<TabId>("store");

const error = ref("");
const notice = ref("");
let noticeTimer: ReturnType<typeof setTimeout> | undefined;
function flash(msg: string) {
  error.value = "";
  notice.value = msg;
  if (noticeTimer) clearTimeout(noticeTimer);
  noticeTimer = setTimeout(() => (notice.value = ""), 3000);
}
function fail(msg: string) {
  notice.value = "";
  error.value = msg;
}

/* ---------------- Store profile (F7.1) ---------------- */

const saving = ref(false);
const storeForm = reactive({ name: "", phone: "", address: "", taxId: "" });
const logoInput = ref<HTMLInputElement | null>(null);
const logoError = ref("");

onMounted(async () => {
  try {
    if (!settings.loaded) await settings.load();
  } catch (e: unknown) {
    fail(String(e));
    return;
  }
  storeForm.name = settings.storeName;
  storeForm.phone = settings.storePhone;
  storeForm.address = settings.storeAddress;
  storeForm.taxId = settings.taxId;
  receiptForm.header = settings.receiptHeader;
  receiptForm.footer = settings.receiptFooter;
  receiptForm.logoPos = settings.receiptLogoPos;
  receiptForm.format = settings.receiptFormat;
});

async function saveStore() {
  saving.value = true;
  try {
    await Promise.all([
      settings.setValue("store_name", storeForm.name.trim()),
      settings.setValue("store_phone", storeForm.phone.trim()),
      settings.setValue("store_address", storeForm.address.trim()),
      settings.setValue("tax_id", storeForm.taxId.trim()),
    ]);
    flash(t("settings.saved"));
  } catch (e: unknown) {
    fail(String(e));
  } finally {
    saving.value = false;
  }
}

function onLogoPick(e: Event) {
  const input = e.target as HTMLInputElement;
  const file = input.files?.[0];
  input.value = "";
  logoError.value = "";
  if (!file) return;
  if (file.size > 500 * 1024) {
    logoError.value = t("settings.logoTooLarge");
    return;
  }
  const reader = new FileReader();
  reader.onload = () => {
    void settings
      .setValue("store_logo", String(reader.result ?? ""))
      .then(() => flash(t("settings.saved")))
      .catch((err: unknown) => fail(String(err)));
  };
  reader.readAsDataURL(file);
}

function removeLogo() {
  void settings
    .setValue("store_logo", "")
    .then(() => flash(t("settings.saved")))
    .catch((err: unknown) => fail(String(err)));
}

/* ---------------- Tax profiles (F7.2) ---------------- */

const taxProfiles = ref<TaxProfile[]>([]);
const showTaxModal = ref(false);
const taxEditingId = ref<number | null>(null);
const taxForm = reactive({ name: "", rate: 0, isDefault: false });

async function loadTaxProfiles() {
  taxProfiles.value = await select<TaxProfile>(
    "SELECT id, name, rate, is_default FROM tax_profiles ORDER BY is_default DESC, name",
  );
}

function openTaxModal(p?: TaxProfile) {
  error.value = "";
  taxEditingId.value = p ? p.id : null;
  taxForm.name = p ? p.name : "";
  taxForm.rate = p ? p.rate : 0;
  taxForm.isDefault = p ? p.is_default === 1 : taxProfiles.value.every((x) => x.is_default !== 1);
  showTaxModal.value = true;
}

async function saveTaxProfile() {
  const name = taxForm.name.trim();
  if (!name) return;
  if (!(taxForm.rate >= 0 && taxForm.rate <= 100)) {
    fail(t("settings.rateInvalid"));
    return;
  }
  saving.value = true;
  try {
    let id = taxEditingId.value;
    if (id) {
      await execute("UPDATE tax_profiles SET name = ?, rate = ?, is_default = ? WHERE id = ?", [
        name,
        taxForm.rate,
        taxForm.isDefault ? 1 : 0,
        id,
      ]);
    } else {
      id = await insert("INSERT INTO tax_profiles (name, rate, is_default) VALUES (?, ?, ?)", [
        name,
        taxForm.rate,
        taxForm.isDefault ? 1 : 0,
      ]);
    }
    if (taxForm.isDefault && id !== null) {
      await execute(
        "UPDATE tax_profiles SET is_default = CASE WHEN id = ? THEN 1 ELSE 0 END",
        [id],
      );
    }
    await loadTaxProfiles();
    showTaxModal.value = false;
    flash(t("settings.saved"));
  } catch {
    fail(t("settings.duplicateEntry"));
  } finally {
    saving.value = false;
  }
}

async function makeTaxDefault(id: number) {
  await execute("UPDATE tax_profiles SET is_default = CASE WHEN id = ? THEN 1 ELSE 0 END", [id]);
  await loadTaxProfiles();
}

async function deleteTaxProfile(p: TaxProfile) {
  const used = await selectOne<{ n: number }>(
    "SELECT COUNT(*) AS n FROM products WHERE tax_profile_id = ?",
    [p.id],
  );
  if (used && used.n > 0) {
    fail(t("settings.deleteProfileBlocked", { name: p.name }));
    return;
  }
  await execute("DELETE FROM tax_profiles WHERE id = ?", [p.id]);
  await loadTaxProfiles();
  flash(t("settings.saved"));
}

/* ---------------- Currencies (F7.3) ---------------- */

const currencies = ref<Currency[]>([]);
const showCurrencyModal = ref(false);
const currencyEditingId = ref<number | null>(null);
const currencyForm = reactive({ code: "", name: "", symbol: "", rate: 1 });

async function loadCurrencies() {
  currencies.value = await select<Currency>(
    "SELECT id, code, name, symbol, rate, is_base FROM currencies ORDER BY is_base DESC, code",
  );
}

function openCurrencyModal(c?: Currency) {
  error.value = "";
  currencyEditingId.value = c ? c.id : null;
  currencyForm.code = c ? c.code : "";
  currencyForm.name = c ? c.name : "";
  currencyForm.symbol = c ? c.symbol : "";
  currencyForm.rate = c ? c.rate : 1;
  showCurrencyModal.value = true;
}

async function saveCurrency() {
  const code = currencyForm.code.trim().toUpperCase();
  const name = currencyForm.name.trim();
  const symbol = currencyForm.symbol.trim();
  if (!code || !name || !symbol) return;
  if (!(currencyForm.rate > 0)) {
    fail(t("settings.rateInvalid"));
    return;
  }
  saving.value = true;
  try {
    if (currencyEditingId.value) {
      await execute(
        "UPDATE currencies SET code = ?, name = ?, symbol = ?, rate = ? WHERE id = ?",
        [code, name, symbol, currencyForm.rate, currencyEditingId.value],
      );
    } else {
      await execute(
        "INSERT INTO currencies (code, name, symbol, rate, is_base) VALUES (?, ?, ?, ?, 0)",
        [code, name, symbol, currencyForm.rate],
      );
    }
    await loadCurrencies();
    showCurrencyModal.value = false;
    flash(t("settings.saved"));
  } catch {
    fail(t("settings.duplicateEntry"));
  } finally {
    saving.value = false;
  }
}

async function setBaseCurrency(c: Currency) {
  await execute("UPDATE currencies SET is_base = CASE WHEN id = ? THEN 1 ELSE 0 END", [c.id]);
  // Keep the display-currency setting in sync so fmt() uses the base code.
  await settings.setValue("currency", c.code);
  await loadCurrencies();
  flash(t("settings.saved"));
}

async function deleteCurrency(c: Currency) {
  if (c.is_base === 1) {
    fail(t("settings.deleteBaseBlocked"));
    return;
  }
  await execute("DELETE FROM currencies WHERE id = ?", [c.id]);
  await loadCurrencies();
  flash(t("settings.saved"));
}

/* ---------------- Receipt (F7.4) ---------------- */

const receiptForm = reactive({
  header: "",
  footer: "",
  logoPos: "top" as "top" | "bottom",
  format: "thermal" as "thermal" | "a4",
});

async function saveReceipt() {
  saving.value = true;
  try {
    await Promise.all([
      settings.setValue("receipt_header", receiptForm.header.trim()),
      settings.setValue("receipt_footer", receiptForm.footer.trim()),
      settings.setValue("receipt_logo_pos", receiptForm.logoPos),
      settings.setValue("receipt_format", receiptForm.format),
    ]);
    flash(t("settings.saved"));
  } catch (e: unknown) {
    fail(String(e));
  } finally {
    saving.value = false;
  }
}

/* ---------------- Preferences (F7.5) ---------------- */

function onLangChange(e: Event) {
  setLocale((e.target as HTMLSelectElement).value as Locale);
  flash(t("settings.saved"));
}

function onSoundToggle(e: Event) {
  const on = (e.target as HTMLInputElement).checked;
  void settings
    .setValue("sound_enabled", on ? "1" : "0")
    .catch((err: unknown) => fail(String(err)));
}
</script>

<style scoped>
.logo-preview {
  height: 56px;
  width: auto;
  max-width: 160px;
  object-fit: contain;
  border-radius: 6px;
  border: 1px solid var(--pos-border);
  background: #fff;
  padding: 2px;
}
</style>
