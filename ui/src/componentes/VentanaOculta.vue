<template>
  <div class="oculta" role="status">
    <p>App oculta. Pulsa el botón de silencio del mando para volver.</p>
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
