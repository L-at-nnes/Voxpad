<template>
  <transition name="vx-modal">
    <div v-if="value" class="vx-modal" @mousedown.self="$emit('input', false)">
      <div class="vx-modal__panel" :style="{ maxWidth: width + 'px' }">
        <header class="vx-modal__head">
          <h2>{{ title }}</h2>
          <button class="vx-icon-btn" :aria-label="$t('common.close')" @click="$emit('input', false)">
            <VxIcon name="close" :size="18" />
          </button>
        </header>
        <div class="vx-modal__body">
          <slot />
        </div>
        <footer v-if="$slots.footer" class="vx-modal__foot">
          <slot name="footer" />
        </footer>
      </div>
    </div>
  </transition>
</template>

<script lang="ts">
import Vue from 'vue';
import VxIcon from '@/components/VxIcon.vue';

export default Vue.extend({
  name: 'VxModal',
  components: { VxIcon },
  props: {
    value: { type: Boolean, default: false },
    title: { type: String, default: '' },
    width: { type: Number, default: 460 },
  },
  watch: {
    // A stable handler registered once, not recreated on every open/close: the old code built a
    // *new* closure each time this watcher fired, added it on open, then tried to remove it on
    // close with a different (never-added) closure — the removeEventListener call was a silent
    // no-op, so every open/close cycle leaked one more permanent 'keydown' listener on window.
    value(open: boolean) {
      if (open) window.addEventListener('keydown', this.handleEscape);
      else window.removeEventListener('keydown', this.handleEscape);
    },
  },
  beforeDestroy() {
    window.removeEventListener('keydown', this.handleEscape);
  },
  methods: {
    handleEscape(e: KeyboardEvent) {
      if (e.key === 'Escape') this.$emit('input', false);
    },
  },
});
</script>

<style scoped lang="scss">
.vx-modal {
  position: fixed;
  inset: 0;
  z-index: 200;
  display: grid;
  place-items: center;
  padding: 24px;
  background: oklch(0 0 0 / 0.55);
  backdrop-filter: blur(3px);

  &__panel {
    width: 100%;
    max-height: calc(100vh - 60px);
    display: flex;
    flex-direction: column;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    box-shadow: 0 24px 60px oklch(0 0 0 / 0.5);
    overflow: hidden;
  }

  &__head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 18px 20px;
    border-bottom: 1px solid var(--border);
    h2 {
      margin: 0;
      font-size: 17px;
      font-weight: 700;
    }
  }

  &__body {
    padding: 20px;
    overflow-y: auto;
  }

  &__foot {
    display: flex;
    justify-content: flex-end;
    gap: 10px;
    padding: 16px 20px;
    border-top: 1px solid var(--border);
  }
}

.vx-modal-enter-active,
.vx-modal-leave-active {
  transition: opacity 0.16s var(--ease);
}
.vx-modal-enter,
.vx-modal-leave-to {
  opacity: 0;
}
.vx-modal-enter .vx-modal__panel,
.vx-modal-leave-to .vx-modal__panel {
  transform: scale(0.97);
}
.vx-modal__panel {
  transition: transform 0.18s var(--ease);
}
</style>
