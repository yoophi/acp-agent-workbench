import * as React from "react";
import { Slot } from "@radix-ui/react-slot";
import { cva, type VariantProps } from "class-variance-authority";

import { cn } from "../lib/utils";

const buttonVariants = cva(
  "button inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-md text-sm font-semibold transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[#2f8062] focus-visible:ring-offset-2 disabled:pointer-events-none disabled:opacity-60 [&_svg]:pointer-events-none [&_svg]:size-4 [&_svg]:shrink-0",
  {
    variants: {
      variant: {
        primary: "button-primary border border-transparent bg-[#2f8062] text-white shadow-sm hover:bg-[#276f55]",
        default: "button-primary border border-transparent bg-[#2f8062] text-white shadow-sm hover:bg-[#276f55]",
        secondary: "button-secondary border border-[#d7d3c8] bg-white text-[#20231f] shadow-sm hover:bg-[#f7f6f0]",
        ghost: "button-ghost border border-[#d7e5d7] bg-[#f3f7f2] text-[#2e594b] hover:bg-[#eaf3e8]",
        destructive: "border border-transparent bg-[#b44747] text-white shadow-sm hover:bg-[#9d3939]",
        outline: "border border-[#d7d3c8] bg-transparent text-[#20231f] hover:bg-white",
        link: "h-auto border-transparent bg-transparent p-0 text-[#2e594b] underline-offset-4 hover:underline",
      },
      size: {
        default: "min-h-[38px] px-[13px] py-2",
        sm: "min-h-8 px-3 py-1.5 text-xs",
        lg: "min-h-10 px-5 py-2",
        icon: "h-9 w-9 p-0",
      },
    },
    defaultVariants: {
      variant: "secondary",
      size: "default",
    },
  },
);

export interface ButtonProps
  extends React.ButtonHTMLAttributes<HTMLButtonElement>,
    VariantProps<typeof buttonVariants> {
  asChild?: boolean;
  icon?: React.ReactNode;
}

const Button = React.forwardRef<HTMLButtonElement, ButtonProps>(
  ({ className, variant, size, asChild = false, icon, children, ...props }, ref) => {
    const Comp = asChild ? Slot : "button";
    return (
      <Comp className={cn(buttonVariants({ variant, size, className }))} ref={ref} {...props}>
        {icon}
        {children ? <span>{children}</span> : null}
      </Comp>
    );
  },
);
Button.displayName = "Button";

export { Button, buttonVariants };
