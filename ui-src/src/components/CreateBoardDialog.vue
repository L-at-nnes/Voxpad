<template>
  <VxModal :value="open" :title="$t('create.title')" @input="close">
    <div class="vx-create">
      <div class="vx-create__preview" :style="previewStyle">
        <span v-if="!cover" class="vx-create__initials">{{ previewInitials }}</span>
        <button class="vx-create__pick" @click="pick">
          <VxIcon name="image" :size="16" /> {{ cover ? $t('create.changeCover') : $t('create.addCover') }}
        </button>
      </div>

      <label class="vx-create__label">{{ $t('create.name') }}</label>
      <input
        ref="nameInput"
        v-model="name"
        class="vx-input"
        type="text"
        :placeholder="$t('create.namePlaceholder')"
        maxlength="48"
        spellcheck="false"
        @keydown.enter="confirm"
      />
      <p class="vx-create__hint">{{ $t('create.coverHint') }}</p>
    </div>

    <template #footer>
      <button class="vx-btn" @click="close">{{ $t('common.cancel') }}</button>
      <button class="vx-btn vx-btn--accent" :disabled="!name.trim()" @click="confirm">
        {{ $t('create.create') }}
      </button>
    </template>
  </VxModal>
</template>

<script lang="ts">
import Vue from 'vue';
import { gradientFromName, fileUrl, initials } from '@/utils/visual';
import VxModal from '@/components/VxModal.vue';
import VxIcon from '@/components/VxIcon.vue';

export default Vue.extend({
  name: 'CreateBoardDialog',
  components: { VxModal, VxIcon },
  data: () => ({ name: '', cover: '' }),
  computed: {
    open(): boolean {
      return this.$store.getters.createBoardDialog;
    },
    previewInitials(): string {
      return initials(this.name || 'New');
    },
    previewStyle(): Record<string, string> {
      return this.cover
        ? { backgroundImage: `url("${fileUrl(this.cover)}")` }
        : { background: gradientFromName(this.name || 'new soundboard') };
    },
  },
  watch: {
    open(v: boolean) {
      if (v) {
        this.name = '';
        this.cover = '';
        this.$nextTick(() => (this.$refs.nameInput as HTMLInputElement | undefined)?.focus());
      }
    },
  },
  methods: {
    close() {
      this.$store.commit('setCreateBoardDialog', false);
    },
    async pick() {
      const cover = await this.$store.dispatch('pickCover');
      if (cover) this.cover = cover;
    },
    confirm() {
      if (!this.name.trim()) return;
      this.$store.dispatch('createBoard', { name: this.name.trim(), cover: this.cover });
      this.close();
    },
  },
});
</script>

<style scoped lang="scss">
.vx-create {
  display: flex;
  flex-direction: column;
  gap: 12px;

  &__preview {
    position: relative;
    width: 100%;
    aspect-ratio: 16 / 9;
    border-radius: var(--radius);
    background-size: cover;
    background-position: center;
    display: grid;
    place-items: center;
    margin-bottom: 6px;
  }
  &__initials {
    font-size: 40px;
    font-weight: 800;
    color: oklch(0.99 0.01 320);
    text-shadow: 0 2px 8px oklch(0 0 0 / 0.4);
  }
  &__pick {
    position: absolute;
    bottom: 10px;
    right: 10px;
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 7px 12px;
    border: none;
    border-radius: 99px;
    background: oklch(0 0 0 / 0.5);
    backdrop-filter: blur(6px);
    color: var(--text);
    font-size: 12px;
    font-weight: 500;
  }
  &__label {
    font-size: 13px;
    font-weight: 600;
    color: var(--text-dim);
  }
  &__hint {
    margin: 0;
    font-size: 12px;
    color: var(--text-dim);
  }
}
</style>
