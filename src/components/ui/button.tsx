import * as React from "react"
import { cva, type VariantProps } from "class-variance-authority"
import { cn } from "cn"
import { Slot } from "radix-ui"

const buttonVariants = cva(
  "cut glitch-hover [--cut:6px] inline-flex shrink-0 items-center justify-center gap-2 font-display text-sm font-semibold tracking-wide whitespace-nowrap uppercase transition-colors outline-none focus-visible:[--cut-border:var(--ring)] disabled:pointer-events-none disabled:opacity-50 aria-invalid:[--cut-border:var(--destructive)] [&_svg]:pointer-events-none [&_svg]:shrink-0 [&_svg:not([class*='size-'])]:size-4",
  {
    variants: {
      variant: {
        default:
          "text-primary-foreground [--cut-border:var(--primary)] [--cut-fill:var(--primary)] hover:[--cut-fill:color-mix(in_oklab,var(--primary)_85%,white)]",
        destructive:
          "text-white [--cut-border:var(--destructive)] [--cut-fill:var(--destructive)] hover:[--cut-fill:color-mix(in_oklab,var(--destructive)_85%,white)]",
        outline:
          "[--cut-fill:var(--background)] hover:[--cut-border:var(--muted-foreground)] hover:[--cut-fill:var(--accent)]",
        secondary:
          "text-secondary-foreground [--cut-fill:var(--secondary)] hover:[--cut-border:var(--muted-foreground)]",
        ghost:
          "[--cut-border:transparent] [--cut-fill:transparent] hover:[--cut-fill:var(--accent)]",
        link: "text-primary normal-case underline-offset-4 [--cut-border:transparent] [--cut-fill:transparent] hover:underline",
      },
      size: {
        default: "h-9 px-4 py-2 has-[>svg]:px-3",
        xs: "h-6 gap-1 px-2 text-xs has-[>svg]:px-1.5 [&_svg:not([class*='size-'])]:size-3",
        sm: "h-8 gap-1.5 px-3 has-[>svg]:px-2.5",
        lg: "h-10 px-6 has-[>svg]:px-4",
        icon: "size-9",
        "icon-xs": "size-6 [&_svg:not([class*='size-'])]:size-3",
        "icon-sm": "size-8",
        "icon-lg": "size-10",
      },
    },
    defaultVariants: {
      variant: "default",
      size: "default",
    },
  }
)

function Button({
  className,
  variant = "default",
  size = "default",
  asChild = false,
  ...props
}: React.ComponentProps<"button"> &
  VariantProps<typeof buttonVariants> & {
    asChild?: boolean
  }) {
  const Comp = asChild ? Slot.Root : "button"

  return (
    <Comp
      data-slot="button"
      data-variant={variant}
      data-size={size}
      className={cn(buttonVariants({ variant, size, className }))}
      {...props}
    />
  )
}

export { Button, buttonVariants }
