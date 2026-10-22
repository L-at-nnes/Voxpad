<template>
  <div class="vx-viewmode">
    <button
      v-for="opt in options"
      :key="opt.mode"
      class="vx-viewmode__btn"
      :class="{ 'vx-viewmode__btn--active': effectiveViewMode === opt.mode }"
      :title="$t(opt.label)"
      @click="set(opt.mode)"
    >
      <VxIcon :name="opt.icon" :size="16" />
    </button>
  </div>
</template>

<script lang="ts">
import Vue from 'vue';
import VxIcon from '@/components/VxIcon.vue';
import { BoardViewMode } from '@/types';

export default Vue.extend({
  name: 'ViewModeControl',
  components: { VxIcon },
  data: () => ({
    options: [
      { mode: BoardViewMode.Cards, icon: 'grid-view', label: 'app.viewCards' },
      { mode: BoardViewMode.List, icon: 'list-view', label: 'app.viewList' },
      { mode: BoardViewMode.Album, icon: 'disc', label: 'app.viewAlbum' },
    ],
  }),
  computed: {
    effectiveViewMode(): BoardViewMode {
      return this.$store.getters.effectiveViewMode;
    },
  },
  methods: {
    set(mode: BoardViewMode) {
      this.$store.dispatch('setTabViewMode', mode);
    },
  },
});
</script>

<style scoped lang="scss">
.vx-viewmode {
  display: flex;
  align-items: center;
  gap: 2px;
  padding: 3px;
  height: 38px;
  border-radius: 99px;
  background: var(--surface);
  border: 1px solid var(--border);

  &__btn {
    display: grid;
    place-items: center;
    width: 30px;
    height: 30px;
    border-radius: 99px;
    border: none;
    background: transparent;
    color: var(--text-dim);
    cursor: pointer;
    transition: background 0.14s var(--ease), color 0.14s var(--ease);

    &:hover {
      color: var(--text);
    }
    &--active {
      background: var(--accent-soft);
      color: var(--accent);
    }
  }
}
</style>
