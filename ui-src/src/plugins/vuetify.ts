import Vue from 'vue';
import Vuetify from 'vuetify/lib/framework';

Vue.use(Vuetify);

// Voxpad only uses Vuetify for a few primitives (dialogs). The real surface is custom-styled.
export default new Vuetify({
  theme: {
    dark: true,
    options: { customProperties: true },
    themes: {
      dark: {
        primary: '#d24bb8',
        background: '#1a1626',
      },
    },
  },
});
