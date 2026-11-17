<template>
  <VxModal :value="open" :title="$t('merge.title')" @input="close">
    <div v-if="board" class="vx-merge">
      <p class="vx-merge__hint">{{ $t('merge.hint', { name: board.name, count: board.sounds.length }) }}</p>

      <label class="vx-merge__label">{{ $t('merge.into') }}</label>
      <div class="vx-merge__list">
        <button
          v-for="t in targets"
          :key="t.id"
          class="vx-merge__target"
          :class="{ 'vx-merge__target--on': dest === t.id }"
          @click="dest = t.id"
        >
          <span class="vx-merge__name">{{ t.name }}</span>
          <span class="vx-merge__count">{{ t.sounds.length }}</span>
        </button>
      </div>

      <label class="vx-merge__label">{{ $t('merge.tag') }}</label>
      <input
        ref="tagInput"
        v-model="tag"
        class="vx-input"
        type="text"
        :placeholder="$t('merge.tagPlaceholder')"
        maxlength="32"
        spellcheck="false"
        @keydown.enter="confirm"
      />
    </div>

    <template #footer>
      <button class="vx-btn" @click="close">{{ $t('common.cancel') }}</button>
      <button class="vx-btn vx-btn--accent" :disabled="!canConfirm" @click="confirm">
        {{ $t('merge.confirm') }}
      </button>
    </template>
  </VxModal>
</template>

<script lang="ts">
import Vue from 'vue';
import { Tab } from '@/types';
import VxModal from '@/components/VxModal.vue';

const MAX_TAG_LEN = 32;

export default Vue.extend({
  name: 'MergeBoardDialog',
  components: { VxModal },
  data: () => ({ dest: null as number | null, tag: '' }),
  computed: {
    board(): Tab | null {
      return this.$store.getters.mergeBoard;
    },
    open(): boolean {
      return !!this.board;
    },
    targets(): Tab[] {
      return this.$store.getters.tabs.filter((t: Tab) => !this.board || t.id !== this.board.id);
    },
    canConfirm(): boolean {
      return this.dest !== null && !!this.tag.trim();
    },
  },
  watch: {
    // (re)initialise whenever the dialog opens for a board
    board(b: Tab | null, prev: Tab | null) {
      if (!b || (prev && prev.id === b.id)) return;
      this.dest = null;
      this.tag = b.name.slice(0, MAX_TAG_LEN);
    },
  },
  methods: {
    close() {
      this.$store.commit('setMergeBoard', null);
    },
    confirm() {
      if (!this.board || !this.canConfirm || this.dest === null) return;
      const payload = { id: this.board.id, dest: this.dest, tag: this.tag.trim() };
      this.close();
      this.$store.dispatch('boardToTag', payload);
    },
  },
});
</script>

<style scoped lang="scss">
.vx-merge {
  display: flex;
  flex-direction: column;
  gap: 8px;

  &__hint {
    margin: 0 0 6px;
    color: var(--text-dim);
    font-size: 13px;
    line-height: 1.45;
  }

  &__label {
    margin-top: 6px;
    font-size: 12px;
    font-weight: 600;
    color: var(--text-dim);
  }

  &__list {
    display: flex;
    flex-direction: column;
    gap: 4px;
    max-height: 220px;
    overflow-y: auto;
  }

  &__target {
    display: flex;
    align-items: center;
    gap: 12px;
    width: 100%;
    padding: 10px 14px;
    border: 1px solid transparent;
    border-radius: var(--radius-sm);
    background: none;
    color: var(--text-dim);
    font-size: 14px;
    font-weight: 500;
    text-align: left;
    cursor: pointer;
    transition: background 0.12s var(--ease), color 0.12s var(--ease);

    &:hover {
      background: var(--surface-2);
      color: var(--text);
    }
    &--on {
      color: var(--accent);
      border-color: var(--accent-soft);
      background: var(--accent-soft);
    }
  }

  &__name {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  &__count {
    flex: none;
    font-size: 13px;
    font-variant-numeric: tabular-nums;
    opacity: 0.75;
  }
}
</style>
