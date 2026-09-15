<script setup lang="ts">
import { computed } from "vue";
import { useI18n } from "vue-i18n";

const { t } = useI18n();
const visible = defineModel<boolean>({ required: true });
const props = defineProps<{
  results: Array<{ status?: string; fileName?: string; error?: string }>;
  fileNames: string[];
  logCount: number;
}>();
const emit = defineEmits<{
  openOutput: [];
  exportLogs: [];
}>();

const successCount = computed(
  () => props.results.filter((r) => r && r.status === "success").length,
);
const failCount = computed(() => props.results.length - successCount.value);
const failures = computed(() =>
  props.results.filter((r) => r && r.status !== "success"),
);
</script>

<template>
  <transition name="dialog-pop">
    <div v-if="visible" class="dialog-overlay" @click="visible = false">
      <div class="dialog-content" @click.stop>
        <div class="dialog-header">
          <h3>
            {{
              results && results.length > 0
                ? t("conversion.resultSuccess")
                : t("conversion.resultFailed")
            }}
          </h3>
          <button class="dialog-close" @click="visible = false">×</button>
        </div>
        <div class="dialog-body">
          <div v-if="results && results.length > 0">
            <div class="result-summary">
              <div class="result-item">
                <span class="result-label">{{ t("conversion.totalLabel") }}</span>
                <span class="result-value">{{ results.length }} 个</span>
              </div>
              <div class="result-item">
                <span class="result-label">{{ t("conversion.successLabel") }}</span>
                <span class="result-value success">{{ successCount }} 个</span>
              </div>
              <div class="result-item">
                <span class="result-label">{{ t("conversion.failLabel") }}</span>
                <span class="result-value error">{{ failCount }} 个</span>
              </div>
            </div>
            <div v-if="failures.length > 0" class="error-details">
              <h4>{{ t("conversion.errorDetails") }}</h4>
              <ul class="error-list">
                <li v-for="(result, index) in failures" :key="index">
                  {{
                    result.fileName ||
                    fileNames[results.indexOf(result)] ||
                    t("conversion.errorFile")
                  }}: {{ result.error || t("common.unknownError") }}
                </li>
              </ul>
            </div>
          </div>
          <div v-else>
            <p>{{ t("conversion.severeError") }}</p>
          </div>
        </div>
        <div class="dialog-footer">
          <button
            class="btn-text"
            @click="emit('openOutput')"
            :disabled="successCount === 0"
          >
            {{ t("conversion.openOutputDir") }}
          </button>
          <button
            class="btn-text secondary"
            @click="emit('exportLogs')"
            :disabled="logCount === 0"
          >
            {{ t("conversion.exportLog") }}
          </button>
          <button class="btn-text secondary" @click="visible = false">
            {{ t("common.close") }}
          </button>
        </div>
      </div>
    </div>
  </transition>
</template>

<style scoped>
.result-summary {
  display: flex;
  flex-direction: column;
  gap: 8px;
  margin-bottom: 12px;
}

.result-item {
  display: flex;
  justify-content: space-between;
  gap: 12px;
  padding: 10px 12px;
  border-radius: 10px;
  background: rgba(0, 0, 0, 0.03);
  text-align: left;
}

.result-label {
  font-size: 13px;
  color: #64748b;
  font-weight: 600;
}

.result-value {
  font-size: 13px;
  font-weight: 700;
  color: #1d1d1f;
  font-variant-numeric: tabular-nums;
}

.result-value.success {
  color: #059669;
}

.result-value.error {
  color: #dc2626;
}

.error-details {
  text-align: left;
  margin-top: 8px;
}

.error-details h4 {
  margin: 0 0 8px;
  font-size: 13px;
  color: #64748b;
}

.error-list {
  margin: 0;
  padding-left: 18px;
  font-size: 12.5px;
  color: #6b7280;
  line-height: 1.55;
}
</style>
