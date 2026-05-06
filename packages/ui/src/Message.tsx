import * as React from "react";

import { cn } from "./lib/utils";

type MessageProps = React.HTMLAttributes<HTMLElement> & {
  as?: "article" | "div";
};

const Message = React.forwardRef<HTMLElement, MessageProps>(
  ({ as = "article", className, ...props }, ref) => {
    const Comp = as as React.ElementType;
    return (
      <Comp
        ref={ref}
        className={cn(
          "grid grid-cols-[132px_minmax(0,1fr)] gap-3.5 rounded-md border-l-4 bg-background px-3.5 py-3 max-sm:grid-cols-1",
          className,
        )}
        {...props}
      />
    );
  },
);
Message.displayName = "Message";

const MessageMeta = React.forwardRef<HTMLDivElement, React.HTMLAttributes<HTMLDivElement>>(
  ({ className, ...props }, ref) => (
    <div ref={ref} className={cn("grid content-start gap-1", className)} {...props} />
  ),
);
MessageMeta.displayName = "MessageMeta";

const MessageContent = React.forwardRef<HTMLDivElement, React.HTMLAttributes<HTMLDivElement>>(
  ({ className, ...props }, ref) => (
    <div ref={ref} className={cn("min-w-0 break-words text-sm leading-6", className)} {...props} />
  ),
);
MessageContent.displayName = "MessageContent";

const MessageActions = React.forwardRef<HTMLDivElement, React.HTMLAttributes<HTMLDivElement>>(
  ({ className, ...props }, ref) => (
    <div ref={ref} className={cn("mt-2 flex flex-wrap gap-2", className)} {...props} />
  ),
);
MessageActions.displayName = "MessageActions";

export { Message, MessageActions, MessageContent, MessageMeta };
