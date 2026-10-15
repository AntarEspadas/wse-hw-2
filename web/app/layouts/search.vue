<template>
  <div>

    <v-form @submit.prevent="onSubmit">
      <v-text-field v-model="query" label="Search" />
    </v-form>

    <slot />
  </div>
</template>

<script lang="ts" setup>
  const router = useRouter()

  const query = ref("")

  function onSubmit() {
    if (query.value.trim() === "") {
      return
    }
    router.push({path: "/search", query: {t: "con", q: query.value}})
  }
</script>