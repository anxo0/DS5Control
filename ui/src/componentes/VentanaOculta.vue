<script setup lang="ts">
import { t } from '../i18n'
</script>

<template>
  <div class="oculta" role="status">
    <p>{{ t('general.oculta.texto') }}</p>
  </div>
</template>

<style scoped>
/* Solo se ve en el navegador: en el host, la ventana se oculta de verdad. */
.oculta {
  position: fixed;
  inset: 0;
  display: grid;
  place-items: center;
  background: var(--ground);
  color: var(--muted);
}
</style>
