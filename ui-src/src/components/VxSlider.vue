<template>
  <div class="vx-slider">
    <button class="vx-slider__step" :disabled="value <= min" @click="nudge(-1)">
      <VxIcon name="minus" :size="15" />
    </button>
    <input
      type="range"
      :min="min"
      :max="max"
      :step="step"
      :value="value"
      :style="trackStyle"
      @input="onInput"
    />
    <button class="vx-slider__step" :disabled="value >= max" @click="nudge(1)">
      <VxIcon name="plus" :size="15" />
    </button>
    <span v-if="showValue" class="vx-slider__value">{{ value }}</span>
  </div>
</template>

<script lang="ts">
import Vue from 'vue';
import VxIcon from '@/components/VxIcon.vue';

export default Vue.extend({
  name: 'VxSlider',
  components: { VxIcon },
  props: {
    value: { type: Number, required: true },
    min: { type: Number, default: 0 },
    max: { type: Number, default: 100 },
    step: { type: Number, default: 1 },
    showValue: { type: Boolean, default: true },
  },
  computed: {
    pct(): number {
      return ((this.value - this.min) / (this.max - this.min)) * 100;
    },
    trackStyle(): Record<string, string> {
      return {
        background: `linear-gradient(to right, var(--accent) ${this.pct}%, var(--surface-3) ${this.pct}%)`,
      };
    },
  },
  methods: {
    onInput(e: Event) {
      this.$emit('input', Number((e.target as HTMLInputElement).value));
    },
    nudge(dir: number) {
      const next = Math.min(this.max, Math.max(this.min, this.value + dir));
      if (next !== this.value) this.$emit('input', next);
    },
  },
});
</script>

<style scoped lang="scss">
.vx-slider {
  display: flex;
  align-items: center;
  gap: 8px;
  width: 100%;

  &__step {
    flex: none;
    width: 26px;
    height: 26px;
    display: grid;
    place-items: center;
    border-radius: 7px;
    border: 1px solid var(--border);
    background: var(--surface-2);
    color: var(--text-dim);
    transition: background 0.14s var(--ease), color 0.14s var(--ease), border-color 0.14s var(--ease);

    &:hover:not(:disabled) {
      background: var(--surface-3);
      color: var(--text);
      border-color: var(--accent);
    }
    &:disabled {
      opacity: 0.35;
      cursor: default;
    }
  }

  input {
    flex: 1;
    min-width: 0;
    -webkit-appearance: none;
    appearance: none;
    height: 6px;
    border-radius: 99px;
    outline: none;
    cursor: pointer;
  }
  input::-webkit-slider-thumb {
    -webkit-appearance: none;
    appearance: none;
    width: 16px;
    height: 16px;
    border-radius: 50%;
    background: var(--text);
    border: 3px solid var(--accent);
    box-shadow: 0 2px 6px oklch(0 0 0 / 0.4);
    transition: transform 0.12s var(--ease);
  }
  input::-webkit-slider-thumb:hover {
    transform: scale(1.15);
  }

  &__value {
    flex: none;
    min-width: 30px;
    text-align: right;
    font-size: 13px;
    font-weight: 600;
    font-variant-numeric: tabular-nums;
    color: var(--text-dim);
  }
}
</style>
