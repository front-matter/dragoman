<script>
  import { cn } from '$lib/utils.js'
  let { class: className, children, value = $bindable(), ...rest } = $props()
</script>

<select
  class={cn(
    'flex h-9 w-full rounded-md border border-input bg-background px-3 py-1 text-sm shadow-sm',
    'focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:border-primary',
    'disabled:cursor-not-allowed disabled:opacity-50',
    'transition-colors',
    className,
  )}
  bind:value
  {...rest}
>
  {@render children?.()}
</select>
