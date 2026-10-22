<template>
  <div class="vx-zoom">
    <VxIcon name="zoom" :size="16" />
    <button
      v-for="n in levels"
      :key="n"
      class="vx-zoom__dot"
      :class="{ 'vx-zoom__dot--active': n - 1 === effectiveZoom }"
      :title="`${$t('app.zoom')} ${n}`"
      @click="set(n - 1)"
    ></button>
    <!-- Reset indicator: shown when this tab has a zoom override -->
    <button
      v-if="tabHasZoomOverride"
      class="vx-zoom__reset"
      :title="$t('app.resetZoom')"
      @click.stop="resetTabZoom"
    >
      <VxIcon name="close" :size="11" />
    </button>
  </div>
</template>

<script lang="ts">
import Vue from 'vue';
import VxIcon from '@/components/VxIcon.vue';
import { Tab } from '@/types';

export default Vue.extend({
  name: 'ZoomControl',
  components: { VxIcon },
  data: () => ({ levels: 6 }),
  computed: {
    effectiveZoom(): number {
      return this.$store.getters.effectiveZoom;
    },
    currentTab(): Tab | undefined {
      return this.$store.getters.currentTab;
    },
    tabHasZoomOverride(): boolean {
      return !!this.currentTab && this.currentTab.zoom != null;
    },
  },
  methods: {
    set(z: number) {
      // Always sets per-tab zoom (the global default lives in settings/inspector)
      this.$store.dispatch('setTabZoom', z);
    },
    resetTabZoom() {
      this.$store.dispatch('setTabZoom', null);
    },
  },
});
</script>

<style scoped lang="scss">
.vx-zoom {
  display: flex;
  align-items: center;
  gap: 7px;
  padding: 0 12px;
  height: 38px;
  border-radius: 99px;
  background: var(--surface);
  border: 1px solid var(--border);
  color: var(--text-dim);

  &__dot {
    width: 9px;
    height: 9px;
    border-radius: 50%;
    border: none;
    background: var(--surface-3);
    transition: background 0.14s var(--ease), transform 0.14s var(--ease);

    &:hover {
      background: var(--text-dim);
    }
    &--active {
      background: var(--accent);
      transform: scale(1.15);
    }
  }

  &__reset {
    display: grid;
    place-items: center;
    width: 18px;
    height: 18px;
    border-radius: 50%;
    border: none;
    background: var(--surface-3);
    color: var(--text-dim);
    cursor: pointer;
    transition: background 0.12s var(--ease), color 0.12s var(--ease);
    margin-left: -2px;

    &:hover {
      background: var(--accent-soft);
      color: var(--accent);
    }
  }
}
</style>
