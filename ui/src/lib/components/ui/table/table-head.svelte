<script>
  import { cn } from '$lib/utils.js'
  let { class: className, children, ...rest } = $props()
</script>

<th class={cn('h-10 px-2 text-left align-middle font-medium text-muted-foreground', className)} {...rest}>
  {@render children?.()}
</th>
