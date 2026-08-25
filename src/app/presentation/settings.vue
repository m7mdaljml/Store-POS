<template>
  <div class="d-flex align-items-center justify-content-between mb-3">
    <h1 class="h4 mb-0">{{ t("settings.title") }}</h1>
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
        <i class="bi mx-1" :class="tabDef.icon"></i>{{ t(tabDef.label) }}
      </button>
    </li>
  </ul>

  <!-- F7.1 Store profile -->
  <div v-if="tab === 'store'" class="card">
    <div class="card-body">
      <div class="row g-3">
        <div class="col-md-6">
          <label class="form-label" for="set-name">{{
            t("settings.storeName")
          }}</label>
          <input
            id="set-name"
            v-model="storeForm.name"
            class="form-control"
            :class="{ 'is-invalid': !!storeErrors.name }"
            type="text"
            @input="clearFieldError(storeErrors, 'name')"
          />
          <div v-if="storeErrors.name" class="invalid-feedback">
            {{ storeErrors.name }}
          </div>
        </div>
        <div class="col-md-6">
          <label class="form-label" for="set-phone"
            >{{ t("settings.storePhone") }} {{ t("common.optional") }}</label
          >
          <input
            id="set-phone"
            v-model="storeForm.phone"
            class="form-control"
            type="text"
          />
        </div>
        <div class="col-12">
          <label class="form-label" for="set-address"
            >{{ t("settings.storeAddress") }} {{ t("common.optional") }}</label
          >
          <textarea
            id="set-address"
            v-model="storeForm.address"
            class="form-control"
            rows="2"
          ></textarea>
        </div>
        <div class="col-md-6">
          <label class="form-label" for="set-taxid"
            >{{ t("settings.storeTaxId") }} {{ t("common.optional") }}</label
          >
          <input
            id="set-taxid"
            v-model="storeForm.taxId"
            class="form-control"
            type="text"
          />
        </div>
        <div class="col-md-6">
          <span class="form-label d-block">{{ t("settings.storeLogo") }}</span>
          <img
            v-if="settings.storeLogo"
            :src="settings.storeLogo"
            class="logo-preview mb-2 d-block"
            alt=""
          />
          <input
            ref="logoInput"
            class="d-none"
            type="file"
            accept="image/*"
            @change="onLogoPick"
          />
          <div class="btn-group btn-group-sm">
            <button
              type="button"
              class="btn btn-outline-secondary"
              @click="logoInput?.click()"
            >
              <i class="bi bi-upload mx-1"></i>{{ t("settings.uploadLogo") }}
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
          <div v-if="logoError" class="text-danger small mt-1">
            {{ logoError }}
          </div>
        </div>
      </div>
      <div class="mt-3">
        <AsyncButton
          variant="primary"
          :loading="saving"
          :disabled="!canSaveStore"
          @click="saveStore"
        >
          {{ t("common.save") }}
        </AsyncButton>
      </div>
    </div>
  </div>

  <!-- F7.2 Tax profiles -->
  <div v-if="tab === 'tax'" class="card">
    <div class="card-header d-flex justify-content-between align-items-center">
      <span>{{
        taxProfiles.length
          ? t("common.results", { count: taxProfiles.length })
          : t("settings.noProfiles")
      }}</span>
      <button
        type="button"
        class="btn btn-primary btn-sm"
        @click="openTaxModal()"
      >
        <i class="bi bi-plus-lg mx-1"></i>{{ t("settings.addProfile") }}
      </button>
    </div>
    <div class="table-responsive">
      <table class="table table-sm table-striped align-middle mb-0">
        <thead v-if="taxProfiles.length">
          <tr>
            <th>{{ t("settings.profileName") }}</th>
            <th class="text-end">{{ t("settings.ratePercent") }}</th>
            <th class="text-center">{{ t("settings.isDefault") }}</th>
            <th class="text-end">{{ t("common.actions") }}</th>
          </tr>
        </thead>
        <tbody v-if="!taxProfiles.length">
          <tr>
            <td colspan="4" class="p-0 border-0">
              <EmptyState
                :image="emptyExpenses"
                :message="t('settings.noProfiles')"
              />
            </td>
          </tr>
        </tbody>
        <tbody v-for="p in taxProfiles" :key="p.id">
          <tr>
            <td>{{ p.name }}</td>
            <td class="text-end">{{ p.rate }}%</td>
            <td class="text-center">
              <span v-if="p.is_default" class="badge text-bg-success">{{
                t("settings.isBase")
              }}</span>
            </td>
            <td class="text-end text-nowrap">
              <button
                type="button"
                class="btn btn-sm btn-outline-secondary mx-1"
                @click="openTaxModal(p)"
              >
                <i class="bi bi-pencil"></i>
              </button>
              <button
                v-if="!p.is_default"
                type="button"
                class="btn btn-sm btn-outline-secondary mx-1"
                :title="t('settings.setDefault')"
                @click="makeTaxDefault(p.id)"
              >
                <i class="bi bi-star"></i>
              </button>
              <button
                type="button"
                class="btn btn-sm btn-outline-danger"
                @click="deleteTaxProfile(p)"
              >
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
      <span>{{
        currencies.length
          ? t("common.results", { count: currencies.length })
          : t("settings.noCurrencies")
      }}</span>
      <button
        type="button"
        class="btn btn-primary btn-sm"
        @click="openCurrencyModal()"
      >
        <i class="bi bi-plus-lg mx-1"></i>{{ t("settings.addCurrency") }}
      </button>
    </div>
    <div class="table-responsive">
      <table class="table table-sm table-striped align-middle mb-0">
        <thead v-if="currencies.length">
          <tr>
            <th>{{ t("settings.currencyCode") }}</th>
            <th>{{ t("settings.currencyName") }}</th>
            <th>{{ t("settings.currencySymbol") }}</th>
            <th class="text-end">{{ t("settings.exchangeRate") }}</th>
            <th class="text-center">{{ t("settings.isBase") }}</th>
            <th class="text-end">{{ t("common.actions") }}</th>
          </tr>
        </thead>
        <tbody v-if="!currencies.length">
          <tr>
            <td colspan="6" class="p-0 border-0">
              <EmptyState
                :image="emptyExpenses"
                :message="t('settings.noCurrencies')"
              />
            </td>
          </tr>
        </tbody>
        <tbody v-for="c in currencies" :key="c.id">
          <tr>
            <td class="fw-semibold">{{ c.code }}</td>
            <td>{{ c.name }}</td>
            <td>{{ c.symbol }}</td>
            <td class="text-end">{{ c.rate }}</td>
            <td class="text-center">
              <span v-if="c.is_base" class="badge text-bg-success">{{
                t("settings.isBase")
              }}</span>
            </td>
            <td class="text-end text-nowrap">
              <button
                type="button"
                class="btn btn-sm btn-outline-secondary mx-1"
                @click="openCurrencyModal(c)"
              >
                <i class="bi bi-pencil"></i>
              </button>
              <button
                v-if="!c.is_base"
                type="button"
                class="btn btn-sm btn-outline-secondary mx-1"
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
  </div>

  <!-- F7.4 Receipt customization -->
  <div v-if="tab === 'receipt'" class="card">
    <div class="card-body">
      <div class="row g-3">
        <div class="col-12">
          <label class="form-label" for="r-header">{{
            t("settings.receiptHeader")
          }}</label>
          <input
            id="r-header"
            v-model="receiptForm.header"
            class="form-control"
            type="text"
          />
          <div class="form-text">{{ t("settings.receiptHeaderHint") }}</div>
        </div>
        <div class="col-12">
          <label class="form-label" for="r-footer">{{
            t("settings.receiptFooter")
          }}</label>
          <textarea
            id="r-footer"
            v-model="receiptForm.footer"
            class="form-control"
            rows="2"
            :placeholder="t('settings.footerPlaceholder')"
          ></textarea>
        </div>
        <div class="col-md-4">
          <label class="form-label" for="r-logo-pos">{{
            t("settings.logoPosition")
          }}</label>
          <select
            id="r-logo-pos"
            v-model="receiptForm.logoPos"
            class="form-select"
          >
            <option value="top">{{ t("settings.posTop") }}</option>
            <option value="bottom">{{ t("settings.posBottom") }}</option>
          </select>
        </div>
        <div class="col-md-8">
          <span class="form-label d-block">{{
            t("settings.paperFormat")
          }}</span>
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
        <AsyncButton
          variant="primary"
          :loading="saving"
          :disabled="!canSaveReceipt"
          @click="saveReceipt"
        >
          {{ t("common.save") }}
        </AsyncButton>
      </div>
    </div>
  </div>

  <!-- F7.5 App preferences -->
  <div v-if="tab === 'prefs'" class="card">
    <div class="card-body">
      <div class="row g-4">
        <div class="col-md-4">
          <span class="form-label d-block fw-semibold">{{
            t("settings.theme")
          }}</span>
          <div class="btn-group">
            <button
              type="button"
              class="btn btn-outline-secondary"
              :class="{ active: theme.theme === 'dark' }"
              @click="theme.set('dark')"
            >
              <i class="bi bi-moon mx-1"></i>{{ t("settings.themeDark") }}
            </button>
            <button
              type="button"
              class="btn btn-outline-secondary"
              :class="{ active: theme.theme === 'light' }"
              @click="theme.set('light')"
            >
              <i class="bi bi-sun mx-1"></i>{{ t("settings.themeLight") }}
            </button>
          </div>
        </div>
        <div class="col-md-4">
          <label class="form-label fw-semibold" for="pref-lang">{{
            t("settings.language")
          }}</label>
          <select
            id="pref-lang"
            class="form-select"
            :value="locale"
            @change="onLangChange"
          >
            <option value="en">English</option>
            <option value="ar">العربية</option>
          </select>
        </div>
        <div class="col-md-4">
          <label class="form-label fw-semibold" for="pref-page-size">{{
            t("settings.pageSize")
          }}</label>
          <select
            id="pref-page-size"
            class="form-select"
            :value="settings.pageSize"
            @change="onPageSizeChange"
          >
            <option v-for="opt in PAGE_SIZE_OPTIONS" :key="opt" :value="opt">
              {{ opt }}
            </option>
          </select>
          <div class="form-text">{{ t("settings.pageSizeHint") }}</div>
        </div>
        <div class="col-md-4">
          <span class="form-label d-block fw-semibold">{{
            t("settings.sounds")
          }}</span>
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

  <!-- F8.1–F8.5 Backups & data safety -->
  <template v-if="tab === 'backup'">
    <div class="row g-3 mb-3">
      <div class="col-md-6">
        <div class="card h-100">
          <div class="card-body">
            <h6 class="card-title mb-1">{{ t("settings.integrityTitle") }}</h6>
            <p class="text-muted small mb-2">
              {{ t("settings.integrityHint") }}
            </p>
            <div
              v-if="integrity"
              class="small mb-2"
              :class="integrity.ok ? 'text-success' : 'text-danger'"
            >
              <i
                class="bi mx-1"
                :class="
                  integrity.ok ? 'bi-check-circle' : 'bi-exclamation-triangle'
                "
              ></i>
              {{ integrity.text }}
            </div>
            <AsyncButton
              size="sm"
              variant="outline-secondary"
              :loading="checkingIntegrity"
              @click="runIntegrityCheck"
            >
              {{ t("settings.runCheck") }}
            </AsyncButton>
          </div>
        </div>
      </div>
      <div class="col-md-6">
        <div class="card h-100">
          <div class="card-body">
            <h6 class="card-title mb-1">{{ t("settings.manualTitle") }}</h6>
            <p class="text-muted small mb-2">{{ backupDirDisplay }}</p>
            <div class="btn-group btn-group-sm">
              <button
                type="button"
                class="btn btn-outline-secondary"
                @click="chooseFolder"
              >
                {{ t("settings.chooseFolder") }}
              </button>
              <AsyncButton
                size="sm"
                variant="primary"
                :loading="backingUp"
                @click="backupNow"
              >
                <i v-if="!backingUp" class="bi bi-database-add mx-1"></i
                >{{ t("settings.backupNow") }}
              </AsyncButton>
              <AsyncButton
                size="sm"
                variant="outline-primary"
                :loading="restoringFile"
                @click="restoreFromFile"
              >
                <i
                  v-if="!restoringFile"
                  class="bi bi-box-arrow-in-down mx-1"
                ></i
                >{{ t("settings.restoreFromFile") }}
              </AsyncButton>
            </div>
          </div>
        </div>
      </div>
    </div>

    <div class="card mb-3">
      <div class="card-body">
        <h6 class="card-title mb-3">{{ t("settings.autoTitle") }}</h6>
        <div class="row g-3 align-items-end">
          <div class="col-md-4">
            <span class="form-label d-block">{{
              t("settings.autoTitle")
            }}</span>
            <div class="form-check form-switch">
              <input
                id="auto-backup"
                v-model="autoForm.enabled"
                class="form-check-input"
                type="checkbox"
                role="switch"
              />
              <label
                class="form-check-label small text-muted"
                for="auto-backup"
                >{{ t("settings.enabled") }}</label
              >
            </div>
          </div>
          <div class="col-md-4">
            <label class="form-label" for="auto-freq">{{
              t("settings.freqLabel")
            }}</label>
            <select id="auto-freq" v-model="autoForm.freq" class="form-select">
              <option value="daily">{{ t("settings.freqDaily") }}</option>
              <option value="weekly">{{ t("settings.freqWeekly") }}</option>
            </select>
          </div>
          <div class="col-md-2">
            <label class="form-label" for="auto-retention">{{
              t("settings.retentionLabel")
            }}</label>
            <input
              id="auto-retention"
              v-model.number="autoForm.retention"
              class="form-control"
              :class="{ 'is-invalid': !!autoErrors.retention }"
              type="number"
              min="1"
              max="100"
              @input="clearFieldError(autoErrors, 'retention')"
            />
            <div v-if="autoErrors.retention" class="invalid-feedback">
              {{ autoErrors.retention }}
            </div>
          </div>
          <div class="col-md-2">
            <AsyncButton
              variant="primary"
              :loading="saving"
              :disabled="!canSaveAuto"
              @click="saveAutoSettings"
            >
              {{ t("common.save") }}
            </AsyncButton>
          </div>
        </div>
      </div>
    </div>

    <div class="card mb-3">
      <div class="card-header">
        {{
          backups.length
            ? t("common.results", { count: backups.length })
            : t("settings.noBackups")
        }}
      </div>
      <div class="table-responsive">
        <table class="table table-sm table-striped align-middle mb-0">
          <thead v-if="backups.length">
            <tr>
              <th>{{ t("common.date") }}</th>
              <th>{{ t("common.type") }}</th>
              <th class="text-end">{{ t("settings.sizeCol") }}</th>
              <th>{{ t("settings.backupFolderLabel") }}</th>
              <th class="text-end">{{ t("common.actions") }}</th>
            </tr>
          </thead>
          <tbody v-if="!backups.length">
            <tr>
              <td colspan="5" class="p-0 border-0">
                <EmptyState
                  :image="emptyStock"
                  :message="t('settings.noBackups')"
                />
              </td>
            </tr>
          </tbody>
          <tbody v-for="b in backups" :key="b.id">
            <tr>
              <td>{{ fmtDateTime(b.createdAt) }}</td>
              <td>
                <span class="badge text-bg-secondary">{{ b.kind }}</span>
              </td>
              <td class="text-end">{{ fmtSize(b.sizeBytes) }}</td>
              <td
                class="text-truncate"
                style="max-width: 320px"
                :title="b.path"
              >
                {{ b.path }}
              </td>
              <td class="text-end text-nowrap">
                <button
                  type="button"
                  class="btn btn-sm btn-outline-primary mx-1"
                  @click="restore(b.path)"
                >
                  {{ t("settings.restore") }}
                </button>
                <button
                  type="button"
                  class="btn btn-sm btn-outline-danger"
                  @click="removeBackup(b.path)"
                >
                  <i class="bi bi-trash"></i>
                </button>
              </td>
            </tr>
          </tbody>
        </table>
      </div>
    </div>

    <div class="card" v-can="'export.excel'">
      <div class="card-body d-flex flex-wrap align-items-center gap-3">
        <div class="mx-auto">
          <h6 class="card-title mb-1">{{ t("settings.exportWorkbook") }}</h6>
          <p class="text-muted small mb-0">
            {{ t("settings.exportWorkbookHint") }}
          </p>
        </div>
        <AsyncButton
          variant="outline-success"
          :loading="saving"
          @click="exportWorkbook"
        >
          <i v-if="!saving" class="bi bi-file-earmark-spreadsheet mx-1"></i
          >{{ t("settings.exportWorkbook") }}
        </AsyncButton>
      </div>
    </div>
  </template>

  <!-- Open Register -->
  <div v-if="tab === 'register'" class="card">
    <div class="card-body">
      <template v-if="openSession">
        <div class="d-flex align-items-center gap-3 mb-3">
          <span class="badge text-bg-success">{{ t("sessions.open") }}</span>
          <span class="text-muted small">
            {{
              t(
                "checkout.registerOpen",
                {
                  openedAt: dateLabel(openSession.openedAt),
                  openingCash: fmt(openSession.openingCash),
                  count: openSession.salesCount,
                  salesTotal: fmt(openSession.salesTotal),
                },
                openSession.salesCount,
              )
            }}
          </span>
        </div>
        <button
          type="button"
          class="btn btn-primary"
          :disabled="registerBusy"
          @click="openCloseModal"
        >
          <i class="bi bi-box-arrow-right mx-1"></i
          >{{ t("checkout.closeRegister") }}
        </button>
      </template>
      <template v-else>
        <p class="text-muted mb-3">{{ t("checkout.registerClosed") }}</p>
        <button
          type="button"
          class="btn btn-primary"
          :disabled="registerBusy"
          @click="openOpenModal"
        >
          <i class="bi bi-box-arrow-in-right mx-1"></i
          >{{ t("checkout.openRegister") }}
        </button>
      </template>
    </div>
  </div>

  <!-- Open register modal -->
  <div v-if="showOpenModal" class="modal-backdrop show"></div>
  <div v-if="showOpenModal" class="modal d-block" tabindex="-1">
    <div class="modal-dialog modal-dialog-centered">
      <div class="modal-content">
        <div class="modal-header">
          <h5 class="modal-title">
            <i class="bi bi-cash-stack mx-2"></i
            >{{ t("checkout.openRegisterTitle") }}
          </h5>
          <button
            type="button"
            class="btn-close"
            @click="showOpenModal = false"
          ></button>
        </div>
        <div class="modal-body">
          <p class="small mb-3">{{ t("checkout.openRegisterBody") }}</p>
          <label class="form-label" for="set-opening-cash">{{
            t("checkout.openingCash")
          }}</label>
          <div class="input-group">
            <input
              id="set-opening-cash"
              v-model.number="openCashAmt"
              class="form-control text-end"
              type="number"
              min="0"
              step="1"
            />
            <span class="input-group-text">{{
              baseCurrencySymbol ||
                settings.currency ||
                t("checkout.currencyFallback")
            }}</span>
          </div>
          <div class="d-flex justify-content-end gap-2 mt-3 pt-3 border-top">
            <button
              type="button"
              class="btn btn-secondary"
              :disabled="registerBusy"
              @click="showOpenModal = false"
            >
              {{ t("common.cancel") }}
            </button>
            <AsyncButton
              variant="primary"
              :loading="registerBusy"
              @click="confirmOpen"
            >
              <i class="bi bi-box-arrow-in-right mx-1"></i
              >{{ t("checkout.openRegister") }}
            </AsyncButton>
          </div>
        </div>
      </div>
    </div>
  </div>

  <!-- Close register modal -->
  <div v-if="showCloseModal" class="modal-backdrop show"></div>
  <div v-if="showCloseModal" class="modal d-block" tabindex="-1">
    <div class="modal-dialog modal-dialog-centered">
      <div class="modal-content">
        <div class="modal-header">
          <h5 class="modal-title">
            <i class="bi bi-cash-stack mx-2"></i
            >{{ t("checkout.closeRegisterTitle") }}
          </h5>
          <button
            type="button"
            class="btn-close"
            @click="showCloseModal = false"
          ></button>
        </div>
        <div class="modal-body">
          <p class="small mb-3">
            {{
              t("checkout.closeRegisterBody", {
                opening: fmt(openSession?.openingCash ?? 0),
              })
            }}
          </p>
          <label class="form-label" for="set-closing-cash">{{
            t("checkout.closingCash")
          }}</label>
          <div class="input-group">
            <input
              id="set-closing-cash"
              v-model.number="closeCashAmt"
              class="form-control text-end"
              type="number"
              min="0"
              step="1"
            />
            <span class="input-group-text">{{
              baseCurrencySymbol ||
                settings.currency ||
                t("checkout.currencyFallback")
            }}</span>
          </div>
          <div class="d-flex justify-content-end gap-2 mt-3 pt-3 border-top">
            <button
              type="button"
              class="btn btn-secondary"
              :disabled="registerBusy"
              @click="showCloseModal = false"
            >
              {{ t("common.cancel") }}
            </button>
            <AsyncButton
              variant="primary"
              :loading="registerBusy"
              @click="confirmClose"
            >
              <i class="bi bi-box-arrow-right mx-1"></i
              >{{ t("checkout.closeRegister") }}
            </AsyncButton>
          </div>
        </div>
      </div>
    </div>
  </div>

  <!-- Close result modal -->
  <div v-if="closeResult" class="modal-backdrop show"></div>
  <div v-if="closeResult" class="modal d-block" tabindex="-1">
    <div class="modal-dialog modal-dialog-centered">
      <div class="modal-content">
        <div class="modal-header">
          <h5 class="modal-title">
            <i class="bi bi-check-circle mx-2"></i
            >{{ t("checkout.registerClosedTitle") }}
          </h5>
          <button
            type="button"
            class="btn-close"
            @click="closeResult = null"
          ></button>
        </div>
        <div class="modal-body">
          <p class="small mb-2">
            {{
              t(
                "checkout.salesRecorded",
                {
                  count: closeResult.salesCount,
                  total: fmt(closeResult.salesTotal),
                },
                closeResult.salesCount,
              )
            }}
          </p>
          <table class="table table-sm mb-0">
            <tbody>
              <tr>
                <td>{{ t("checkout.expectedCash") }}</td>
                <td class="text-end fw-semibold">
                  {{ fmt(closeResult.expectedCash ?? 0) }}
                </td>
              </tr>
              <tr>
                <td>{{ t("checkout.countedCash") }}</td>
                <td class="text-end">
                  {{ fmt(closeResult.closingCash ?? 0) }}
                </td>
              </tr>
              <tr>
                <td>{{ t("checkout.variance") }}</td>
                <td
                  class="text-end fw-semibold"
                  :class="
                    (closeResult.variance ?? 0) < -0.005
                      ? 'text-danger'
                      : (closeResult.variance ?? 0) > 0.005
                        ? 'text-warning'
                        : 'text-success'
                  "
                >
                  {{ fmt(closeResult.variance ?? 0) }}
                </td>
              </tr>
            </tbody>
          </table>
          <div class="d-flex justify-content-center gap-2 mt-3 pt-3 border-top">
            <button
              type="button"
              class="btn btn-primary"
              @click="closeResult = null"
            >
              {{ t("checkout.done") }}
            </button>
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
          <h5 class="modal-title">
            {{ taxEditingId ? t("common.edit") : t("settings.addProfile") }}
          </h5>
          <button
            type="button"
            class="btn-close"
            @click="showTaxModal = false"
          ></button>
        </div>
        <div class="modal-body">
          <div class="mb-3">
            <label class="form-label" for="tax-name">{{
              t("settings.profileName")
            }}</label>
            <input
              id="tax-name"
              v-model="taxForm.name"
              class="form-control"
              :class="{ 'is-invalid': !!taxErrors.name }"
              type="text"
              @input="clearFieldError(taxErrors, 'name')"
            />
            <div v-if="taxErrors.name" class="invalid-feedback">
              {{ taxErrors.name }}
            </div>
          </div>
          <div class="mb-3">
            <label class="form-label" for="tax-rate">{{
              t("settings.ratePercent")
            }}</label>
            <input
              id="tax-rate"
              v-model.number="taxForm.rate"
              class="form-control"
              :class="{ 'is-invalid': !!taxErrors.rate }"
              type="number"
              min="0"
              max="100"
              step="1"
              @input="clearFieldError(taxErrors, 'rate')"
            />
            <div v-if="taxErrors.rate" class="invalid-feedback">
              {{ taxErrors.rate }}
            </div>
          </div>
          <div class="form-check form-switch">
            <input
              id="tax-default"
              v-model="taxForm.isDefault"
              class="form-check-input"
              type="checkbox"
              role="switch"
            />
            <label class="form-check-label" for="tax-default">{{
              t("settings.isDefault")
            }}</label>
          </div>
          <div class="d-flex justify-content-end gap-2 mt-3 pt-3 border-top">
            <button
              type="button"
              class="btn btn-outline-secondary"
              :disabled="saving"
              @click="showTaxModal = false"
            >
              {{ t("common.cancel") }}
            </button>
            <AsyncButton
              variant="primary"
              :loading="saving"
              :disabled="!canSaveTax"
              @click="saveTaxProfile"
            >
              {{ t("common.save") }}
            </AsyncButton>
          </div>
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
          <h5 class="modal-title">
            {{
              currencyEditingId ? t("common.edit") : t("settings.addCurrency")
            }}
          </h5>
          <button
            type="button"
            class="btn-close"
            @click="showCurrencyModal = false"
          ></button>
        </div>
        <div class="modal-body">
          <div class="row g-3">
            <div class="col-6">
              <label class="form-label" for="cur-code">{{
                t("settings.currencyCode")
              }}</label>
              <input
                v-model="currencyForm.code"
                class="form-control text-uppercase"
                :class="{ 'is-invalid': !!curErrors.code }"
                type="text"
                maxlength="5"
                @input="clearFieldError(curErrors, 'code')"
              />
              <div v-if="curErrors.code" class="invalid-feedback">
                {{ curErrors.code }}
              </div>
            </div>
            <div class="col-6">
              <label class="form-label" for="cur-symbol">{{
                t("settings.currencySymbol")
              }}</label>
              <input
                id="cur-symbol"
                v-model="currencyForm.symbol"
                class="form-control"
                :class="{ 'is-invalid': !!curErrors.symbol }"
                type="text"
                maxlength="4"
                @input="clearFieldError(curErrors, 'symbol')"
              />
              <div v-if="curErrors.symbol" class="invalid-feedback">
                {{ curErrors.symbol }}
              </div>
            </div>
            <div class="col-12">
              <label class="form-label" for="cur-name">{{
                t("settings.currencyName")
              }}</label>
              <input
                id="cur-name"
                v-model="currencyForm.name"
                class="form-control"
                :class="{ 'is-invalid': !!curErrors.name }"
                type="text"
                @input="clearFieldError(curErrors, 'name')"
              />
              <div v-if="curErrors.name" class="invalid-feedback">
                {{ curErrors.name }}
              </div>
            </div>
            <div class="col-12">
              <label class="form-label" for="cur-rate">{{
                t("settings.exchangeRate")
              }}</label>
              <input
                id="cur-rate"
                v-model.number="currencyForm.rate"
                class="form-control"
                :class="{ 'is-invalid': !!curErrors.rate }"
                type="number"
                min="0.000001"
                step="1"
                @input="clearFieldError(curErrors, 'rate')"
              />
              <div v-if="curErrors.rate" class="invalid-feedback">
                {{ curErrors.rate }}
              </div>
            </div>
          </div>
          <div class="d-flex justify-content-end gap-2 mt-3 pt-3 border-top">
            <button
              type="button"
              class="btn btn-outline-secondary"
              :disabled="saving"
              @click="showCurrencyModal = false"
            >
              {{ t("common.cancel") }}
            </button>
            <AsyncButton
              variant="primary"
              :loading="saving"
              :disabled="!canSaveCurrency"
              @click="saveCurrency"
            >
              {{ t("common.save") }}
            </AsyncButton>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, reactive, ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import {
  open as openFileDialog,
  save as saveDialog,
} from "@tauri-apps/plugin-dialog";
import { useI18n } from "vue-i18n";
import AsyncButton from "../../components/AsyncButton.vue";
import EmptyState from "../../components/EmptyState.vue";
import {
  applyFieldRules,
  clearFieldError,
  useFormGuard,
} from "../../composables/useFormGuard";
import { useToast } from "../../composables/useToast";
import { useConfirm } from "../../composables/useConfirm";
import { closeDb, execute, insert, select, selectOne } from "../../lib/db";
import { baseCurrencySymbol, formatMoney, loadBaseCurrencySymbol } from "../../lib/currency";
import { useAuthStore } from "../../stores/auth";
import { useSettingsStore, PAGE_SIZE_OPTIONS } from "../../stores/settings";
import { useThemeStore } from "../../stores/theme";
import { setLocale, type Locale } from "../../i18n";
import type {
  BackupInfo,
  Currency,
  SaleSession,
  TaxProfile,
} from "../../types";
import emptyExpenses from "../../assets/empty/expenses.svg";
import emptyStock from "../../assets/empty/stock.svg";

const { t, locale } = useI18n();
const auth = useAuthStore();
const settings = useSettingsStore();
const theme = useThemeStore();
const toast = useToast();
const { confirmDialog } = useConfirm();

type TabId =
  | "register"
  | "store"
  | "tax"
  | "currency"
  | "receipt"
  | "prefs"
  | "backup";
const allTabs: Array<{
  id: TabId;
  label: string;
  icon: string;
  manageOnly?: boolean;
}> = [
  { id: "register", label: "settings.tabRegister", icon: "bi-cash-stack" },
  {
    id: "store",
    label: "settings.tabStore",
    icon: "bi-shop",
    manageOnly: true,
  },
  { id: "tax", label: "settings.tabTax", icon: "bi-percent", manageOnly: true },
  {
    id: "currency",
    label: "settings.tabCurrency",
    icon: "bi-cash-coin",
    manageOnly: true,
  },
  {
    id: "receipt",
    label: "settings.tabReceipt",
    icon: "bi-receipt",
    manageOnly: true,
  },
  {
    id: "prefs",
    label: "settings.tabPrefs",
    icon: "bi-sliders",
    manageOnly: true,
  },
  {
    id: "backup",
    label: "settings.tabBackup",
    icon: "bi-shield-check",
    manageOnly: true,
  },
];
const canManage = computed(
  () => auth.role === "Admin" || auth.can("settings.manage"),
);
const tabs = computed(() =>
  allTabs.filter((t) => !t.manageOnly || canManage.value),
);
const tab = ref<TabId>("register");

function flash(msg: string) {
  toast.success(msg);
}
function fail(msg: string) {
  toast.error(msg);
}

/* ---------------- Store profile (F7.1) ---------------- */

const saving = ref(false);
const storeForm = reactive({ name: "", phone: "", address: "", taxId: "" });
const storeErrors = reactive<Record<string, string>>({});
const storeGuard = useFormGuard(storeForm);
const canSaveStore = computed(
  () => storeGuard.isDirty.value && !saving.value && !!storeForm.name.trim(),
);
const logoInput = ref<HTMLInputElement | null>(null);
const logoError = ref("");

onMounted(async () => {
  try {
    if (!settings.loaded) await settings.load();
  } catch (e: unknown) {
    fail(String(e));
    return;
  }
  if (canManage.value) {
    storeForm.name = settings.storeName;
    storeForm.phone = settings.storePhone;
    storeForm.address = settings.storeAddress;
    storeForm.taxId = settings.taxId;
    receiptForm.header = settings.receiptHeader;
    receiptForm.footer = settings.receiptFooter;
    receiptForm.logoPos = settings.receiptLogoPos;
    receiptForm.format = settings.receiptFormat;
    initBackupForm();
    storeGuard.capture();
    receiptGuard.capture();
    autoGuard.capture();
    void loadBackups();
    void loadTaxProfiles().catch((e: unknown) => fail(String(e)));
    void loadCurrencies().catch((e: unknown) => fail(String(e)));
  }
  void loadOpenSession();
});

async function saveStore() {
  const ok = applyFieldRules(storeErrors, [
    ["name", !!storeForm.name.trim(), t("settings.storeName")],
  ]);
  if (!ok) {
    toast.error(t("common.fixErrors"));
    return;
  }
  saving.value = true;
  try {
    await Promise.all([
      settings.setValue("store_name", storeForm.name.trim()),
      settings.setValue("store_phone", storeForm.phone.trim()),
      settings.setValue("store_address", storeForm.address.trim()),
      settings.setValue("tax_id", storeForm.taxId.trim()),
    ]);
    storeGuard.markSaved();
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
const taxErrors = reactive<Record<string, string>>({});
const taxGuard = useFormGuard(taxForm);
const canSaveTax = computed(() => taxGuard.isDirty.value && !saving.value);

async function loadTaxProfiles() {
  taxProfiles.value = await select<TaxProfile>(
    "SELECT id, name, rate, is_default FROM tax_profiles ORDER BY is_default DESC, name",
  );
}

function openTaxModal(p?: TaxProfile) {
  Object.keys(taxErrors).forEach((k) => delete taxErrors[k]);
  taxEditingId.value = p ? p.id : null;
  taxForm.name = p ? p.name : "";
  taxForm.rate = p ? p.rate : 0;
  taxForm.isDefault = p
    ? p.is_default === 1
    : taxProfiles.value.every((x) => x.is_default !== 1);
  taxGuard.capture();
  showTaxModal.value = true;
}

async function saveTaxProfile() {
  const ok = applyFieldRules(taxErrors, [
    ["name", !!taxForm.name.trim(), t("settings.profileName")],
    [
      "rate",
      Number.isFinite(taxForm.rate) && taxForm.rate >= 0 && taxForm.rate <= 100,
      t("settings.ratePercent"),
    ],
  ]);
  if (!ok) {
    toast.error(t("common.fixErrors"));
    return;
  }
  saving.value = true;
  try {
    let id = taxEditingId.value;
    if (id) {
      await execute(
        "UPDATE tax_profiles SET name = ?, rate = ?, is_default = ? WHERE id = ?",
        [taxForm.name.trim(), taxForm.rate, taxForm.isDefault ? 1 : 0, id],
      );
    } else {
      id = await insert(
        "INSERT INTO tax_profiles (name, rate, is_default) VALUES (?, ?, ?)",
        [taxForm.name.trim(), taxForm.rate, taxForm.isDefault ? 1 : 0],
      );
    }
    if (taxForm.isDefault && id !== null) {
      await execute(
        "UPDATE tax_profiles SET is_default = CASE WHEN id = ? THEN 1 ELSE 0 END",
        [id],
      );
    }
    await loadTaxProfiles();
    taxGuard.markSaved();
    showTaxModal.value = false;
    flash(t("settings.saved"));
  } catch {
    fail(t("settings.duplicateEntry"));
  } finally {
    saving.value = false;
  }
}

async function makeTaxDefault(id: number) {
  await execute(
    "UPDATE tax_profiles SET is_default = CASE WHEN id = ? THEN 1 ELSE 0 END",
    [id],
  );
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
const curErrors = reactive<Record<string, string>>({});
const currencyGuard = useFormGuard(currencyForm);
const canSaveCurrency = computed(
  () => currencyGuard.isDirty.value && !saving.value,
);

async function loadCurrencies() {
  currencies.value = await select<Currency>(
    "SELECT id, code, name, symbol, rate, is_base FROM currencies ORDER BY is_base DESC, code",
  );
}

function openCurrencyModal(c?: Currency) {
  Object.keys(curErrors).forEach((k) => delete curErrors[k]);
  currencyEditingId.value = c ? c.id : null;
  currencyForm.code = c ? c.code : "";
  currencyForm.name = c ? c.name : "";
  currencyForm.symbol = c ? c.symbol : "";
  currencyForm.rate = c ? c.rate : 1;
  currencyGuard.capture();
  showCurrencyModal.value = true;
}

async function saveCurrency() {
  const ok = applyFieldRules(curErrors, [
    ["code", !!currencyForm.code.trim(), t("settings.currencyCode")],
    ["name", !!currencyForm.name.trim(), t("settings.currencyName")],
    ["symbol", !!currencyForm.symbol.trim(), t("settings.currencySymbol")],
    [
      "rate",
      Number.isFinite(currencyForm.rate) && currencyForm.rate > 0,
      t("settings.exchangeRate"),
    ],
  ]);
  if (!ok) {
    toast.error(t("common.fixErrors"));
    return;
  }
  saving.value = true;
  try {
    const code = currencyForm.code.trim().toUpperCase();
    if (currencyEditingId.value) {
      await execute(
        "UPDATE currencies SET code = ?, name = ?, symbol = ?, rate = ? WHERE id = ?",
        [
          code,
          currencyForm.name.trim(),
          currencyForm.symbol.trim(),
          currencyForm.rate,
          currencyEditingId.value,
        ],
      );
    } else {
      await execute(
        "INSERT INTO currencies (code, name, symbol, rate, is_base) VALUES (?, ?, ?, ?, 0)",
        [
          code,
          currencyForm.name.trim(),
          currencyForm.symbol.trim(),
          currencyForm.rate,
        ],
      );
    }
    await loadCurrencies();
    await loadBaseCurrencySymbol();
    currencyGuard.markSaved();
    showCurrencyModal.value = false;
    flash(t("settings.saved"));
  } catch {
    fail(t("settings.duplicateEntry"));
  } finally {
    saving.value = false;
  }
}

async function setBaseCurrency(c: Currency) {
  await execute(
    "UPDATE currencies SET is_base = CASE WHEN id = ? THEN 1 ELSE 0 END",
    [c.id],
  );
  // Keep the display-currency setting in sync so fmt() uses the base code.
  await settings.setValue("currency", c.code);
  await loadBaseCurrencySymbol();
  await loadCurrencies();
  flash(t("settings.saved"));
}

async function deleteCurrency(c: Currency) {
  if (c.is_base === 1) {
    fail(t("settings.deleteBaseBlocked"));
    return;
  }
  await execute("DELETE FROM currencies WHERE id = ?", [c.id]);
  await loadBaseCurrencySymbol();
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
const receiptGuard = useFormGuard(receiptForm);
const canSaveReceipt = computed(
  () => receiptGuard.isDirty.value && !saving.value,
);

async function saveReceipt() {
  saving.value = true;
  try {
    await Promise.all([
      settings.setValue("receipt_header", receiptForm.header.trim()),
      settings.setValue("receipt_footer", receiptForm.footer.trim()),
      settings.setValue("receipt_logo_pos", receiptForm.logoPos),
      settings.setValue("receipt_format", receiptForm.format),
    ]);
    receiptGuard.markSaved();
    flash(t("settings.saved"));
  } catch (e: unknown) {
    fail(String(e));
  } finally {
    saving.value = false;
  }
}

/* ---------------- Open Register ---------------- */

const openSession = ref<SaleSession | null>(null);
const registerBusy = ref(false);
const showOpenModal = ref(false);
const openCashAmt = ref(0);
const showCloseModal = ref(false);
const closeCashAmt = ref(0);
const closeResult = ref<SaleSession | null>(null);

function fmt(n: number): string {
  return formatMoney(n);
}

function dateLabel(d: string): string {
  const date = new Date(d + (d.includes("T") ? "" : "Z"));
  if (isNaN(date.getTime())) return d;
  return date.toLocaleString(locale.value);
}

async function loadOpenSession() {
  try {
    openSession.value = await invoke<SaleSession | null>("get_open_session");
  } catch (e: unknown) {
    fail(String(e));
  }
}

function openOpenModal() {
  openCashAmt.value = 0;
  showOpenModal.value = true;
}

async function confirmOpen() {
  if (isNaN(openCashAmt.value) || openCashAmt.value < 0) {
    toast.error(t("checkout.invalidCash"));
    return;
  }
  registerBusy.value = true;
  try {
    openSession.value = await invoke<SaleSession>("open_session", {
      input: { openingCash: openCashAmt.value, userId: auth.user?.id ?? null },
    });
    showOpenModal.value = false;
    flash(t("checkout.registerOpened"));
  } catch (e: unknown) {
    fail(String(e));
  } finally {
    registerBusy.value = false;
  }
}

function openCloseModal() {
  closeCashAmt.value = 0;
  closeResult.value = null;
  showCloseModal.value = true;
}

async function confirmClose() {
  if (!openSession.value) return;
  if (isNaN(closeCashAmt.value) || closeCashAmt.value < 0) {
    toast.error(t("checkout.invalidCounted"));
    return;
  }
  registerBusy.value = true;
  try {
    closeResult.value = await invoke<SaleSession>("close_session", {
      input: {
        sessionId: openSession.value.id,
        closingCash: closeCashAmt.value,
        userId: auth.user?.id ?? null,
      },
    });
    openSession.value = null;
    showCloseModal.value = false;
  } catch (e: unknown) {
    fail(String(e));
  } finally {
    registerBusy.value = false;
  }
}

watch(
  () => tab.value,
  (t) => {
    if (t === "register") void loadOpenSession();
  },
);

/* ---------------- Preferences (F7.5) ---------------- */

function onLangChange(e: Event) {
  setLocale((e.target as HTMLSelectElement).value as Locale);
  flash(t("settings.saved"));
}

function onPageSizeChange(e: Event) {
  const value = Number((e.target as HTMLSelectElement).value);
  void settings
    .setValue("page_size", String(value))
    .then(() => flash(t("settings.saved")))
    .catch((err: unknown) => fail(String(err)));
}

function onSoundToggle(e: Event) {
  const on = (e.target as HTMLInputElement).checked;
  void settings
    .setValue("sound_enabled", on ? "1" : "0")
    .catch((err: unknown) => fail(String(err)));
}

/* ---------------- Backups & data safety (F8.1–F8.5) ---------------- */

const integrity = ref<{ ok: boolean; text: string } | null>(null);
const checkingIntegrity = ref(false);
const backups = ref<BackupInfo[]>([]);
const backingUp = ref(false);
const restoringFile = ref(false);
const autoForm = reactive({
  enabled: false,
  freq: "daily" as "daily" | "weekly",
  retention: 5,
});
const autoErrors = reactive<Record<string, string>>({});
const autoGuard = useFormGuard(autoForm);
const canSaveAuto = computed(() => autoGuard.isDirty.value && !saving.value);

const backupDirDisplay = computed(
  () => settings.values["backup_dir"] || t("settings.defaultFolder"),
);

function initBackupForm() {
  autoForm.enabled = settings.values["backup_auto"] === "1";
  autoForm.freq =
    settings.values["backup_freq"] === "weekly" ? "weekly" : "daily";
  const parsed = Number(settings.values["backup_retention"]);
  autoForm.retention = !isNaN(parsed) || parsed >= 1 ? parsed : 5;
}

async function loadBackups() {
  try {
    backups.value = await invoke<BackupInfo[]>("list_backups");
  } catch (e: unknown) {
    console.error(e);
  }
}

async function runIntegrityCheck() {
  checkingIntegrity.value = true;
  integrity.value = null;
  try {
    const res = (await invoke<string>("check_db_integrity")).trim();
    const ok = res.toLowerCase() === "ok";
    integrity.value = { ok, text: ok ? t("settings.integrityOk") : res };
  } catch (e: unknown) {
    integrity.value = { ok: false, text: String(e) };
  } finally {
    checkingIntegrity.value = false;
  }
}

async function chooseFolder() {
  const picked = await openFileDialog({ directory: true, multiple: false });
  if (typeof picked === "string" && picked) {
    await settings
      .setValue("backup_dir", picked)
      .catch((e: unknown) => fail(String(e)));
    flash(t("settings.saved"));
  }
}

async function backupNow() {
  backingUp.value = true;
  try {
    await invoke("create_backup", {
      dir: settings.values["backup_dir"] || null,
      kind: "manual",
    });
    flash(t("settings.backupDone"));
    await loadBackups();
  } catch (e: unknown) {
    fail(String(e));
  } finally {
    backingUp.value = false;
  }
}

async function saveAutoSettings() {
  const ok = applyFieldRules(autoErrors, [
    [
      "retention",
      Number.isInteger(autoForm.retention) &&
        autoForm.retention >= 1 &&
        autoForm.retention <= 100,
      t("settings.retentionLabel"),
    ],
  ]);
  if (!ok) {
    toast.error(t("common.fixErrors"));
    return;
  }
  saving.value = true;
  try {
    await Promise.all([
      settings.setValue("backup_auto", autoForm.enabled ? "1" : "0"),
      settings.setValue("backup_freq", autoForm.freq),
      settings.setValue(
        "backup_retention",
        String(Math.max(1, Math.floor(autoForm.retention || 5))),
      ),
    ]);
    autoGuard.markSaved();
    flash(t("settings.saved"));
  } catch (e: unknown) {
    fail(String(e));
  } finally {
    saving.value = false;
  }
}

/** SQLite UTC timestamps → local display. */
function fmtDateTime(raw: string): string {
  const normalized = raw.includes("T") ? raw : raw.replace(" ", "T");
  const date = new Date(
    normalized.endsWith("Z") ? normalized : normalized + "Z",
  );
  return isNaN(date.getTime()) ? raw : date.toLocaleString(locale.value);
}

function fmtSize(n: number): string {
  if (n > 1048576) return (n / 1048576).toFixed(1) + " MB";
  return Math.max(1, Math.round(n / 1024)) + " KB";
}

async function restore(path: string) {
  if (!(await confirmDialog({ message: t("settings.restoreConfirm") }))) return;
  try {
    // Release the webview's SQL connections so the file can be replaced.
    await closeDb();
    await invoke("restore_database", { sourcePath: path });
    flash(t("settings.restoreDone"));
    window.setTimeout(() => window.location.reload(), 1200);
  } catch (e: unknown) {
    fail(String(e));
  }
}

/** Restores from any database file picked on disk (e.g. a downloaded or
 *  copied backup) — independent of the in-app backup list. */
async function restoreFromFile() {
  const picked = await openFileDialog({
    multiple: false,
    directory: false,
    filters: [
      { name: "SQLite database", extensions: ["db", "sqlite", "db3"] },
      { name: "All files", extensions: ["*"] },
    ],
  });
  if (typeof picked !== "string" || !picked) return;
  if (!(await confirmDialog({ message: t("settings.restoreConfirm") }))) return;
  restoringFile.value = true;
  try {
    // Release the webview's SQL connections so the file can be replaced.
    await closeDb();
    await invoke("restore_database", { sourcePath: picked });
    flash(t("settings.restoreDone"));
    window.setTimeout(() => window.location.reload(), 1200);
  } catch (e: unknown) {
    fail(String(e));
    restoringFile.value = false;
  }
}

async function removeBackup(path: string) {
  if (!(await confirmDialog({ message: t("settings.deleteConfirm") }))) return;
  try {
    await invoke("delete_backup", { path });
    await loadBackups();
  } catch (e: unknown) {
    fail(String(e));
  }
}

async function exportWorkbook() {
  const path = await saveDialog({
    defaultPath: "store-export.xlsx",
    filters: [{ name: "Excel", extensions: ["xlsx"] }],
  });
  if (!path) return;
  saving.value = true;
  try {
    const sheets = await invoke<number>("export_full_workbook", { path });
    flash(t("settings.exportDone", { count: sheets }));
  } catch (e: unknown) {
    fail(String(e));
  } finally {
    saving.value = false;
  }
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
