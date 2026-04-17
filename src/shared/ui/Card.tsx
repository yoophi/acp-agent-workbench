import * as React from "react";

import { cn } from "../lib/utils";

export const cardClassName =
  "rounded-lg border bg-card text-card-foreground shadow-card";

type CardProps = React.HTMLAttributes<HTMLElement> & {
  as?: "div" | "section" | "article";
};

const Card = React.forwardRef<HTMLElement, CardProps>(({ as = "div", className, ...props }, ref) => {
  const Comp = as as React.ElementType;
  return <Comp ref={ref} className={cn(cardClassName, className)} {...props} />;
});
Card.displayName = "Card";

const CardHeader = React.forwardRef<HTMLDivElement, React.HTMLAttributes<HTMLDivElement>>(
  ({ className, ...props }, ref) => (
    <div
      ref={ref}
      className={cn(
        "flex items-center justify-between gap-4 border-b px-6 py-4 max-sm:flex-col max-sm:items-stretch",
        className,
      )}
      {...props}
    />
  ),
);
CardHeader.displayName = "CardHeader";

const CardTitleBlock = React.forwardRef<HTMLDivElement, React.HTMLAttributes<HTMLDivElement>>(
  ({ className, ...props }, ref) => (
    <div ref={ref} className={cn("min-w-0", className)} {...props} />
  ),
);
CardTitleBlock.displayName = "CardTitleBlock";

const CardTitle = React.forwardRef<HTMLHeadingElement, React.HTMLAttributes<HTMLHeadingElement>>(
  ({ className, ...props }, ref) => (
    <h2 ref={ref} className={cn("text-lg font-semibold leading-none tracking-tight", className)} {...props} />
  ),
);
CardTitle.displayName = "CardTitle";

const CardDescription = React.forwardRef<HTMLParagraphElement, React.HTMLAttributes<HTMLParagraphElement>>(
  ({ className, ...props }, ref) => (
    <p ref={ref} className={cn("text-sm text-muted-foreground", className)} {...props} />
  ),
);
CardDescription.displayName = "CardDescription";

const CardContent = React.forwardRef<HTMLDivElement, React.HTMLAttributes<HTMLDivElement>>(
  ({ className, ...props }, ref) => (
    <div ref={ref} className={cn("p-6", className)} {...props} />
  ),
);
CardContent.displayName = "CardContent";

export { Card, CardContent, CardDescription, CardHeader, CardTitle, CardTitleBlock };
